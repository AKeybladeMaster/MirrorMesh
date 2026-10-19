use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use mm_domain::{DeviceId, PeerFingerprint};
use mm_ports::PortError;

const APP_DATA_DIRECTORY: &str = "mirrormesh";
const TRUSTED_PEERS_DATABASE: &str = "trusted-peers-v1.sqlite";

pub(super) fn data_home_directory(
    xdg_data_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, PortError> {
    if let Some(path) = xdg_data_home.filter(|path| path.is_absolute()) {
        return Ok(path.to_path_buf());
    }

    home.filter(|path| path.is_absolute())
        .map(|path| path.join(".local").join("share"))
        .ok_or(PortError::InvalidConfiguration)
}

pub(super) fn trusted_peer_database_path(data_home: &Path) -> PathBuf {
    data_home
        .join(APP_DATA_DIRECTORY)
        .join(TRUSTED_PEERS_DATABASE)
}

pub(super) fn device_id_from_fingerprint(fingerprint: PeerFingerprint) -> DeviceId {
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&fingerprint.as_bytes()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    DeviceId::from_bytes(bytes)
}

pub(super) fn initialize_once<R>(
    slot: &OnceLock<Mutex<Option<(DeviceId, R)>>>,
    initialize: impl FnOnce() -> Result<(DeviceId, R), PortError>,
) -> Result<DeviceId, PortError> {
    let mut runtime_slot = slot
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| PortError::StorageFailure)?;
    if let Some((device_id, _)) = runtime_slot.as_ref() {
        return Ok(*device_id);
    }

    let (device_id, runtime) = initialize()?;
    *runtime_slot = Some((device_id, runtime));
    Ok(device_id)
}

