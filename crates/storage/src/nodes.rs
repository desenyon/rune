use crate::db::Store;
use crate::error::{Result, StorageError};
use crate::fts;
use rune_core::{ContentHash, Node, NodeId, NodeKind, Timestamp, Validity};
use rusqlite::{params, OptionalExtension};
use std::str::FromStr;

impl Store {
    pub fn upsert_node(&self, node: &Node) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO nodes (id, kind, name, payload, created_at, updated_at, content_hash, validity)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    kind = excluded.kind,
                    name = excluded.name,
                    payload = excluded.payload,
                    updated_at = excluded.updated_at,
                    content_hash = excluded.content_hash,
                    validity = excluded.validity",
                params![
                    node.id.to_string(),
                    node.kind.as_str(),
                    node.name,
                    serde_json::to_string(&node.payload)?,
                    node.created_at.as_millis(),
                    node.updated_at.as_millis(),
                    node.content_hash.map(|h| h.to_hex()),
                    node.validity.to_string(),
                ],
            )?;
            fts::upsert(conn, node)?;
            Ok(())
        })
    }

    pub fn get_node(&self, id: NodeId) -> Result<Node> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, kind, name, payload, created_at, updated_at, content_hash, validity FROM nodes WHERE id = ?1",
                [id.to_string()],
                row_to_node,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id.to_string()),
                other => StorageError::Sqlite(other),
            })
        })
    }

    pub fn find_node_by_name(&self, kind: NodeKind, name: &str) -> Result<Option<Node>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, kind, name, payload, created_at, updated_at, content_hash, validity
                 FROM nodes WHERE kind = ?1 AND name = ?2 LIMIT 1",
                params![kind.as_str(), name],
                row_to_node,
            )
            .optional()
            .map_err(StorageError::from)
        })
    }

    pub fn nodes_of_kind(&self, kind: NodeKind) -> Result<Vec<Node>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, kind, name, payload, created_at, updated_at, content_hash, validity
                 FROM nodes WHERE kind = ?1 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([kind.as_str()], row_to_node)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    pub fn delete_node(&self, id: NodeId) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM nodes WHERE id = ?1", [id.to_string()])?;
            fts::delete(conn, id)?;
            Ok(())
        })
    }

    pub fn node_count(&self) -> Result<i64> {
        self.with_conn(|conn| {
            conn.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))
                .map_err(StorageError::from)
        })
    }
}

fn row_to_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<Node> {
    let id: String = row.get(0)?;
    let kind: String = row.get(1)?;
    let name: Option<String> = row.get(2)?;
    let payload: String = row.get(3)?;
    let created_at: i64 = row.get(4)?;
    let updated_at: i64 = row.get(5)?;
    let content_hash: Option<String> = row.get(6)?;
    let validity: String = row.get(7)?;
    Ok(Node {
        id: NodeId::from_str(&id).map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?,
        kind: NodeKind::parse(&kind),
        name,
        payload: serde_json::from_str(&payload)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?,
        created_at: Timestamp::from_millis(created_at),
        updated_at: Timestamp::from_millis(updated_at),
        content_hash: content_hash
            .map(|hex| ContentHash::from_hex(&hex))
            .transpose()
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?,
        validity: parse_validity(&validity),
    })
}

fn parse_validity(value: &str) -> Validity {
    serde_json::from_value(serde_json::Value::String(value.to_string())).unwrap_or(Validity::Active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Store;

    #[test]
    fn upsert_is_idempotent_and_updates_payload() {
        let store = Store::open_in_memory().unwrap();
        let mut node = Node::new(NodeKind::File, Some("a.rs".into()), serde_json::json!({"n": 1}));
        store.upsert_node(&node).unwrap();
        node.payload = serde_json::json!({"n": 2});
        node.touch();
        store.upsert_node(&node).unwrap();
        let loaded = store.get_node(node.id).unwrap();
        assert_eq!(loaded.payload["n"], 2);
        assert_eq!(store.node_count().unwrap(), 1);
    }
}
