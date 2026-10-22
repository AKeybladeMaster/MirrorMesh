use std::sync::{Mutex, MutexGuard, OnceLock};

use mm_domain::{PeerFingerprint, PeerPublicKey};
use mm_ports::{LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError};
use zeroize::Zeroizing;

use crate::identity::{CredentialStore, IdentityCore};

const ED25519_SEED_BYTES: usize = 32;

unsafe extern "C" {
    fn mirrormesh_ios_keychain_read_seed(output: *mut u8, capacity: usize) -> i32;
    fn mirrormesh_ios_keychain_write_seed(seed: *const u8, length: usize) -> i32;
    fn mirrormesh_ios_keychain_delete_seed() -> i32;
}

struct KeychainCredentialStore;

impl CredentialStore for KeychainCredentialStore {
    fn read_seed(&self) -> Result<Option<Zeroizing<Vec<u8>>>, PortError> {
        let mut seed = Zeroizing::new([0_u8; ED25519_SEED_BYTES]);
        // SAFETY: The Swift bridge writes at most `capacity` bytes and the provided array
        // remains valid for the duration of the synchronous call.
        let status = unsafe { mirrormesh_ios_keychain_read_seed(seed.as_mut_ptr(), seed.len()) };
        match status {
            0 => Ok(Some(Zeroizing::new(seed.to_vec()))),
            1 => Ok(None),
            -2 => Err(PortError::StorageFailure),
            _ => Err(PortError::Unavailable),
        }
    }

    fn write_seed(&self, seed: &[u8; ED25519_SEED_BYTES]) -> Result<(), PortError> {
        // SAFETY: The Swift bridge reads exactly `length` bytes before returning.
        let status = unsafe { mirrormesh_ios_keychain_write_seed(seed.as_ptr(), seed.len()) };
        if status == 0 {
            Ok(())
        } else {
            Err(PortError::StorageFailure)
        }
    }

    fn delete_seed(&self) -> Result<(), PortError> {
        // SAFETY: The Swift bridge takes no pointers and completes synchronously.
        let status = unsafe { mirrormesh_ios_keychain_delete_seed() };
        if status == 0 {
            Ok(())
        } else {
            Err(PortError::StorageFailure)
        }
    }
}

pub struct IosPairingIdentity {
    inner: IdentityCore<KeychainCredentialStore>,
}

impl IosPairingIdentity {
    pub fn open_or_create() -> Result<Self, PortError> {
        let _lock = identity_lock()?;
        let inner = IdentityCore::open_or_create(KeychainCredentialStore)?;
        Ok(Self { inner })
    }
}

impl LocalPairingIdentityPort for IosPairingIdentity {
    fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
        let _lock = identity_lock()?;
        self.inner.local_public_key()
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        let _lock = identity_lock()?;
        self.inner.sign_pairing_message(message)
    }

    fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
        let _lock = identity_lock()?;
        self.inner.rotate_identity()
    }

    fn delete_identity(&self) -> Result<(), PortError> {
        let _lock = identity_lock()?;
        self.inner.delete_identity()
    }
}

impl LocalTlsIdentityPort for IosPairingIdentity {
    fn tls_server_public_key(&self) -> Result<PeerPublicKey, PortError> {
        let _lock = identity_lock()?;
        self.inner.tls_server_public_key()
    }

    fn sign_tls13_certificate_verify(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        let _lock = identity_lock()?;
        self.inner.sign_tls13_certificate_verify(message)
    }
}

impl PairingIdentityPort for IosPairingIdentity {
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

fn identity_lock() -> Result<MutexGuard<'static, ()>, PortError> {
    static IDENTITY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    IDENTITY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| PortError::StorageFailure)
}
