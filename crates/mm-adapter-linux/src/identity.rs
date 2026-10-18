#![cfg(any(test, target_os = "linux"))]

use ed25519_dalek::{Signer, SigningKey};
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::{MAX_TRUSTED_PEER_DISPLAY_NAME_CHARS, PeerFingerprint, PeerPublicKey};
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
use zeroize::Zeroizing;

const ED25519_SEED_BYTES: usize = 32;
const PAIRING_PROOF_DOMAIN: &[u8] = b"MirrorMesh.PairingProof.v1\0";

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

#[cfg(target_os = "linux")]
mod secret_service_store {
    use std::env;
    use std::fs::{self, File, OpenOptions};
    use std::io;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::PathBuf;

    use mm_domain::{PeerFingerprint, PeerPublicKey};
    use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
    use secret_service::blocking::{Item, SecretService};
    use secret_service::{EncryptionType, SearchItemsResult};
    use zeroize::Zeroizing;

    use super::{CredentialStore, IdentityCore};

    const APPLICATION_ATTRIBUTE: &str = "application";
    const APPLICATION_ID: &str = "org.mirrormesh.MirrorMesh";
    const PURPOSE_ATTRIBUTE: &str = "purpose";
    const PURPOSE: &str = "pairing-identity-ed25519-v1";
    const ITEM_LABEL: &str = "MirrorMesh local pairing identity";
    const SECRET_CONTENT_TYPE: &str = "application/octet-stream";
    const LOCK_FILE_NAME: &str = "mirrormesh-pairing-identity.lock";

    pub struct LinuxPairingIdentity {
        inner: IdentityCore<SecretServiceCredentialStore>,
    }

    impl LinuxPairingIdentity {
        /// Opens the user's Secret Service identity, creating it only if it is absent.
        ///
        /// Call this after the user starts pairing. It can block while the desktop keyring
        /// service unlocks a collection or displays its own authorization prompt.
        pub fn open_or_create() -> Result<Self, PortError> {
            let _identity_lock = IdentityLock::acquire()?;
            let inner = IdentityCore::open_or_create(SecretServiceCredentialStore)?;
            Ok(Self { inner })
        }
    }

    impl LocalPairingIdentityPort for LinuxPairingIdentity {
        fn local_public_key(&self) -> Result<PeerPublicKey, PortError> {
            let _identity_lock = IdentityLock::acquire()?;
            self.inner.local_public_key()
        }

        fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
            let _identity_lock = IdentityLock::acquire()?;
            self.inner.sign_pairing_message(message)
        }

        fn rotate_identity(&self) -> Result<PeerPublicKey, PortError> {
            let _identity_lock = IdentityLock::acquire()?;
            self.inner.rotate_identity()
        }

        fn delete_identity(&self) -> Result<(), PortError> {
            let _identity_lock = IdentityLock::acquire()?;
            self.inner.delete_identity()
        }
    }

    impl PairingIdentityPort for LinuxPairingIdentity {
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

    struct SecretServiceCredentialStore;

    impl CredentialStore for SecretServiceCredentialStore {
        fn read_seed(&self) -> Result<Option<Zeroizing<Vec<u8>>>, PortError> {
            let service = connect_secret_service()?;
            let matches = search_identity(&service)?;
            let Some((item, locked)) = take_unique_item(matches)? else {
                return Ok(None);
            };
            if locked {
                item.unlock().map_err(|_| PortError::StorageFailure)?;
            }
            let secret = item.get_secret().map_err(|_| PortError::StorageFailure)?;
            Ok(Some(Zeroizing::new(secret)))
        }

        fn write_seed(&self, seed: &[u8; 32]) -> Result<(), PortError> {
            let service = connect_secret_service()?;
            let matches = search_identity(&service)?;
            if let Some((item, locked)) = take_unique_item(matches)? {
                if locked {
                    item.unlock().map_err(|_| PortError::StorageFailure)?;
                }
                item.set_secret(seed, SECRET_CONTENT_TYPE)
                    .map_err(|_| PortError::StorageFailure)?;
                return Ok(());
            }

            let collection = service
                .get_default_collection()
                .map_err(|_| PortError::StorageFailure)?;
            collection
                .create_item(
                    ITEM_LABEL,
                    identity_attributes(),
                    seed,
                    false,
                    SECRET_CONTENT_TYPE,
                )
                .map_err(|_| PortError::StorageFailure)?;
            Ok(())
        }

        fn delete_seed(&self) -> Result<(), PortError> {
            let service = connect_secret_service()?;
            let matches = search_identity(&service)?;
            let Some((item, locked)) = take_unique_item(matches)? else {
                return Ok(());
            };
            if locked {
                item.unlock().map_err(|_| PortError::StorageFailure)?;
            }
            item.delete().map_err(|_| PortError::StorageFailure)
        }
    }

    fn connect_secret_service<'a>() -> Result<SecretService<'a>, PortError> {
        SecretService::connect(EncryptionType::Dh).map_err(|_| PortError::StorageFailure)
    }

    fn identity_attributes() -> std::collections::HashMap<&'static str, &'static str> {
        std::collections::HashMap::from([
            (APPLICATION_ATTRIBUTE, APPLICATION_ID),
            (PURPOSE_ATTRIBUTE, PURPOSE),
        ])
    }

    fn search_identity<'a>(
        service: &'a SecretService<'a>,
    ) -> Result<SearchItemsResult<Item<'a>>, PortError> {
        service
            .search_items(identity_attributes())
            .map_err(|_| PortError::StorageFailure)
    }

    fn take_unique_item<'a>(
        mut matches: SearchItemsResult<Item<'a>>,
    ) -> Result<Option<(Item<'a>, bool)>, PortError> {
        if matches.unlocked.len() + matches.locked.len() > 1 {
            return Err(PortError::StorageFailure);
        }
        if let Some(item) = matches.unlocked.pop() {
            return Ok(Some((item, false)));
        }
        Ok(matches.locked.pop().map(|item| (item, true)))
    }

    struct IdentityLock(File);

    impl IdentityLock {
        fn acquire() -> Result<Self, PortError> {
            let runtime_dir = env::var_os("XDG_RUNTIME_DIR").ok_or(PortError::StorageFailure)?;
            let runtime_dir = PathBuf::from(runtime_dir);
            let runtime_metadata =
                fs::symlink_metadata(&runtime_dir).map_err(|_| PortError::StorageFailure)?;
            let effective_uid = unsafe { libc::geteuid() };
            if runtime_metadata.file_type().is_symlink()
                || !runtime_metadata.is_dir()
                || runtime_metadata.uid() != effective_uid
                || runtime_metadata.mode() & 0o077 != 0
            {
                return Err(PortError::StorageFailure);
            }

            let lock_path = runtime_dir.join(LOCK_FILE_NAME);
            let lock_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open(lock_path)
                .map_err(|_| PortError::StorageFailure)?;
            let lock_metadata = lock_file
                .metadata()
                .map_err(|_| PortError::StorageFailure)?;
            if !lock_metadata.is_file()
                || lock_metadata.uid() != effective_uid
                || lock_metadata.mode() & 0o077 != 0
                || lock_metadata.nlink() != 1
            {
                return Err(PortError::StorageFailure);
            }

            loop {
                let result = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX) };
                if result == 0 {
                    return Ok(Self(lock_file));
                }
                if io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                    return Err(PortError::StorageFailure);
                }
            }
        }
    }

    impl Drop for IdentityLock {
        fn drop(&mut self) {
            // Closing the file also releases flock if explicit unlock fails.
            let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

#[cfg(target_os = "linux")]
pub use secret_service_store::LinuxPairingIdentity;
