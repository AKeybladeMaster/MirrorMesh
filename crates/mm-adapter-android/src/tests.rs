use std::sync::{Arc, Mutex};

use ed25519_dalek::SigningKey;
use mm_adapter_identity::Ed25519PairingIdentity;
use mm_domain::PeerPublicKey;
use mm_ports::{LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort, PortError};
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

    fn fail_reads(&self) {
        *self.fail_read.lock().unwrap() = true;
    }

    fn fail_writes(&self) {
        *self.fail_write.lock().unwrap() = true;
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

fn tls13_certificate_verify_message(transcript_hash_bytes: usize) -> Vec<u8> {
    let mut message = vec![0x20; 64];
    message.extend_from_slice(b"TLS 1.3, server CertificateVerify\0");
    message.resize(message.len() + transcript_hash_bytes, 0x5a);
    message
}

#[test]
fn signs_valid_tls13_server_certificate_verify_inputs() {
    let identity = IdentityCore::open_or_create(FakeStore::default()).unwrap();

    for transcript_hash_bytes in [32, 48] {
        let message = tls13_certificate_verify_message(transcript_hash_bytes);
        let public_key = identity.tls_server_public_key().unwrap();
        let signature = identity.sign_tls13_certificate_verify(&message).unwrap();
        Ed25519PairingIdentity
            .verify_signature(&public_key, &message, &signature)
            .unwrap();
    }
}

#[test]
fn rejects_invalid_certificate_verify_inputs_before_store_access() {
    let store = FakeStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    store.fail_reads();

    let mut wrong_context = tls13_certificate_verify_message(32);
    wrong_context[64] = b'X';
    let mut wrong_padding = tls13_certificate_verify_message(32);
    wrong_padding[0] = 0;

    for message in [
        b"application message".as_slice(),
        &tls13_certificate_verify_message(31),
        &tls13_certificate_verify_message(64),
        &wrong_context,
        &wrong_padding,
    ] {
        assert_eq!(
            identity.sign_tls13_certificate_verify(message),
            Err(PortError::InvalidConfiguration)
        );
    }
}

#[test]
fn tls_signer_uses_the_rotated_android_identity() {
    let identity = IdentityCore::open_or_create(FakeStore::default()).unwrap();
    let old_public_key = identity.tls_server_public_key().unwrap();
    let message = tls13_certificate_verify_message(32);
    let old_signature = identity.sign_tls13_certificate_verify(&message).unwrap();

    let new_public_key = identity.rotate_identity().unwrap();
    let new_signature = identity.sign_tls13_certificate_verify(&message).unwrap();

    assert_ne!(old_public_key, new_public_key);
    Ed25519PairingIdentity
        .verify_signature(&old_public_key, &message, &old_signature)
        .unwrap();
    assert_eq!(
        Ed25519PairingIdentity.verify_signature(&old_public_key, &message, &new_signature),
        Err(PortError::IdentityRejected)
    );
    Ed25519PairingIdentity
        .verify_signature(&new_public_key, &message, &new_signature)
        .unwrap();
}

#[test]
fn identity_store_failures_fail_closed() {
    let store = FakeStore::default();
    store.fail_reads();
    assert_eq!(
        IdentityCore::open_or_create(store).err(),
        Some(PortError::StorageFailure)
    );

    let store = FakeStore::default();
    store.fail_writes();
    assert_eq!(
        IdentityCore::open_or_create(store).err(),
        Some(PortError::StorageFailure)
    );

    let store = FakeStore::default();
    let identity = IdentityCore::open_or_create(store.clone()).unwrap();
    let current_key = identity.local_public_key().unwrap();
    store.fail_writes();
    assert_eq!(identity.rotate_identity(), Err(PortError::StorageFailure));
    assert_eq!(identity.local_public_key().unwrap(), current_key);
    store.fail_deletes();
    assert_eq!(identity.delete_identity(), Err(PortError::StorageFailure));
    assert_eq!(identity.local_public_key().unwrap(), current_key);
}

#[test]
fn known_seed_preserves_the_shared_ed25519_identity_encoding() {
    let seed = [0x11; 32];
    let expected = PeerPublicKey::new(
        SigningKey::from_bytes(&seed)
            .verifying_key()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(
        IdentityCore::open_or_create(FakeStore::with_seed(seed.to_vec()))
            .unwrap()
            .local_public_key()
            .unwrap(),
        expected
    );
}
