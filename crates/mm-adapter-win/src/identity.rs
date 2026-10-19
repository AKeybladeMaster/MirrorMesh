use std::ptr::{NonNull, null, null_mut};

use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::{MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS, PeerFingerprint, PeerPublicKey};
use mm_ports::{LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError};
use zeroize::{Zeroize, Zeroizing};

const ED25519_SEED_BYTES: usize = 32;
const CREDENTIAL_TARGET: &str = "MirrorMesh/Identity/Ed25519/v1";
const IDENTITY_MUTEX_NAME: &str = "Local\\MirrorMesh.Identity.Ed25519.v1";
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
        let signing_key = self.read_signing_key()?;
        Self::public_key_from(&signing_key)
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        if !is_canonical_pairing_message(message) {
            return Err(PortError::InvalidConfiguration);
        }
        let signing_key = self.read_signing_key()?;
        Ok(signing_key.sign(message).to_bytes().to_vec())
    }

    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
        let mut seed = Zeroizing::new([0u8; ED25519_SEED_BYTES]);
        getrandom::fill(&mut *seed).map_err(|_| PortError::Unavailable)?;
        let signing_key = SigningKey::from_bytes(&seed);
        let public_key = Self::public_key_from(&signing_key)?;

        // Keep the existing persisted identity unless replacement succeeds.
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
        let signing_key = self.read_signing_key()?;
        Ok(signing_key.sign(message).to_bytes().to_vec())
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

pub struct WindowsPairingIdentity {
    inner: IdentityCore<WindowsCredentialStore>,
}

impl WindowsPairingIdentity {
    /// Opens the per-user Ed25519 identity, creating it only when Credential Manager has no key.
    pub fn open_or_create() -> Result<Self, PortError> {
        let _identity_lock = IdentityMutex::acquire()?;
        Ok(Self {
            inner: IdentityCore::open_or_create(WindowsCredentialStore)?,
        })
    }
}

impl LocalPairingIdentityPort for WindowsPairingIdentity {
    fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
        let _identity_lock = IdentityMutex::acquire()?;
        self.inner.local_public_key()
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        let _identity_lock = IdentityMutex::acquire()?;
        self.inner.sign_pairing_message(message)
    }

    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
        let _identity_lock = IdentityMutex::acquire()?;
        self.inner.rotate_identity()
    }

    fn delete_identity(&self) -> Result<(), PortError> {
        let _identity_lock = IdentityMutex::acquire()?;
        self.inner.delete_identity()
    }
}

impl LocalTlsIdentityPort for WindowsPairingIdentity {
    fn tls_server_public_key(&self) -> Result<PeerPublicKey, PortError> {
        let _identity_lock = IdentityMutex::acquire()?;
        self.inner.tls_server_public_key()
    }

    fn sign_tls13_certificate_verify(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        let _identity_lock = IdentityMutex::acquire()?;
        self.inner.sign_tls13_certificate_verify(message)
    }
}

impl PairingIdentityPort for WindowsPairingIdentity {
    fn fingerprint_public_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Result<PeerFingerprint, PortError> {
        self.inner.fingerprint_public_key(public_key)
    }

