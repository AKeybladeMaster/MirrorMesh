use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::{PeerFingerprint, PeerPublicKey};
use mm_ports::{LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError};
use zeroize::Zeroizing;

const ED25519_SEED_BYTES: usize = 32;
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
                let mut seed = Zeroizing::new([0_u8; ED25519_SEED_BYTES]);
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
        let mut seed = Zeroizing::new([0_u8; ED25519_SEED_BYTES]);
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
        let mut seed = Zeroizing::new([0_u8; ED25519_SEED_BYTES]);
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
    mm_domain::is_canonical_pairing_message(message)
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
