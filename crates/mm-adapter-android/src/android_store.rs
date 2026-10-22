use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use mm_ports::PortError;
#[cfg(target_os = "android")]
use mm_ports::{LocalPairingIdentityPort, LocalTlsIdentityPort, PairingIdentityPort};
#[cfg(target_os = "android")]
use zeroize::Zeroize;
use zeroize::Zeroizing;

use crate::identity::CredentialStore;
#[cfg(target_os = "android")]
use crate::identity::IdentityCore;

const RECORD_MAGIC: &[u8; 5] = b"MMID\x01";
const RECORD_HEADER_BYTES: usize = RECORD_MAGIC.len();
const WRAPPED_SEED_BYTES: usize = 12 + 32 + 16;
const RECORD_BYTES: usize = RECORD_HEADER_BYTES + WRAPPED_SEED_BYTES;
#[cfg(target_os = "android")]
const ANDROID_KEY_ALIAS: &str = "org.mirrormesh.identity.seed-wrap.v1";
#[cfg(target_os = "android")]
const ANDROID_KEY_AAD: &[u8] = b"MirrorMesh.AndroidIdentitySeed.v1\0";
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) trait SeedCipher: Send + Sync {
    /// Returns a 12-byte nonce followed by an authenticated 32-byte seed ciphertext.
    fn encrypt_seed(&self, seed: &[u8; 32]) -> Result<Vec<u8>, PortError>;
    fn decrypt_seed(&self, wrapped_seed: &[u8]) -> Result<Zeroizing<Vec<u8>>, PortError>;
    fn delete_wrapping_key(&self) -> Result<(), PortError>;
}

pub(super) struct EncryptedSeedFile<C: SeedCipher> {
    path: PathBuf,
    cipher: C,
}

impl<C: SeedCipher> EncryptedSeedFile<C> {
    pub(super) fn new(path: PathBuf, cipher: C) -> Self {
        Self { path, cipher }
    }

    fn read_record(&self) -> Result<Option<Vec<u8>>, PortError> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(target_os = "android")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
        }

        let file = match options.open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(PortError::StorageFailure),
        };
        let metadata = file.metadata().map_err(|_| PortError::StorageFailure)?;
        if !metadata.is_file() || metadata.len() != RECORD_BYTES as u64 {
            return Err(PortError::StorageFailure);
        }

        let mut record = Vec::with_capacity(RECORD_BYTES);
        file.take(RECORD_BYTES as u64 + 1)
            .read_to_end(&mut record)
            .map_err(|_| PortError::StorageFailure)?;
        if record.len() != RECORD_BYTES || !record.starts_with(RECORD_MAGIC) {
            return Err(PortError::StorageFailure);
        }
        Ok(Some(record))
    }

    fn atomic_write(&self, record: &[u8]) -> Result<(), PortError> {
        let parent = self.path.parent().ok_or(PortError::StorageFailure)?;
        let file_name = self
            .path
            .file_name()
            .ok_or(PortError::StorageFailure)?
            .to_string_lossy();
        let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_path = parent.join(format!(
            ".{file_name}.tmp.{}.{}",
            std::process::id(),
            suffix
        ));

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(target_os = "android")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
        }

        let mut created_temporary_file = false;
        let write_result = (|| {
            let mut file = options
                .open(&temporary_path)
                .map_err(|_| PortError::StorageFailure)?;
            created_temporary_file = true;
            file.write_all(record)
                .map_err(|_| PortError::StorageFailure)?;
            file.sync_all().map_err(|_| PortError::StorageFailure)?;
            fs::rename(&temporary_path, &self.path).map_err(|_| PortError::StorageFailure)?;
            Ok(())
        })();

        if write_result.is_err() && created_temporary_file {
            let _ = fs::remove_file(&temporary_path);
        }
        write_result
    }
}

impl<C: SeedCipher> CredentialStore for EncryptedSeedFile<C> {
    fn read_seed(&self) -> Result<Option<Zeroizing<Vec<u8>>>, PortError> {
        let Some(record) = self.read_record()? else {
            return Ok(None);
        };
        let wrapped_seed = &record[RECORD_HEADER_BYTES..];
        let seed = self.cipher.decrypt_seed(wrapped_seed)?;
        if seed.len() != 32 {
            return Err(PortError::StorageFailure);
        }
        Ok(Some(seed))
    }

