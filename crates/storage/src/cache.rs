use crate::db::Store;
use crate::error::{Result, StorageError};
use rune_core::{ContentHash, Fingerprint, Timestamp};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

pub struct ContentCache<'a> {
    pub store: &'a Store,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub fingerprint: Fingerprint,
    pub blob_hash: ContentHash,
    pub kind: String,
    pub created_at: Timestamp,
    pub expires_at: Option<Timestamp>,
}

impl ContentCache<'_> {
    pub fn put(&self, entry: &CacheEntry, bytes: &[u8]) -> Result<()> {
        let current = Fingerprint::of(&entry.kind, &[&serde_json::to_vec(&entry.fingerprint.inputs)?]);
        if current.hash != entry.fingerprint.hash && !entry.fingerprint.inputs.is_empty() {
            // Store the caller-supplied fingerprint; input change invalidates on get.
        }
        let meta = self.store.blobs().put(bytes, Some("application/octet-stream"))?;
        self.store.record_blob(&meta)?;
        self.store.with_conn(|conn| {
            conn.execute(
                "INSERT INTO cache_entries (cache_key, fingerprint, blob_hash, kind, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(cache_key) DO UPDATE SET
                    fingerprint = excluded.fingerprint,
                    blob_hash = excluded.blob_hash,
                    kind = excluded.kind,
                    created_at = excluded.created_at,
                    expires_at = excluded.expires_at",
                params![
                    entry.key,
                    serde_json::to_string(&entry.fingerprint)?,
                    meta.hash.to_hex(),
                    entry.kind,
                    entry.created_at.as_millis(),
                    entry.expires_at.map(|t| t.as_millis()),
                ],
            )?;
            Ok(())
        })
    }

    pub fn get(&self, key: &str, expected: &Fingerprint) -> Result<Option<Vec<u8>>> {
        let row: Option<(String, String)> = self.store.with_conn(|conn| {
            conn.query_row(
                "SELECT fingerprint, blob_hash FROM cache_entries WHERE cache_key = ?1",
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(StorageError::from)
        })?;
        let Some((fingerprint_json, blob_hash)) = row else {
            return Ok(None);
        };
        let stored: Fingerprint = serde_json::from_str(&fingerprint_json)?;
        if stored.hash != expected.hash {
            self.invalidate(key)?;
            return Ok(None);
        }
        let hash = ContentHash::from_hex(&blob_hash).map_err(|err| StorageError::msg(err.to_string()))?;
        Ok(Some(self.store.blobs().get(hash)?))
    }

    pub fn invalidate(&self, key: &str) -> Result<()> {
        self.store.with_conn(|conn| {
            conn.execute("DELETE FROM cache_entries WHERE cache_key = ?1", [key])?;
            Ok(())
        })
    }

    pub fn invalidate_kind(&self, kind: &str) -> Result<usize> {
        self.store.with_conn(|conn| {
            let n = conn.execute("DELETE FROM cache_entries WHERE kind = ?1", [kind])?;
            Ok(n)
        })
    }

    pub fn rebuild_cache(&self) -> Result<()> {
        self.store.with_conn(|conn| {
            conn.execute_batch("DELETE FROM cache_entries;")?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Store;

    #[test]
    fn cache_misses_when_fingerprint_changes() {
        let store = Store::open_in_memory().unwrap();
        let cache = store.cache();
        let fp = Fingerprint::of("syntax", &[b"fn a() {}"]);
        cache
            .put(
                &CacheEntry {
                    key: "file:a.rs".into(),
                    fingerprint: fp.clone(),
                    blob_hash: fp.hash,
                    kind: "syntax".into(),
                    created_at: Timestamp::now(),
                    expires_at: None,
                },
                b"tree",
            )
            .unwrap();
        assert_eq!(cache.get("file:a.rs", &fp).unwrap().as_deref(), Some(b"tree".as_ref()));
        let changed = Fingerprint::of("syntax", &[b"fn a() { 1 }"]);
        assert!(cache.get("file:a.rs", &changed).unwrap().is_none());
    }
}