    fn verify_signature(
        &self,
        public_key: &PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError> {
        self.inner.verify_signature(public_key, message, signature)
    }
}

struct WindowsCredentialStore;

impl CredentialStore for WindowsCredentialStore {
    fn read_seed(&self) -> Result<Option<Zeroizing<Vec<u8>>>, PortError> {
        use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
        use windows_sys::Win32::Security::Credentials::{CRED_TYPE_GENERIC, CredReadW};

        let target = wide_string(CREDENTIAL_TARGET);
        let mut credential = null_mut();
        let success = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if success == 0 {
            return if unsafe { GetLastError() } == ERROR_NOT_FOUND {
                Ok(None)
            } else {
                Err(PortError::StorageFailure)
            };
        }

        let credential = NonNull::new(credential).ok_or(PortError::StorageFailure)?;
        let credential = CredentialBuffer(credential);
        let record = unsafe { credential.0.as_ref() };
        let size = record.CredentialBlobSize as usize;
        if size > 0 && record.CredentialBlob.is_null() {
            return Err(PortError::StorageFailure);
        }
        if size > windows_sys::Win32::Security::Credentials::CRED_MAX_CREDENTIAL_BLOB_SIZE as usize
        {
            return Err(PortError::StorageFailure);
        }

        let bytes = if size == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(record.CredentialBlob, size) }.to_vec()
        };
        Ok(Some(Zeroizing::new(bytes)))
    }

    fn write_seed(&self, seed: &[u8; ED25519_SEED_BYTES]) -> Result<(), PortError> {
        use windows_sys::Win32::Security::Credentials::{
            CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredWriteW,
        };

        let mut target = wide_string(CREDENTIAL_TARGET);
        let mut stored_seed = Zeroizing::new(*seed);
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: stored_seed.len() as u32,
            CredentialBlob: stored_seed.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..CREDENTIALW::default()
        };
        let success = unsafe { CredWriteW(&credential, 0) };
        if success == 0 {
            return Err(PortError::StorageFailure);
        }
        Ok(())
    }

    fn delete_seed(&self) -> Result<(), PortError> {
        use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
        use windows_sys::Win32::Security::Credentials::{CRED_TYPE_GENERIC, CredDeleteW};

        let target = wide_string(CREDENTIAL_TARGET);
        let success = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if success == 0 && unsafe { GetLastError() } != ERROR_NOT_FOUND {
            return Err(PortError::StorageFailure);
        }
        Ok(())
    }
}

struct CredentialBuffer(NonNull<windows_sys::Win32::Security::Credentials::CREDENTIALW>);

impl Drop for CredentialBuffer {
    fn drop(&mut self) {
        use windows_sys::Win32::Security::Credentials::{CRED_MAX_CREDENTIAL_BLOB_SIZE, CredFree};

        let record = unsafe { self.0.as_mut() };
        if !record.CredentialBlob.is_null()
            && record.CredentialBlobSize <= CRED_MAX_CREDENTIAL_BLOB_SIZE
        {
            let secret = unsafe {
                std::slice::from_raw_parts_mut(
                    record.CredentialBlob,
                    record.CredentialBlobSize as usize,
                )
            };
            secret.zeroize();
        }
        unsafe { CredFree(self.0.as_ptr().cast()) };
    }
}

struct IdentityMutex(windows_sys::Win32::Foundation::HANDLE);

impl IdentityMutex {
    fn acquire() -> Result<Self, PortError> {
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{CreateMutexW, INFINITE, WaitForSingleObject};

        let name = wide_string(IDENTITY_MUTEX_NAME);
        let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(PortError::StorageFailure);
        }

        let result = unsafe { WaitForSingleObject(handle, INFINITE) };
        if result != WAIT_OBJECT_0 && result != WAIT_ABANDONED {
            unsafe { CloseHandle(handle) };
            return Err(PortError::StorageFailure);
        }
        Ok(Self(handle))
    }
}

impl Drop for IdentityMutex {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::ReleaseMutex;

        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
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
    const TLS13_SIGNATURE_INPUT_PREFIX_BYTES: usize = 64 + TLS13_SERVER_CERT_VERIFY_CONTEXT.len();
    const SHA256_TRANSCRIPT_HASH_BYTES: usize = 32;
    const SHA384_TRANSCRIPT_HASH_BYTES: usize = 48;

    (message.len() == TLS13_SIGNATURE_INPUT_PREFIX_BYTES + SHA256_TRANSCRIPT_HASH_BYTES
        || message.len() == TLS13_SIGNATURE_INPUT_PREFIX_BYTES + SHA384_TRANSCRIPT_HASH_BYTES)
        && message[..64].iter().all(|byte| *byte == 0x20)
        && message[64..TLS13_SIGNATURE_INPUT_PREFIX_BYTES] == TLS13_SERVER_CERT_VERIFY_CONTEXT[..]
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}
