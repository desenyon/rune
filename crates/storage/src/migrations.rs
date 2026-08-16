use crate::error::{Result, StorageError};
use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct Migration {
    pub version: i64,
    pub name: String,
    pub sql: String,
    pub checksum: String,
}

impl Migration {
    pub fn new(version: i64, name: impl Into<String>, sql: impl Into<String>) -> Self {
        let sql = sql.into();
        let checksum = blake3::hash(sql.as_bytes()).to_hex().to_string();
        Self {
            version,
            name: name.into(),
            sql,
            checksum,
        }
    }
}

pub fn bundled_migrations() -> Vec<Migration> {
    vec![Migration::new(
        1,
        "001_initial",
        include_str!("../migrations/001_initial.sql"),
    )]
}

pub fn migrate(conn: &mut Connection, migrations: &[Migration]) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    ensure_migration_table(conn)?;
    let applied = applied_migrations(conn)?;
    for migration in migrations {
        if let Some(existing) = applied.iter().find(|item| item.0 == migration.version) {
            if existing.1 != migration.checksum {
                return Err(StorageError::Incompatible(format!(
                    "migration {} checksum mismatch; refusing to destroy user state",
                    migration.name
                )));
            }
            continue;
        }
        if applied.iter().any(|(version, _)| *version > migration.version) {
            return Err(StorageError::Incompatible(format!(
                "refusing to apply older migration {} after newer versions",
                migration.name
            )));
        }
        let tx = conn.transaction()?;
        tx.execute_batch(&migration.sql).map_err(|source| StorageError::Migration {
            name: migration.name.clone(),
            source: Box::new(StorageError::Sqlite(source)),
        })?;
        let applied_at = now_millis();
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at, checksum) VALUES (?1, ?2, ?3, ?4)",
            params![migration.version, migration.name, applied_at, migration.checksum],
        )?;
        tx.commit()?;
        tracing::info!(version = migration.version, name = %migration.name, "applied migration");
    }
    Ok(())
}

pub fn applied_migrations(conn: &Connection) -> Result<Vec<(i64, String)>> {
    if !table_exists(conn, "schema_migrations")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare("SELECT version, checksum FROM schema_migrations ORDER BY version")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn ensure_migration_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at INTEGER NOT NULL,
            checksum TEXT NOT NULL
        );",
    )?;
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn fresh_database_applies_initial_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &bundled_migrations()).unwrap();
        let applied = applied_migrations(&conn).unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].0, 1);
    }

    #[test]
    fn checksum_mismatch_refuses_to_destroy_state() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &bundled_migrations()).unwrap();
        let mutated = vec![Migration::new(1, "001_initial", "SELECT 1;")];
        let err = migrate(&mut conn, &mutated).unwrap_err();
        assert!(matches!(err, StorageError::Incompatible(_)));
    }

    #[test]
    fn interrupted_migration_can_retry_after_new_connection() {
        let mut conn = Connection::open_in_memory().unwrap();
        let good = bundled_migrations();
        migrate(&mut conn, &good).unwrap();
        let failing = vec![Migration::new(
            2,
            "002_fail",
            "CREATE TABLE ok (id TEXT PRIMARY KEY); SELECT broken;",
        )];
        assert!(migrate(&mut conn, &failing).is_err());
        let applied = applied_migrations(&conn).unwrap();
        assert_eq!(applied.len(), 1);
        let retry = vec![Migration::new(
            2,
            "002_ok",
            "CREATE TABLE ok (id TEXT PRIMARY KEY);",
        )];
        migrate(&mut conn, &retry).unwrap();
        assert_eq!(applied_migrations(&conn).unwrap().len(), 2);
    }
}
