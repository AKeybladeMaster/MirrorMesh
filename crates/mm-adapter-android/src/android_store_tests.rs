use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use mm_ports::{LocalPairingIdentityPort, PortError};

use super::{CredentialStore, EncryptedSeedFile, RECORD_MAGIC, SeedCipher};
use crate::identity::IdentityCore;
use zeroize::Zeroizing;

#[derive(Clone, Default)]
struct TestCipher {
    fail_encrypt: Arc<AtomicBool>,
    fail_delete: Arc<AtomicBool>,
}

impl SeedCipher for TestCipher {
    fn encrypt_seed(&self, seed: &[u8; 32]) -> Result<Vec<u8>, PortError> {
        if self.fail_encrypt.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(PortError::StorageFailure);
        }

        let mut wrapped = vec![0x42; 12];
        let ciphertext = seed.iter().map(|byte| byte ^ 0xa5).collect::<Vec<_>>();
        let checksum = ciphertext
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        wrapped.extend_from_slice(&ciphertext);
        wrapped.extend(std::iter::repeat_n(checksum, 16));
        Ok(wrapped)
    }

    fn decrypt_seed(&self, wrapped_seed: &[u8]) -> Result<Zeroizing<Vec<u8>>, PortError> {
        if wrapped_seed.len() != 60 || wrapped_seed[..12] != [0x42; 12] {
            return Err(PortError::StorageFailure);
        }
        let ciphertext = &wrapped_seed[12..44];
        let checksum = ciphertext
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        if wrapped_seed[44..].iter().any(|byte| *byte != checksum) {
            return Err(PortError::StorageFailure);
        }
        Ok(Zeroizing::new(
            ciphertext.iter().map(|byte| byte ^ 0xa5).collect(),
        ))
    }

    fn delete_wrapping_key(&self) -> Result<(), PortError> {
        if self.fail_delete.load(std::sync::atomic::Ordering::SeqCst) {
            Err(PortError::StorageFailure)
        } else {
            Ok(())
        }
    }
}

fn new_path() -> (PathBuf, PathBuf) {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "mirrormesh-android-identity-test-{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    (directory.join("identity.bin"), directory)
}

fn cleanup(directory: PathBuf) {
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn encrypted_seed_record_survives_reopen_without_plaintext_seed() {
    let (path, directory) = new_path();
    let store = EncryptedSeedFile::new(path.clone(), TestCipher::default());
    store.write_seed(&[0x37; 32]).unwrap();
    let identity =
        IdentityCore::open_or_create(EncryptedSeedFile::new(path.clone(), TestCipher::default()))
            .unwrap();
    let public_key = identity.local_public_key().unwrap();
    drop(identity);

    let record = std::fs::read(&path).unwrap();
    assert!(record.starts_with(RECORD_MAGIC));
    assert_eq!(record.len(), 65);
    assert!(
        !record
            .windows(32)
            .any(|window| window == [0x37; 32].as_slice())
    );

    let reopened =
        IdentityCore::open_or_create(EncryptedSeedFile::new(path, TestCipher::default())).unwrap();
    assert_eq!(reopened.local_public_key().unwrap(), public_key);
    cleanup(directory);
}

#[test]
fn rejects_tampered_or_malformed_seed_records() {
    let (path, directory) = new_path();
    let store = EncryptedSeedFile::new(path.clone(), TestCipher::default());
    store.write_seed(&[0x37; 32]).unwrap();
    let mut record = std::fs::read(&path).unwrap();
    record[RECORD_MAGIC.len() + 12] ^= 0x80;
    std::fs::write(&path, record).unwrap();
    assert_eq!(store.read_seed().err(), Some(PortError::StorageFailure));

    std::fs::write(&path, b"MMID\x01short").unwrap();
    assert_eq!(store.read_seed().err(), Some(PortError::StorageFailure));
    cleanup(directory);
}

#[test]
fn failed_rotation_keeps_the_previous_encrypted_identity() {
    let (path, directory) = new_path();
    let cipher = TestCipher::default();
    let identity =
        IdentityCore::open_or_create(EncryptedSeedFile::new(path.clone(), cipher.clone())).unwrap();
    let original_key = identity.local_public_key().unwrap();
    cipher
        .fail_encrypt
        .store(true, std::sync::atomic::Ordering::SeqCst);

    assert_eq!(identity.rotate_identity(), Err(PortError::StorageFailure));
    assert_eq!(identity.local_public_key().unwrap(), original_key);
    cleanup(directory);
}

#[test]
fn deleting_the_encrypted_record_allows_a_new_identity() {
    let (path, directory) = new_path();
    let identity =
        IdentityCore::open_or_create(EncryptedSeedFile::new(path.clone(), TestCipher::default()))
            .unwrap();
    let previous_key = identity.local_public_key().unwrap();
    identity.delete_identity().unwrap();
    assert!(!path.exists());

    let replacement =
        IdentityCore::open_or_create(EncryptedSeedFile::new(path, TestCipher::default())).unwrap();
    assert_ne!(replacement.local_public_key().unwrap(), previous_key);
    cleanup(directory);
}

#[test]
fn key_cleanup_failure_is_reported_and_a_retry_can_finish_deletion() {
    let (path, directory) = new_path();
    let cipher = TestCipher::default();
    let store = EncryptedSeedFile::new(path.clone(), cipher.clone());
    store.write_seed(&[0x37; 32]).unwrap();
    cipher
        .fail_delete
        .store(true, std::sync::atomic::Ordering::SeqCst);

    assert_eq!(store.delete_seed(), Err(PortError::StorageFailure));
    assert!(!path.exists());
    cipher
        .fail_delete
        .store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(store.delete_seed(), Ok(()));
    cleanup(directory);
}
