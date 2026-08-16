use crate::error::{Result, StorageError};
use rune_core::{ContentHash, Timestamp};
use rusqlite::params;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct BlobStore {
    root: PathBuf,
    ephemeral: bool,
}

#[derive(Clone, Debug)]
pub struct BlobMeta {
    pub hash: ContentHash,
    pub size: u64,
    pub media_type: Option<String>,
    pub created_at: Timestamp,
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl BlobStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            ephemeral: false,
        })
    }

    pub fn open_temp() -> Result<Self> {
        let token = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rune-blobs-{}-{}-{}",
            std::process::id(),
            Timestamp::now().as_millis(),
            token
        ));
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            ephemeral: true,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn put(&self, bytes: &[u8], media_type: Option<&str>) -> Result<BlobMeta> {
        let hash = ContentHash::hash(bytes);
        let path = self.path_for(hash);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            let mut file = fs::File::create(&path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        Ok(BlobMeta {
            hash,
            size: bytes.len() as u64,
            media_type: media_type.map(str::to_string),
            created_at: Timestamp::now(),
        })
    }

    pub fn get(&self, hash: ContentHash) -> Result<Vec<u8>> {
        let path = self.path_for(hash);
        fs::read(&path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                StorageError::MissingBlob(hash.to_hex())
            } else {
                StorageError::Io(err)
            }
        })
    }

    pub fn exists(&self, hash: ContentHash) -> bool {
        self.path_for(hash).exists()
    }

    pub fn path_for(&self, hash: ContentHash) -> PathBuf {
        let hex = hash.to_hex();
        self.root.join(&hex[..2]).join(&hex)
    }
}

impl Drop for BlobStore {
    fn drop(&mut self) {
        if self.ephemeral {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

impl crate::db::Store {
    pub fn record_blob(&self, meta: &BlobMeta) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO blobs (hash, size, media_type, created_at, last_accessed_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(hash) DO UPDATE SET last_accessed_at = excluded.last_accessed_at",
                params![
                    meta.hash.to_hex(),
                    meta.size as i64,
                    meta.media_type,
                    meta.created_at.as_millis()
                ],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_is_content_addressed() {
        let store = BlobStore::open_temp().unwrap();
        let a = store.put(b"hello rune", Some("text/plain")).unwrap();
        let b = store.put(b"hello rune", None).unwrap();
        assert_eq!(a.hash, b.hash);
        assert_eq!(store.get(a.hash).unwrap(), b"hello rune");
    }
}