    fn write_seed(&self, seed: &[u8; 32]) -> Result<(), PortError> {
        let wrapped_seed = self.cipher.encrypt_seed(seed)?;
        if wrapped_seed.len() != WRAPPED_SEED_BYTES {
            return Err(PortError::StorageFailure);
        }
        let mut record = Vec::with_capacity(RECORD_BYTES);
        record.extend_from_slice(RECORD_MAGIC);
        record.extend_from_slice(&wrapped_seed);
        self.atomic_write(&record)
    }

    fn delete_seed(&self) -> Result<(), PortError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => return Err(PortError::StorageFailure),
        }?;
        self.cipher.delete_wrapping_key()
    }
}

#[cfg(target_os = "android")]
pub struct AndroidPairingIdentity {
    inner: IdentityCore<EncryptedSeedFile<AndroidKeyStoreCipher>>,
}

#[cfg(target_os = "android")]
impl AndroidPairingIdentity {
    /// Opens the app-private identity after the user starts pairing, creating it only if absent.
    pub fn open_or_create() -> Result<Self, PortError> {
        let _identity_lock = identity_lock()?;
        let path = android_seed_path()?;
        let inner =
            IdentityCore::open_or_create(EncryptedSeedFile::new(path, AndroidKeyStoreCipher))?;
        Ok(Self { inner })
    }
}

#[cfg(target_os = "android")]
impl LocalPairingIdentityPort for AndroidPairingIdentity {
    fn local_public_key(&self) -> Result<mm_domain::PeerPublicKey, PortError> {
        let _identity_lock = identity_lock()?;
        self.inner.local_public_key()
    }

    fn sign_pairing_message(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        let _identity_lock = identity_lock()?;
        self.inner.sign_pairing_message(message)
    }

    fn rotate_identity(&self) -> Result<mm_domain::PeerPublicKey, PortError> {
        let _identity_lock = identity_lock()?;
        self.inner.rotate_identity()
    }

    fn delete_identity(&self) -> Result<(), PortError> {
        let _identity_lock = identity_lock()?;
        self.inner.delete_identity()
    }
}

#[cfg(target_os = "android")]
impl LocalTlsIdentityPort for AndroidPairingIdentity {
    fn tls_server_public_key(&self) -> Result<mm_domain::PeerPublicKey, PortError> {
        let _identity_lock = identity_lock()?;
        self.inner.tls_server_public_key()
    }

    fn sign_tls13_certificate_verify(&self, message: &[u8]) -> Result<Vec<u8>, PortError> {
        let _identity_lock = identity_lock()?;
        self.inner.sign_tls13_certificate_verify(message)
    }
}

#[cfg(target_os = "android")]
impl PairingIdentityPort for AndroidPairingIdentity {
    fn fingerprint_public_key(
        &self,
        public_key: &mm_domain::PeerPublicKey,
    ) -> Result<mm_domain::PeerFingerprint, PortError> {
        self.inner.fingerprint_public_key(public_key)
    }

    fn verify_signature(
        &self,
        public_key: &mm_domain::PeerPublicKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), PortError> {
        self.inner.verify_signature(public_key, message, signature)
    }
}

#[cfg(target_os = "android")]
fn identity_lock() -> Result<std::sync::MutexGuard<'static, ()>, PortError> {
    use std::sync::{Mutex, OnceLock};

    static IDENTITY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    IDENTITY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| PortError::StorageFailure)
}

