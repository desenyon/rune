use crate::blobs::BlobStore;
use crate::cache::ContentCache;
use crate::error::{Result, StorageError};
use crate::fts;
use crate::migrations::{bundled_migrations, migrate};
use crate::settings::SettingsStore;
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
    blobs: Arc<BlobStore>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        )?;
        configure(&conn)?;
        migrate(&mut conn, &bundled_migrations())?;
        let blob_root = path
            .parent()
            .map(|parent| parent.join("blobs"))
            .unwrap_or_else(|| PathBuf::from("blobs"));
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
            blobs: Arc::new(BlobStore::open(blob_root)?),
        })
    }

    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn)?;
        migrate(&mut conn, &bundled_migrations())?;
        let blobs = BlobStore::open_temp()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
            blobs: Arc::new(blobs),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.conn.lock();
        f(&conn)
    }

    pub fn with_conn_mut<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut conn = self.conn.lock();
        f(&mut conn)
    }

    pub fn blobs(&self) -> &BlobStore {
        self.blobs.as_ref()
    }

    pub fn search_text(&self, query: &str, limit: usize) -> Result<Vec<(String, String, f64)>> {
        self.with_conn(|conn| fts::search(conn, query, limit))
    }

    pub fn cache(&self) -> ContentCache<'_> {
        ContentCache {
            store: self,
        }
    }

    pub fn settings(&self) -> SettingsStore<'_> {
        SettingsStore { store: self }
    }

    pub fn backup_to(&self, dest: impl AsRef<Path>) -> Result<()> {
        let dest = dest.as_ref();
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.with_conn(|conn| {
            conn.backup(rusqlite::DatabaseName::Main, dest, None)
                .map_err(StorageError::from)
        })
    }
}

pub fn open(path: impl AsRef<Path>) -> Result<Store> {
    Store::open(path)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
        PRAGMA temp_store = MEMORY;
        PRAGMA recursive_triggers = ON;
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::{Node, NodeKind};

    #[test]
    fn open_memory_and_insert_node() {
        let store = Store::open_in_memory().unwrap();
        let node = Node::new(NodeKind::Repository, Some("demo".into()), serde_json::json!({"root": "/tmp/demo"}));
        store.upsert_node(&node).unwrap();
        let loaded = store.get_node(node.id).unwrap();
        assert_eq!(loaded.name.as_deref(), Some("demo"));
    }
}
