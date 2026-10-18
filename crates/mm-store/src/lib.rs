use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mm_ports::{PortError, PreferencesPort};

mod sqlite_trusted_peers;
pub use sqlite_trusted_peers::{MAX_TRUSTED_PEERS, SqliteTrustedPeers};

const FILE_HEADER: &str = "mirrormesh-preferences-v1\n";
const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_KEY_BYTES: usize = 128;
const MAX_VALUE_BYTES: usize = 8 * 1024;

#[derive(Debug)]
pub enum StoreError {
    InvalidKey,
    InvalidValue,
    MalformedFile,
    FileTooLarge,
    Sqlite(rusqlite::Error),
    UnsupportedSchema(i64),
    Io(std::io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => formatter.write_str("preference key is invalid"),
            Self::InvalidValue => formatter.write_str("preference value exceeds its allowed size"),
            Self::MalformedFile => formatter.write_str("preferences file is malformed"),
            Self::FileTooLarge => formatter.write_str("preferences file exceeds its allowed size"),
            Self::Sqlite(error) => write!(formatter, "SQLite storage failed: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported SQLite schema version: {version}")
            }
            Self::Io(error) => write!(formatter, "preferences I/O failed: {error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

pub struct FilePreferences {
    path: PathBuf,
    values: Mutex<BTreeMap<String, String>>,
}

impl FilePreferences {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let path = path.into();
        let values = if path.exists() {
            let metadata = fs::metadata(&path)?;
            if metadata.len() as usize > MAX_FILE_BYTES {
                return Err(StoreError::FileTooLarge);
            }
            let mut contents = Vec::with_capacity(metadata.len() as usize);
            OpenOptions::new()
                .read(true)
                .open(&path)?
                .take((MAX_FILE_BYTES + 1) as u64)
                .read_to_end(&mut contents)?;
            if contents.len() > MAX_FILE_BYTES {
                return Err(StoreError::FileTooLarge);
            }
            decode_file(&contents)?
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            path,
            values: Mutex::new(values),
        })
    }

    fn persist(&self, values: &BTreeMap<String, String>) -> Result<(), StoreError> {
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        if let Some(parent) = parent {
            fs::create_dir_all(parent)?;
        }
        let temporary = temporary_path(&self.path);
        let encoded = encode_file(values);
        if encoded.len() > MAX_FILE_BYTES {
            return Err(StoreError::FileTooLarge);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        if let Err(error) = file.write_all(&encoded).and_then(|()| file.sync_all()) {
            let _ = fs::remove_file(&temporary);
            return Err(StoreError::Io(error));
        }
        drop(file);
        if let Err(error) = fs::rename(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(StoreError::Io(error));
        }
        Ok(())
    }
}

impl PreferencesPort for FilePreferences {
    fn get(&self, key: &str) -> Result<Option<String>, PortError> {
        validate_key(key).map_err(|_| PortError::InvalidConfiguration)?;
        self.values
            .lock()
            .map_err(|_| PortError::StorageFailure)
            .map(|values| values.get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), PortError> {
        validate_key(key).map_err(|_| PortError::InvalidConfiguration)?;
        if value.len() > MAX_VALUE_BYTES {
            return Err(PortError::InvalidConfiguration);
        }
        let mut values = self.values.lock().map_err(|_| PortError::StorageFailure)?;
        let mut updated = values.clone();
        updated.insert(key.to_owned(), value.to_owned());
        self.persist(&updated)
            .map_err(|_| PortError::StorageFailure)?;
        *values = updated;
        Ok(())
    }

    fn remove(&self, key: &str) -> Result<(), PortError> {
        validate_key(key).map_err(|_| PortError::InvalidConfiguration)?;
        let mut values = self.values.lock().map_err(|_| PortError::StorageFailure)?;
        if !values.contains_key(key) {
            return Ok(());
        }
        let mut updated = values.clone();
        updated.remove(key);
        self.persist(&updated)
            .map_err(|_| PortError::StorageFailure)?;
        *values = updated;
        Ok(())
    }
}

fn validate_key(key: &str) -> Result<(), StoreError> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES || key.chars().any(char::is_control) {
        return Err(StoreError::InvalidKey);
    }
    Ok(())
}

fn encode_file(values: &BTreeMap<String, String>) -> Vec<u8> {
    let mut output = FILE_HEADER.as_bytes().to_vec();
    for (key, value) in values {
        output.extend_from_slice(hex_encode(key.as_bytes()).as_bytes());
        output.push(b'=');
        output.extend_from_slice(hex_encode(value.as_bytes()).as_bytes());
        output.push(b'\n');
    }
    output
}

fn decode_file(contents: &[u8]) -> Result<BTreeMap<String, String>, StoreError> {
    if contents.len() > MAX_FILE_BYTES || !contents.starts_with(FILE_HEADER.as_bytes()) {
        return Err(if contents.len() > MAX_FILE_BYTES {
            StoreError::FileTooLarge
        } else {
            StoreError::MalformedFile
        });
    }
    let text = std::str::from_utf8(&contents[FILE_HEADER.len()..])
        .map_err(|_| StoreError::MalformedFile)?;
    let mut values = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let (encoded_key, encoded_value) = line.split_once('=').ok_or(StoreError::MalformedFile)?;
        let key =
            String::from_utf8(hex_decode(encoded_key)?).map_err(|_| StoreError::MalformedFile)?;
        let value =
            String::from_utf8(hex_decode(encoded_value)?).map_err(|_| StoreError::MalformedFile)?;
        validate_key(&key)?;
        if value.len() > MAX_VALUE_BYTES || values.insert(key, value).is_some() {
            return Err(StoreError::MalformedFile);
        }
    }
    Ok(values)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(value: &str) -> Result<Vec<u8>, StoreError> {
    let mut output = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks(2) {
        if pair.len() != 2 {
            return Err(StoreError::MalformedFile);
        }
        let high = from_hex(pair[0]).ok_or(StoreError::MalformedFile)?;
        let low = from_hex(pair[1]).ok_or(StoreError::MalformedFile)?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".tmp-{}", std::process::id()));
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::{FilePreferences, StoreError, validate_key};
    use mm_ports::PreferencesPort;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "mirrormesh-preferences-{}-{unique}",
            std::process::id()
        ))
    }

    #[test]
    fn persists_values_and_removes_keys() {
        let file = path();
        let preferences = FilePreferences::open(&file).unwrap();
        preferences.set("quality", "720p30").unwrap();
        drop(preferences);

        let reopened = FilePreferences::open(&file).unwrap();
        assert_eq!(reopened.get("quality").unwrap().as_deref(), Some("720p30"));
        reopened.remove("quality").unwrap();
        assert_eq!(
            FilePreferences::open(&file)
                .unwrap()
                .get("quality")
                .unwrap(),
            None
        );
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn rejects_corrupt_file_and_invalid_keys() {
        let file = path();
        std::fs::write(&file, b"broken\n").unwrap();
        assert!(matches!(
            FilePreferences::open(&file),
            Err(StoreError::MalformedFile)
        ));
        assert!(matches!(validate_key(""), Err(StoreError::InvalidKey)));
        assert!(matches!(
            validate_key("key\nname"),
            Err(StoreError::InvalidKey)
        ));
        let _ = std::fs::remove_file(file);
    }
}