#[cfg(target_os = "android")]
pub(super) fn android_data_directory() -> Result<PathBuf, PortError> {
    use jni::JavaVM;
    use jni::objects::JObject;

    let android = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(android.vm().cast()) }.map_err(|_| PortError::StorageFailure)?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|_| PortError::StorageFailure)?;

    // `JObject` is a non-owning handle; keep the ndk-context global reference intact.
    let raw_context = unsafe { JObject::from_raw(android.context().cast()) };
    let context = env
        .new_local_ref(&raw_context)
        .map_err(|_| PortError::StorageFailure)?;
    let directory = env
        .call_method(&context, "getNoBackupFilesDir", "()Ljava/io/File;", &[])
        .and_then(|value| value.l())
        .map_err(|_| PortError::StorageFailure)?;
    let path = env
        .call_method(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .and_then(|value| value.l())
        .map_err(|_| PortError::StorageFailure)?;
    let path: String = env
        .get_string(&path.into())
        .map_err(|_| PortError::StorageFailure)?
        .into();
    Ok(PathBuf::from(path))
}

#[cfg(target_os = "android")]
fn android_seed_path() -> Result<PathBuf, PortError> {
    Ok(android_data_directory()?.join("mirrormesh-identity-v1.bin"))
}

#[cfg(target_os = "android")]
struct AndroidKeyStoreCipher;

#[cfg(target_os = "android")]
impl SeedCipher for AndroidKeyStoreCipher {
    fn encrypt_seed(&self, seed: &[u8; 32]) -> Result<Vec<u8>, PortError> {
        with_android_env(|env| encrypt_seed(env, seed))
    }

    fn decrypt_seed(&self, wrapped_seed: &[u8]) -> Result<Zeroizing<Vec<u8>>, PortError> {
        with_android_env(|env| decrypt_seed(env, wrapped_seed))
    }

    fn delete_wrapping_key(&self) -> Result<(), PortError> {
        with_android_env(delete_wrapping_key)
    }
}

#[cfg(target_os = "android")]
fn with_android_env<T>(
    operation: impl FnOnce(&mut jni::JNIEnv<'_>) -> Result<T, PortError>,
) -> Result<T, PortError> {
    use jni::JavaVM;

    let android = ndk_context::android_context();
    let vm =
        unsafe { JavaVM::from_raw(android.vm().cast()) }.map_err(|_| PortError::StorageFailure)?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|_| PortError::StorageFailure)?;
    let result = operation(&mut env);
    if result.is_err() && env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    result
}

#[cfg(target_os = "android")]
macro_rules! jni_call {
    ($env:ident, $call:expr) => {
        $call.map_err(|_| PortError::StorageFailure)?
    };
}

