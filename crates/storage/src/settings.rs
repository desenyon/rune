use crate::db::Store;
use crate::error::Result;
use rune_core::Timestamp;
use rusqlite::{params, OptionalExtension};

pub struct SettingsStore<'a> {
    pub store: &'a Store,
}

impl SettingsStore<'_> {
    pub fn set(&self, scope: &str, key: &str, value: &serde_json::Value) -> Result<()> {
        self.store.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (scope, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scope, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![scope, key, serde_json::to_string(value)?, Timestamp::now().as_millis()],
            )?;
            Ok(())
        })
    }

    pub fn get(&self, scope: &str, key: &str) -> Result<Option<serde_json::Value>> {
        self.store.with_conn(|conn| {
            let raw: Option<String> = conn
                .query_row(
                    "SELECT value FROM settings WHERE scope = ?1 AND key = ?2",
                    params![scope, key],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(raw.map(|text| serde_json::from_str(&text)).transpose()?)
        })
    }

    pub fn list_scope(&self, scope: &str) -> Result<Vec<(String, serde_json::Value)>> {
        self.store.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT key, value FROM settings WHERE scope = ?1")?;
            let rows = stmt.query_map([scope], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((key, value))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (key, value) = row?;
                out.push((key, serde_json::from_str(&value)?));
            }
            Ok(out)
        })
    }
}
