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
    static DATABASE_PATH: OnceLock<PathBuf> = OnceLock::new();

    /// Opens Linux identity and trust storage after the user explicitly starts pairing.
    pub fn initialize_pairing_runtime() -> Result<DeviceId, PortError> {
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
            let _ = DATABASE_PATH.set(database_path);
            Ok((device_id, runtime))
        })
    }

    pub fn create_pairing_service(
        address: std::net::SocketAddr,
    ) -> Result<
        mm_adapter_signaling::native_service::NativePairingService<
            SqliteTrustedPeers,
            LinuxPairingIdentity,
        >,
        mm_adapter_signaling::native_service::NativePairingServiceError,
    > {
        use mm_adapter_signaling::native_service::{
            NativePairingService, NativePairingServiceError,
        };
        let id = initialize_pairing_runtime().map_err(NativePairingServiceError::Platform)?;
        let path = DATABASE_PATH
            .get()
            .ok_or(NativePairingServiceError::Platform(PortError::Unavailable))?;
        let identity = Arc::new(
            LinuxPairingIdentity::open_or_create().map_err(NativePairingServiceError::Platform)?,
        );
        let peers = Arc::new(
            SqliteTrustedPeers::open(path)
                .map_err(|_| NativePairingServiceError::Platform(PortError::StorageFailure))?,
        );
        let runtime = PairingRuntime::new(id, Arc::clone(&identity), peers)
            .map_err(NativePairingServiceError::Platform)?;
        NativePairingService::bind(address, runtime, identity)
    }

    pub fn revoke_trusted_peer(id: DeviceId, now: u64) -> Result<(), PortError> {
        use mm_ports::TrustedPeerPort;
        let path = DATABASE_PATH.get().ok_or(PortError::Unavailable)?;
        SqliteTrustedPeers::open(path)
            .map_err(|_| PortError::StorageFailure)?
            .revoke(id, now)
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
}

#[cfg(target_os = "linux")]
pub use linux_runtime::{create_pairing_service, initialize_pairing_runtime, revoke_trusted_peer};
