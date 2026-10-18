use std::sync::{Arc, Mutex};

use ed25519_dalek::SigningKey;
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::PeerPublicKey;
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
use zeroize::Zeroizing;

use super::identity::{CredentialStore, IdentityCore};

#[derive(Clone, Default)]
struct FakeStore {
    seed: Arc<Mutex<Option<Zeroizing<Vec<u8>>>>>,
    fail_read: Arc<Mutex<bool>>,
    fail_write: Arc<Mutex<bool>>,
    fail_delete: Arc<Mutex<bool>>,
}

impl FakeStore {
    fn with_seed(seed: Vec<u8>) -> Self {
        Self {
            seed: Arc::new(Mutex::new(Some(Zeroizing::new(seed)))),
            ..Self::default()
        }
    }

    fn stored_seed(&self) -> Option<Vec<u8>> {
        self.seed.lock().unwrap().as_ref().map(|seed| seed.to_vec())
    }

    fn fail_writes(&self) {
        *self.fail_write.lock().unwrap() = true;
    }

    fn allow_writes(&self) {
        *self.fail_write.lock().unwrap() = false;
    }

    fn fail_deletes(&self) {
        *self.fail_delete.lock().unwrap() = true;
    }
}

impl CredentialStore for FakeStore {
    fn read_seed(&self) -> Result<Option<Zeroizing<Vec<u8>>>, PortError> {
        if *self.fail_read.lock().unwrap() {
            return Err(PortError::StorageFailure);
        }
        Ok(self
            .seed
            .lock()
            .unwrap()
            .as_ref()
            .map(|seed| Zeroizing::new(seed.to_vec())))
    }

    fn write_seed(&self, seed: &[u8; 32]) -> Result<(), PortError> {
        if *self.fail_write.lock().unwrap() {
            return Err(PortError::StorageFailure);
        }
        *self.seed.lock().unwrap() = Some(Zeroizing::new(seed.to_vec()));
        Ok(())
    }

    fn delete_seed(&self) -> Result<(), PortError> {
        if *self.fail_delete.lock().unwrap() {
            return Err(PortError::StorageFailure);
        }
        self.seed.lock().unwrap().take();
        Ok(())
    }
}

fn pairing_message(name: &str) -> Vec<u8> {
    let name = name.as_bytes();
    let mut message = b"MirrorMesh.PairingProof.v1\0".to_vec();
    message.extend_from_slice(&[1; 16]);
    message.extend_from_slice(&[2; 16]);
    message.extend_from_slice(&[3; 16]);
    message.extend_from_slice(&(name.len() as u32).to_be_bytes());
    message.extend_from_slice(name);
    message.push(1);
    message
}

#[test]
fn creates_a_persisted_identity_once_and_signs_verifiable_pairing_messages() {
    let store = FakeStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let public_key = identity.local_public_key().unwrap();
    let message = pairing_message("linux receiver");
    let signature = identity.sign_pairing_message(&message).unwrap();

    Ed25519PairingIdentity
        .verify_signature(&public_key, &message, &signature)
        .unwrap();
    let reopened = IdentityCore::open_or_create(store).unwrap();
    assert_eq!(reopened.local_public_key().unwrap(), public_key);
    Ed25519PairingIdentity
        .verify_signature(&public_key, &message, &signature)
        .unwrap();
}

#[test]
fn rotation_updates_persisted_identity_before_returning_the_new_key() {
    let store = FakeStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let old_key = identity.local_public_key().unwrap();
    let message = pairing_message("rotated receiver");

    let new_key = identity.rotate_identity().unwrap();
    let new_signature = identity.sign_pairing_message(&message).unwrap();

    assert_ne!(new_key, old_key);
    assert_eq!(identity.local_public_key().unwrap(), new_key);
    assert_eq!(
        Ed25519PairingIdentity.verify_signature(&old_key, &message, &new_signature),
        Err(PortError::IdentityRejected)
    );
    Ed25519PairingIdentity
        .verify_signature(&new_key, &message, &new_signature)
        .unwrap();
}

#[test]
fn deletion_removes_the_persisted_key_and_disables_the_loaded_identity() {
    let store = FakeStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let old_key = identity.local_public_key().unwrap();

    identity.delete_identity().unwrap();

    assert!(store.stored_seed().is_none());
    assert_eq!(identity.local_public_key(), Err(PortError::Unavailable));
    assert_eq!(
        identity.sign_pairing_message(&pairing_message("after deletion")),
        Err(PortError::Unavailable)
    );
    assert_ne!(
        IdentityCore::open_or_create(store)
            .unwrap()
            .local_public_key()
            .unwrap(),
        old_key
    );
}

#[test]
fn malformed_persisted_key_fails_closed_without_replacing_it() {
    let malformed = vec![0x42; 31];
    let store = FakeStore::with_seed(malformed.clone());

    assert_eq!(
        IdentityCore::open_or_create(store.clone()).err(),
        Some(PortError::StorageFailure)
    );
    assert_eq!(store.stored_seed(), Some(malformed));
}

#[test]
fn storage_failures_never_fall_back_to_an_ephemeral_identity() {
    let read_failure = FakeStore::default();
    *read_failure.fail_read.lock().unwrap() = true;
    assert_eq!(
        IdentityCore::open_or_create(read_failure.clone()).err(),
        Some(PortError::StorageFailure)
    );
    assert!(read_failure.stored_seed().is_none());

    let write_failure = FakeStore::default();
    write_failure.fail_writes();
    assert_eq!(
        IdentityCore::open_or_create(write_failure.clone()).err(),
        Some(PortError::StorageFailure)
    );
    assert!(write_failure.stored_seed().is_none());
}

#[test]
fn failed_rotation_and_deletion_keep_the_current_identity_available() {
    let store = FakeStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let current_key = identity.local_public_key().unwrap();
    store.fail_writes();

    assert_eq!(identity.rotate_identity(), Err(PortError::StorageFailure));
    assert_eq!(identity.local_public_key().unwrap(), current_key);
    assert!(
        identity
            .sign_pairing_message(&pairing_message("still available"))
            .is_ok()
    );

    store.allow_writes();
    store.fail_deletes();
    assert_eq!(identity.delete_identity(), Err(PortError::StorageFailure));
    assert!(store.stored_seed().is_some());
    assert!(
        identity
            .sign_pairing_message(&pairing_message("still available"))
            .is_ok()
    );
}

#[test]
fn signing_rejects_messages_outside_the_canonical_pairing_proof() {
    let identity = IdentityCore::open_or_create(FakeStore::default()).unwrap();

    assert_eq!(
        identity.sign_pairing_message(b"unrelated message"),
        Err(PortError::InvalidConfiguration)
    );
    let mut canonical = pairing_message("peer");
    canonical.push(0);
    assert_eq!(
        identity.sign_pairing_message(&canonical),
        Err(PortError::InvalidConfiguration)
    );
}

#[test]
fn known_seed_produces_its_expected_ed25519_public_key() {
    let seed = [0x11; 32];
    let store = FakeStore::with_seed(seed.to_vec());
    let expected = PeerPublicKey::new(
        SigningKey::from_bytes(&seed)
            .verifying_key()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();

    assert_eq!(
        IdentityCore::open_or_create(store)
            .unwrap()
            .local_public_key()
            .unwrap(),
        expected
    );
}