#[cfg(target_os = "android")]
fn get_or_create_wrapping_key<'local>(
    env: &mut jni::JNIEnv<'local>,
    create_if_missing: bool,
) -> Result<jni::objects::JObject<'local>, PortError> {
    use jni::objects::{JObject, JValue};

    let key_store_type = jni_call!(env, env.new_string("AndroidKeyStore"));
    let key_store_type = JObject::from(key_store_type);
    let key_store = jni_call!(
        env,
        env.call_static_method(
            "java/security/KeyStore",
            "getInstance",
            "(Ljava/lang/String;)Ljava/security/KeyStore;",
            &[JValue::Object(&key_store_type)],
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;
    let no_parameter = JObject::null();
    jni_call!(
        env,
        env.call_method(
            &key_store,
            "load",
            "(Ljava/security/KeyStore$LoadStoreParameter;)V",
            &[JValue::Object(&no_parameter)],
        )
    );

    let alias = jni_call!(env, env.new_string(ANDROID_KEY_ALIAS));
    let alias = JObject::from(alias);
    let no_password = JObject::null();
    let key = jni_call!(
        env,
        env.call_method(
            &key_store,
            "getKey",
            "(Ljava/lang/String;[C)Ljava/security/Key;",
            &[JValue::Object(&alias), JValue::Object(&no_password)],
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;
    if !key.is_null() {
        return Ok(key);
    }
    if !create_if_missing {
        return Err(PortError::StorageFailure);
    }

    let algorithm = jni_call!(env, env.new_string("AES"));
    let algorithm = JObject::from(algorithm);
    let provider = jni_call!(env, env.new_string("AndroidKeyStore"));
    let provider = JObject::from(provider);
    let generator = jni_call!(
        env,
        env.call_static_method(
            "javax/crypto/KeyGenerator",
            "getInstance",
            "(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/KeyGenerator;",
            &[JValue::Object(&algorithm), JValue::Object(&provider)],
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;

    let builder = jni_call!(
        env,
        env.new_object(
            "android/security/keystore/KeyGenParameterSpec$Builder",
            "(Ljava/lang/String;I)V",
            &[JValue::Object(&alias), JValue::Int(3)],
        )
    );
    jni_call!(
        env,
        env.call_method(
            &builder,
            "setKeySize",
            "(I)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
            &[JValue::Int(256)],
        )
    );
    let gcm = jni_call!(env, env.new_string("GCM"));
    let gcm = JObject::from(gcm);
    let block_modes = jni_call!(
        env,
        env.new_object_array(1, "java/lang/String", JObject::null())
    );
    jni_call!(env, env.set_object_array_element(&block_modes, 0, gcm));
    jni_call!(
        env,
        env.call_method(
            &builder,
            "setBlockModes",
            "([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
            &[JValue::Object(block_modes.as_ref())],
        )
    );

    let no_padding = jni_call!(env, env.new_string("NoPadding"));
    let no_padding = JObject::from(no_padding);
    let paddings = jni_call!(
        env,
        env.new_object_array(1, "java/lang/String", JObject::null())
    );
    jni_call!(env, env.set_object_array_element(&paddings, 0, no_padding));
    jni_call!(
        env,
        env.call_method(
            &builder,
            "setEncryptionPaddings",
            "([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
            &[JValue::Object(paddings.as_ref())],
        )
    );
    let key_spec = jni_call!(
        env,
        env.call_method(
            &builder,
            "build",
            "()Landroid/security/keystore/KeyGenParameterSpec;",
            &[]
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;
    jni_call!(
        env,
        env.call_method(
            &generator,
            "init",
            "(Ljava/security/spec/AlgorithmParameterSpec;)V",
            &[JValue::Object(&key_spec)],
        )
    );
    jni_call!(
        env,
        env.call_method(&generator, "generateKey", "()Ljavax/crypto/SecretKey;", &[])
    )
    .l()
    .map_err(|_| PortError::StorageFailure)
}

#[cfg(target_os = "android")]
fn delete_wrapping_key(env: &mut jni::JNIEnv<'_>) -> Result<(), PortError> {
    use jni::objects::{JObject, JValue};

    let key_store_type = jni_call!(env, env.new_string("AndroidKeyStore"));
    let key_store_type = JObject::from(key_store_type);
    let key_store = jni_call!(
        env,
        env.call_static_method(
            "java/security/KeyStore",
            "getInstance",
            "(Ljava/lang/String;)Ljava/security/KeyStore;",
            &[JValue::Object(&key_store_type)],
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;
    let no_parameter = JObject::null();
    jni_call!(
        env,
        env.call_method(
            &key_store,
            "load",
            "(Ljava/security/KeyStore$LoadStoreParameter;)V",
            &[JValue::Object(&no_parameter)],
        )
    );
    let alias = jni_call!(env, env.new_string(ANDROID_KEY_ALIAS));
    let alias = JObject::from(alias);
    jni_call!(
        env,
        env.call_method(
            &key_store,
            "deleteEntry",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&alias)],
        )
    );
    Ok(())
}

#[cfg(target_os = "android")]
fn encrypt_seed(env: &mut jni::JNIEnv<'_>, seed: &[u8; 32]) -> Result<Vec<u8>, PortError> {
    use jni::objects::{JByteArray, JObject, JValue};

    let key = get_or_create_wrapping_key(env, true)?;
    let transformation = jni_call!(env, env.new_string("AES/GCM/NoPadding"));
    let transformation = JObject::from(transformation);
    let cipher = jni_call!(
        env,
        env.call_static_method(
            "javax/crypto/Cipher",
            "getInstance",
            "(Ljava/lang/String;)Ljavax/crypto/Cipher;",
            &[JValue::Object(&transformation)],
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;
    jni_call!(
        env,
        env.call_method(
            &cipher,
            "init",
            "(ILjava/security/Key;)V",
            &[JValue::Int(1), JValue::Object(&key)]
        )
    );
    let aad = jni_call!(env, env.byte_array_from_slice(ANDROID_KEY_AAD));
    jni_call!(
        env,
        env.call_method(
            &cipher,
            "updateAAD",
            "([B)V",
            &[JValue::Object(aad.as_ref())]
        )
    );
    let input = jni_call!(env, env.byte_array_from_slice(seed));
    let encrypted = env.call_method(
        &cipher,
        "doFinal",
        "([B)[B",
        &[JValue::Object(input.as_ref())],
    );
    if encrypted.is_err() && env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    env.set_byte_array_region(&input, 0, &[0i8; 32])
        .map_err(|_| PortError::StorageFailure)?;
    let encrypted = encrypted
        .map_err(|_| PortError::StorageFailure)?
        .l()
        .map_err(|_| PortError::StorageFailure)?;
    let encrypted = JByteArray::from(encrypted);
    let mut result = jni_call!(env, env.convert_byte_array(&encrypted));
    let iv = jni_call!(env, env.call_method(&cipher, "getIV", "()[B", &[]))
        .l()
        .map_err(|_| PortError::StorageFailure)?;
    let iv = JByteArray::from(iv);
    let iv = jni_call!(env, env.convert_byte_array(&iv));
    if iv.len() != 12 || result.len() != 48 {
        result.zeroize();
        return Err(PortError::StorageFailure);
    }
    let mut wrapped = Vec::with_capacity(WRAPPED_SEED_BYTES);
    wrapped.extend_from_slice(&iv);
    wrapped.append(&mut result);
    Ok(wrapped)
}

#[cfg(target_os = "android")]
fn decrypt_seed(
    env: &mut jni::JNIEnv<'_>,
    wrapped_seed: &[u8],
) -> Result<Zeroizing<Vec<u8>>, PortError> {
    use jni::objects::{JByteArray, JObject, JValue};

    if wrapped_seed.len() != WRAPPED_SEED_BYTES {
        return Err(PortError::StorageFailure);
    }
    let key = get_or_create_wrapping_key(env, false)?;
    let transformation = jni_call!(env, env.new_string("AES/GCM/NoPadding"));
    let transformation = JObject::from(transformation);
    let cipher = jni_call!(
        env,
        env.call_static_method(
            "javax/crypto/Cipher",
            "getInstance",
            "(Ljava/lang/String;)Ljavax/crypto/Cipher;",
            &[JValue::Object(&transformation)],
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;
    let iv = jni_call!(env, env.byte_array_from_slice(&wrapped_seed[..12]));
    let gcm_parameters = jni_call!(
        env,
        env.new_object(
            "javax/crypto/spec/GCMParameterSpec",
            "(I[B)V",
            &[JValue::Int(128), JValue::Object(iv.as_ref())],
        )
    );
    jni_call!(
        env,
        env.call_method(
            &cipher,
            "init",
            "(ILjava/security/Key;Ljava/security/spec/AlgorithmParameterSpec;)V",
            &[
                JValue::Int(2),
                JValue::Object(&key),
                JValue::Object(&gcm_parameters),
            ],
        )
    );
    let aad = jni_call!(env, env.byte_array_from_slice(ANDROID_KEY_AAD));
    jni_call!(
        env,
        env.call_method(
            &cipher,
            "updateAAD",
            "([B)V",
            &[JValue::Object(aad.as_ref())]
        )
    );
    let ciphertext = jni_call!(env, env.byte_array_from_slice(&wrapped_seed[12..]));
    let plaintext = jni_call!(
        env,
        env.call_method(
            &cipher,
            "doFinal",
            "([B)[B",
            &[JValue::Object(ciphertext.as_ref())]
        )
    )
    .l()
    .map_err(|_| PortError::StorageFailure)?;
    let plaintext = JByteArray::from(plaintext);
    let result = env.convert_byte_array(&plaintext).map(Zeroizing::new);
    if result.is_err() && env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    env.set_byte_array_region(&plaintext, 0, &[0i8; 32])
        .map_err(|_| PortError::StorageFailure)?;
    let result = result.map_err(|_| PortError::StorageFailure)?;
    if result.len() != 32 {
        return Err(PortError::StorageFailure);
    }
    Ok(result)
}
