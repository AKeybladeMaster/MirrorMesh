use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::{MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS, PeerFingerprint, PeerPublicKey};
use mm_ports::{LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError};
use zeroize::Zeroizing;

const ED25519_SEED_BYTES: usize = 32;
const PAIRING_PROOF_DOMAIN: &[u8] = b"MirrorMesh.PairingProof.v1\0";
const TLS13_SERVER_CERT_VERIFY_CONTEXT: &[u8; 34] = b"TLS 1.3, server CertificateVerify\0";

pub(super) trait CredentialStore: Send + Sync {
    fn read_seed(&self) -> Result<Option<Zeroizing<Vec<u8>>>, PortError>;
    fn write_seed(&self, seed: &[u8; ED25519_SEED_BYTES]) -> Result<(), PortError>;
    fn delete_seed(&self) -> Result<(), PortError>;
}

pub(super) struct IdentityCore<S: CredentialStore> {
    store: S,
}

impl<S: CredentialStore> IdentityCore<S> {
    pub(super) fn open_or_create(store: S) -> Result<Self, PortError> {
        match store.read_seed()? {
            Some(seed) => {
                Self::fixed_seed(&seed)?;
            }
            None => {
                let mut seed = Zeroizing::new([0u8; ED25519_SEED_BYTES]);
                getrandom::fill(&mut *seed).map_err(|_| PortError::Unavailable)?;
                store.write_seed(&seed)?;
            }
        }
        Ok(Self { store })
    }

    fn fixed_seed(stored_seed: &[u8]) -> Result<Zeroizing<[u8; ED25519_SEED_BYTES]>, PortError> {
        if stored_seed.len() != ED25519_SEED_BYTES {
            return Err(PortError::StorageFailure);
        }
        let mut seed = Zeroizing::new([0u8; ED25519_SEED_BYTES]);
        seed.copy_from_slice(stored_seed);
        Ok(seed)
    }

    fn read_signing_key(&self) -> Result<SigningKey, PortError> {
        let stored_seed = self.store.read_seed()?.ok_or(PortError::Unavailable)?;
        let seed = Self::fixed_seed(&stored_seed)?;
        Ok(SigningKey::from_bytes(&seed))
    }

    fn public_key_from(signing_key: &SigningKey) -> Result<PeerPublicKey, PortError> {
        PeerPublicKey::new(signing_key.verifying_key().to_bytes().to_vec())
            .map_err(|_| PortError::IdentityRejected)
    }
}

impl<S: CredentialStore> LocalPairingIdentityPort for IdentityCore<S> {
    fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
        Self::public_key_from(&self.read_signing_key()?)
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        if !is_canonical_pairing_message(message) {
            return Err(PortError::InvalidConfiguration);
        }
        Ok(self.read_signing_key()?.sign(message).to_bytes().to_vec())
    }

    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
        let mut seed = Zeroizing::new([0u8; ED25519_SEED_BYTES]);
        getrandom::fill(&mut *seed).map_err(|_| PortError::Unavailable)?;
        let signing_key = SigningKey::from_bytes(&seed);
        let public_key = Self::public_key_from(&signing_key)?;
        self.store.write_seed(&seed)?;
        Ok(public_key)
    }

    fn delete_identity(&self) -> Result<(), PortError> {
        self.store.delete_seed()
    }
}

impl<S: CredentialStore> LocalTlsIdentityPort for IdentityCore<S> {
    fn tls_server_public_key(&self) -> Result<PeerPublicKey, PortError> {
        self.local_public_key()
    }

    fn sign_tls13_certificate_verify(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        if !is_tls13_server_certificate_verify_input(message) {
            return Err(PortError::InvalidConfiguration);
        }
        Ok(self.read_signing_key()?.sign(message).to_bytes().to_vec())
    }
}

impl<S: CredentialStore> PairingIdentityPort for IdentityCore<S> {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<PeerFingerprint, PortError> {
        Ed25519PairingIdentity.fingerprint_public_key(public_key)
    }

    fn verify_signature(
        &self,
        public_key: &PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError> {
        Ed25519PairingIdentity.verify_signature(public_key, message, signature)
    }
}

fn is_canonical_pairing_message(message: &[u8]) -> bool {
    const IDENTIFIER_BYTES: usize = 16 * 3;
    const NAME_LENGTH_BYTES: usize = 4;
    const NAME_START: usize = IDENTIFIER_BYTES + NAME_LENGTH_BYTES;

    let Some(body) = message.strip_prefix(PAIRING_PROOF_DOMAIN) else {
        return false;
    };
    let Some(name_length_bytes) = body.get(IDENTIFIER_BYTES..NAME_START) else {
        return false;
    };
    let Ok(name_length_bytes) = <[u8; NAME_LENGTH_BYTES]>::try_from(name_length_bytes) else {
        return false;
    };
    let name_length = u32::from_be_bytes(name_length_bytes) as usize;
    if name_length == 0 || name_length > MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS * 4 {
        return false;
    }
    let Some(expected_length) = NAME_START
        .checked_add(name_length)
        .and_then(|length| length.checked_add(1))
    else {
        return false;
    };
    if body.len() != expected_length {
        return false;
    }
    let Some(name_bytes) = body.get(NAME_START..NAME_START + name_length) else {
        return false;
    };
    let Ok(name) = std::str::from_utf8(name_bytes) else {
        return false;
    };
    let character_count = name.chars().count();
    if character_count == 0
        || character_count > MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return false;
    }
    let grants = body[expected_length - 1];
    grants != 0 && grants & !0b111 == 0
}

fn is_tls13_server_certificate_verify_input(message: &[u8]) -> bool {
    const SIGNATURE_INPUT_PREFIX_BYTES: usize = 64;

    let context_start = SIGNATURE_INPUT_PREFIX_BYTES;
    let transcript_hash_start = context_start + TLS13_SERVER_CERT_VERIFY_CONTEXT.len();
    let Some(transcript_hash_bytes) = message.len().checked_sub(transcript_hash_start) else {
        return false;
    };

    message[..SIGNATURE_INPUT_PREFIX_BYTES]
        .iter()
        .all(|byte| *byte == 0x20)
        && message[context_start..transcript_hash_start] == *TLS13_SERVER_CERT_VERIFY_CONTEXT
        && matches!(transcript_hash_bytes, 32 | 48)
}