#[cfg(target_os = "linux")]
mod linux_runtime {
    use std::env;
    use std::fs::{self, DirBuilder, OpenOptions};
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, OnceLock};

    use mm_domain::DeviceId;
    use mm_ports::{LocalPairingIdentityPort, PairingIdentityPort, PortError};
    use mm_runtime::PairingRuntime;
    use mm_store::SqliteTrustedPeers;

    use crate::LinuxPairingIdentity;

    use super::{
        data_home_directory, device_id_from_fingerprint, initialize_once,
        trusted_peer_database_path,
    };

    type LinuxPairingRuntime = PairingRuntime<SqliteTrustedPeers, LinuxPairingIdentity>;

    static LINUX_PAIRING_RUNTIME: OnceLock<Mutex<Option<(DeviceId, LinuxPairingRuntime)>>> =
        OnceLock::new();

    /// Opens Linux identity and trust storage after the user explicitly starts pairing.
    pub(super) fn initialize_pairing_runtime() -> Result<DeviceId, PortError> {
        initialize_once(&LINUX_PAIRING_RUNTIME, || {
            let data_home = data_home_directory(
                env::var_os("XDG_DATA_HOME").as_deref().map(Path::new),
                env::var_os("HOME").as_deref().map(Path::new),
            )?;
            let database_path = trusted_peer_database_path(&data_home);
            let database_directory = database_path
                .parent()
                .ok_or(PortError::InvalidConfiguration)?;

            ensure_private_directory(database_directory)?;
            ensure_private_database_file(&database_path)?;

            let trusted_peers = Arc::new(
                SqliteTrustedPeers::open(&database_path).map_err(|_| PortError::StorageFailure)?,
            );
            let identity = Arc::new(LinuxPairingIdentity::open_or_create()?);
            let public_key = identity.local_public_key()?;
            let fingerprint = identity.fingerprint_public_key(&public_key)?;
            let device_id = device_id_from_fingerprint(fingerprint);
            let runtime = PairingRuntime::new(device_id, identity, trusted_peers)?;
            Ok((device_id, runtime))
        })
    }

    fn ensure_private_directory(path: &Path) -> Result<(), PortError> {
        match DirBuilder::new().recursive(true).mode(0o700).create(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(PortError::StorageFailure),
        }

        let metadata = fs::symlink_metadata(path).map_err(|_| PortError::StorageFailure)?;
        let effective_uid = unsafe { libc::geteuid() };
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.uid() != effective_uid
        {
            return Err(PortError::StorageFailure);
        }
        if metadata.mode() & 0o077 != 0 {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|_| PortError::StorageFailure)?;
        }
        Ok(())
    }

    fn ensure_private_database_file(path: &Path) -> Result<(), PortError> {
        let effective_uid = unsafe { libc::geteuid() };
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || !metadata.is_file()
                    || metadata.uid() != effective_uid
                    || metadata.nlink() != 1
                {
                    return Err(PortError::StorageFailure);
                }
                if metadata.mode() & 0o077 != 0 {
                    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                        .map_err(|_| PortError::StorageFailure)?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                    .open(path);
                match file {
                    Ok(file) => drop(file),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        return ensure_private_database_file(path);
                    }
                    Err(_) => return Err(PortError::StorageFailure),
                }
            }
            Err(_) => return Err(PortError::StorageFailure),
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::{ensure_private_database_file, ensure_private_directory};
        use std::fs;

        #[test]
        fn runtime_storage_directory_is_created_with_private_permissions() {
            let root = std::env::temp_dir().join(format!(
                "mirrormesh-linux-runtime-{}-{}",
                std::process::id(),
                std::thread::current().name().unwrap_or("test")
            ));
            let app_data = root.join("mirrormesh");

            ensure_private_directory(&app_data).unwrap();
            let metadata = fs::symlink_metadata(&app_data).unwrap();

            assert!(metadata.is_dir());
            use std::os::unix::fs::MetadataExt;
            assert_eq!(metadata.mode() & 0o777, 0o700);

            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn runtime_database_file_is_created_with_private_permissions() {
            let root = std::env::temp_dir().join(format!(
                "mirrormesh-linux-db-{}-{}",
                std::process::id(),
                std::thread::current().name().unwrap_or("test")
            ));
            fs::create_dir_all(&root).unwrap();
            let database = root.join("trusted-peers-v1.sqlite");

            ensure_private_database_file(&database).unwrap();
            let metadata = fs::symlink_metadata(&database).unwrap();

            assert!(metadata.is_file());
            use std::os::unix::fs::MetadataExt;
            assert_eq!(metadata.mode() & 0o777, 0o600);

            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux_runtime::initialize_pairing_runtime;

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    use mm_domain::{DeviceId, PeerFingerprint};

    use super::{
        data_home_directory, device_id_from_fingerprint, initialize_once,
        trusted_peer_database_path,
    };

    #[test]
    fn device_id_is_stably_derived_as_uuid_v8_from_the_local_fingerprint() {
        let fingerprint = PeerFingerprint::from_bytes([
            0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98, 0xa9, 0xba, 0xcb, 0xdc, 0xed,
            0xfe, 0x0f, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
            0xdd, 0xee, 0xff, 0x00,
        ]);
        let expected = DeviceId::from_bytes([
            0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x86, 0x87, 0x98, 0xa9, 0xba, 0xcb, 0xdc, 0xed,
            0xfe, 0x0f,
        ]);

        assert_eq!(device_id_from_fingerprint(fingerprint), expected);
        assert_eq!(expected.to_string(), "10213243-5465-8687-98a9-bacbdcedfe0f");
    }

    fn test_absolute_paths() -> (PathBuf, PathBuf) {
        #[cfg(windows)]
        {
            (
                PathBuf::from(r"C:\Users\MirrorMesh\.local\share"),
                PathBuf::from(r"C:\Users\MirrorMesh"),
            )
        }
        #[cfg(not(windows))]
        {
            (
                PathBuf::from("/home/MirrorMesh/.local/share"),
                PathBuf::from("/home/MirrorMesh"),
            )
        }
    }

    #[test]
    fn absolute_xdg_data_home_is_used_for_the_application_data_directory() {
        let (xdg_data_home, home) = test_absolute_paths();

        assert_eq!(
            data_home_directory(Some(&xdg_data_home), Some(&home)).unwrap(),
            xdg_data_home
        );
    }

    #[test]
    fn relative_xdg_data_home_falls_back_to_the_home_data_directory() {
        let relative_xdg = Path::new("relative-data");
        let (_, home) = test_absolute_paths();

        assert_eq!(
            data_home_directory(Some(relative_xdg), Some(&home)).unwrap(),
            home.join(".local").join("share")
        );
    }

    #[test]
    fn absent_or_relative_home_fails_when_no_absolute_xdg_directory_is_available() {
        assert!(data_home_directory(None, None).is_err());
        assert!(data_home_directory(None, Some(Path::new("relative-home"))).is_err());
    }

    #[test]
    fn trusted_peer_database_is_scoped_under_mirrormesh_application_data() {
        let (base, _) = test_absolute_paths();

        assert_eq!(
            trusted_peer_database_path(&base),
            base.join("mirrormesh").join("trusted-peers-v1.sqlite")
        );
    }

    #[test]
    fn process_initializer_retains_one_runtime_and_returns_the_same_id() {
        static RUNTIME: OnceLock<Mutex<Option<(DeviceId, u8)>>> = OnceLock::new();
        let calls = AtomicUsize::new(0);
        let expected_id = DeviceId::from_bytes([7; 16]);

        let first = initialize_once(&RUNTIME, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok((expected_id, 1))
        })
        .unwrap();
        let second = initialize_once(&RUNTIME, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok((DeviceId::from_bytes([8; 16]), 2))
        })
        .unwrap();

        assert_eq!(first, expected_id);
        assert_eq!(second, expected_id);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_initialization_can_be_retried_without_retaining_partial_state() {
        static RUNTIME: OnceLock<Mutex<Option<(DeviceId, u8)>>> = OnceLock::new();
        let expected_id = DeviceId::from_bytes([9; 16]);

        assert!(initialize_once(&RUNTIME, || Err(mm_ports::PortError::Unavailable)).is_err());
        assert_eq!(
            initialize_once(&RUNTIME, || Ok((expected_id, 1))).unwrap(),
            expected_id
        );
    }
}
