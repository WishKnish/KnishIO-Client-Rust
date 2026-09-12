//! Atomic file-based key-value persistence backend for secret storage

use super::StorageBackend;
use crate::error::{KnishIOError, Result};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// File-backed storage backend storing key-value pairs atomically as JSON with restrictive file permissions (0600 on Unix)
pub struct FileStorageBackend {
    path: PathBuf,
    store: RwLock<BTreeMap<String, String>>,
}

impl FileStorageBackend {
    /// Create or open a file-backed storage backend at `path`
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| KnishIOError::SecretStorage(format!("file storage: {e}")))?;
            }
        }

        let map: BTreeMap<String, String> = if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| KnishIOError::SecretStorage(format!("file storage: {e}")))?;
            serde_json::from_str(&content).map_err(|_| {
                KnishIOError::SecretStorage(format!(
                    "file storage: corrupted store at {}",
                    path.display()
                ))
            })?
        } else {
            BTreeMap::new()
        };

        Ok(Self {
            path,
            store: RwLock::new(map),
        })
    }

    /// Path to the storage file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Atomically persist current map to disk via tempfile + rename with 0o600 permissions
    fn persist(&self, map: &BTreeMap<String, String>) -> Result<()> {
        let tmp_path = self.path.with_extension(format!(
            "tmp.{}",
            uuid::Uuid::new_v4().simple()
        ));

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);

        #[cfg(unix)]
        options.mode(0o600);

        let mut file = options
            .open(&tmp_path)
            .map_err(|e| KnishIOError::SecretStorage(format!("file storage: {e}")))?;

        let json = serde_json::to_string_pretty(map)
            .map_err(|e| KnishIOError::SecretStorage(format!("file storage: {e}")))?;

        file.write_all(json.as_bytes())
            .map_err(|e| KnishIOError::SecretStorage(format!("file storage: {e}")))?;

        file.sync_all()
            .map_err(|e| KnishIOError::SecretStorage(format!("file storage: {e}")))?;

        drop(file);

        fs::rename(&tmp_path, &self.path)
            .map_err(|e| KnishIOError::SecretStorage(format!("file storage: {e}")))?;

        Ok(())
    }
}

impl StorageBackend for FileStorageBackend {
    fn get_item(&self, key: &str) -> Result<Option<String>> {
        let store = self
            .store
            .read()
            .map_err(|_| KnishIOError::SecretStorage("storage backend lock poisoned".into()))?;
        Ok(store.get(key).cloned())
    }

    fn set_item(&self, key: &str, value: String) -> Result<()> {
        let mut store = self
            .store
            .write()
            .map_err(|_| KnishIOError::SecretStorage("storage backend lock poisoned".into()))?;
        store.insert(key.to_string(), value);
        self.persist(&store)
    }

    fn remove_item(&self, key: &str) -> Result<bool> {
        let mut store = self
            .store
            .write()
            .map_err(|_| KnishIOError::SecretStorage("storage backend lock poisoned".into()))?;
        let removed = store.remove(key).is_some();
        if removed {
            self.persist(&store)?;
        }
        Ok(removed)
    }

    fn keys(&self) -> Result<Vec<String>> {
        let store = self
            .store
            .read()
            .map_err(|_| KnishIOError::SecretStorage("storage backend lock poisoned".into()))?;
        Ok(store.keys().cloned().collect())
    }
}
