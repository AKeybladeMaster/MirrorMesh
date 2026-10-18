use std::sync::{Arc, Mutex};

use ed25519_dalek::SigningKey;
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::PeerPublicKey;
use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
use zeroize::Zeroizing;

use super::identity::{CredentialStore, IdentityCore};

#[derive(Clone, Default)]
struct FakeCredentialStore {
    seed: Arc<Mutex<Option<Zeroizing<Vec<u8>>>>>,
    fail_read: Arc<Mutex<bool>>,
    fail_write: Arc<Mutex<bool>>,
    fail_delete: Arc<Mutex<bool>>,
}

impl FakeCredentialStore {
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

    fn fail_deletes(&self) {
        *self.fail_delete.lock().unwrap() = true;
    }
}

impl CredentialStore for FakeCredentialStore {
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

fn verify(identity: &IdentityCore<FakeCredentialStore>, message: &[u8], signature: &[u8]) {
    let public_key = identity.local_public_key().unwrap();
    Ed25519PairingIdentity
        .verify_signature(&public_key, message, signature)
        .unwrap();
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
    let store = FakeCredentialStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let public_key = identity.local_public_key().unwrap();
    let message = pairing_message("receiver-bound proof");
    let signature = identity.sign_pairing_message(&message).unwrap();

    verify(&identity, &message, &signature);
    let reopened = IdentityCore::open_or_create(store).unwrap();
    assert_eq!(reopened.local_public_key().unwrap(), public_key);
    verify(&reopened, &message, &signature);
}

#[test]
fn rotates_the_key_only_after_the_new_seed_is_persisted() {
    let store = FakeCredentialStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let other_handle = IdentityCore::open_or_create(store.clone()).unwrap();
    let old_key = identity.local_public_key().unwrap();
    let message = pairing_message("old identity");
    let old_signature = identity.sign_pairing_message(&message).unwrap();

    let new_key = identity.rotate_identity().unwrap();
    let new_signature = identity.sign_pairing_message(&message).unwrap();

    assert_ne!(new_key, old_key);
    assert_ne!(new_signature, old_signature);
    assert_eq!(other_handle.local_public_key().unwrap(), new_key);
    assert_eq!(
        Ed25519PairingIdentity.verify_signature(&old_key, &message, &new_signature),
        Err(PortError::IdentityRejected)
    );
    verify(&identity, &message, &new_signature);
    assert_eq!(
        IdentityCore::open_or_create(store)
            .unwrap()
            .local_public_key()
            .unwrap(),
        new_key
    );
}

#[test]
fn deletion_removes_persisted_secret_and_disables_the_loaded_signer() {
    let store = FakeCredentialStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let old_key = identity.local_public_key().unwrap();

    identity.delete_identity().unwrap();

    assert!(store.stored_seed().is_none());
    assert_eq!(identity.local_public_key(), Err(PortError::Unavailable));
    assert_eq!(
        identity.sign_pairing_message(&pairing_message("after deletion")),
        Err(PortError::Unavailable)
    );
    let replacement = IdentityCore::open_or_create(store).unwrap();
    assert_ne!(replacement.local_public_key().unwrap(), old_key);
}

#[test]
fn refuses_a_malformed_stored_seed_without_replacing_it() {
    let malformed = vec![0x42; 31];
    let store = FakeCredentialStore::with_seed(malformed.clone());

    assert!(matches!(
        IdentityCore::open_or_create(store.clone()),
        Err(PortError::StorageFailure)
    ));
    assert_eq!(store.stored_seed(), Some(malformed));
}

#[test]
fn storage_errors_never_fall_back_to_an_ephemeral_identity() {
    let read_failure = FakeCredentialStore::default();
    *read_failure.fail_read.lock().unwrap() = true;
    assert!(matches!(
        IdentityCore::open_or_create(read_failure.clone()),
        Err(PortError::StorageFailure)
    ));
    assert!(read_failure.stored_seed().is_none());

    let write_failure = FakeCredentialStore::default();
    write_failure.fail_writes();
    assert!(matches!(
        IdentityCore::open_or_create(write_failure.clone()),
        Err(PortError::StorageFailure)
    ));
    assert!(write_failure.stored_seed().is_none());
}

#[test]
fn failed_rotation_keeps_the_existing_identity_and_signing_access() {
    let store = FakeCredentialStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let current_key = identity.local_public_key().unwrap();
    store.fail_writes();

    assert_eq!(identity.rotate_identity(), Err(PortError::StorageFailure));
    assert_eq!(identity.local_public_key().unwrap(), current_key);
    assert!(
        identity
            .sign_pairing_message(&pairing_message("existing key"))
            .is_ok()
    );
}

#[test]
fn failed_deletion_keeps_the_persisted_identity_available() {
    let store = FakeCredentialStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
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
fn refuses_to_sign_messages_outside_the_pairing_domain() {
    let identity = IdentityCore::open_or_create(FakeCredentialStore::default()).unwrap();

    assert_eq!(
        identity.sign_pairing_message(b"unrelated message"),
        Err(PortError::InvalidConfiguration)
    );
}

#[test]
fn refuses_noncanonical_payloads_even_with_the_pairing_domain() {
    let identity = IdentityCore::open_or_create(FakeCredentialStore::default()).unwrap();
    let canonical = pairing_message("proof");
    let domain_length = b"MirrorMesh.PairingProof.v1\0".len();
    let name_length_offset = domain_length + 3 * 16;
    let name_offset = name_length_offset + 4;

    let mut cases = vec![b"MirrorMesh.PairingProof.v1\0garbage".to_vec()];
    let mut truncated = canonical.clone();
    truncated.pop();
    cases.push(truncated);

    let mut wrong_name_length = canonical.clone();
    wrong_name_length[name_length_offset + 3] += 1;
    cases.push(wrong_name_length);

    let mut invalid_utf8 = canonical.clone();
    invalid_utf8[name_offset] = 0xff;
    cases.push(invalid_utf8);

    let mut control_character = canonical.clone();
    control_character[name_offset] = 0;
    cases.push(control_character);

    let mut no_grants = canonical.clone();
    *no_grants.last_mut().unwrap() = 0;
    cases.push(no_grants);

    let mut unknown_grant = canonical.clone();
    *unknown_grant.last_mut().unwrap() = 0x81;
    cases.push(unknown_grant);

    let mut trailing_bytes = canonical;
    trailing_bytes.push(0);
    cases.push(trailing_bytes);

    for message in cases {
        assert_eq!(
            identity.sign_pairing_message(&message),
            Err(PortError::InvalidConfiguration),
            "accepted malformed proof message: {message:?}"
        );
    }
}

#[test]
fn known_seed_produces_the_expected_ed25519_public_key() {
    let seed = [0x11; 32];
    let store = FakeCredentialStore::with_seed(seed.to_vec());
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
