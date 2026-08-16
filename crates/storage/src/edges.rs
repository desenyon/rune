use crate::db::Store;
use crate::error::{Result, StorageError};
use rune_core::{Edge, EdgeId, EdgeKind, EdgeMetadata, NodeId, Timestamp, Validity};
use rusqlite::{params, OptionalExtension};
use std::str::FromStr;

impl Store {
    pub fn upsert_edge(&self, edge: &Edge) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO edges (id, from_id, to_id, kind, metadata, created_at, validity)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    from_id = excluded.from_id,
                    to_id = excluded.to_id,
                    kind = excluded.kind,
                    metadata = excluded.metadata,
                    validity = excluded.validity",
                params![
                    edge.id.to_string(),
                    edge.from.to_string(),
                    edge.to.to_string(),
                    edge.kind.as_str(),
                    serde_json::to_string(&edge.metadata)?,
                    edge.created_at.as_millis(),
                    edge.validity.to_string(),
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_edge(&self, id: EdgeId) -> Result<Edge> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, from_id, to_id, kind, metadata, created_at, validity FROM edges WHERE id = ?1",
                [id.to_string()],
                row_to_edge,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id.to_string()),
                other => StorageError::Sqlite(other),
            })
        })
    }

    pub fn edges_from(&self, id: NodeId) -> Result<Vec<Edge>> {
        self.query_edges(
            "SELECT id, from_id, to_id, kind, metadata, created_at, validity FROM edges WHERE from_id = ?1",
            id,
        )
    }

    pub fn edges_to(&self, id: NodeId) -> Result<Vec<Edge>> {
        self.query_edges(
            "SELECT id, from_id, to_id, kind, metadata, created_at, validity FROM edges WHERE to_id = ?1",
            id,
        )
    }

    pub fn edges_from_kind(&self, id: NodeId, kind: EdgeKind) -> Result<Vec<Edge>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, from_id, to_id, kind, metadata, created_at, validity
                 FROM edges WHERE from_id = ?1 AND kind = ?2",
            )?;
            let rows = stmt.query_map(params![id.to_string(), kind.as_str()], row_to_edge)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    pub fn find_edge(&self, from: NodeId, to: NodeId, kind: EdgeKind) -> Result<Option<Edge>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, from_id, to_id, kind, metadata, created_at, validity
                 FROM edges WHERE from_id = ?1 AND to_id = ?2 AND kind = ?3 LIMIT 1",
                params![from.to_string(), to.to_string(), kind.as_str()],
                row_to_edge,
            )
            .optional()
            .map_err(StorageError::from)
        })
    }

    pub fn delete_edge(&self, id: EdgeId) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM edges WHERE id = ?1", [id.to_string()])?;
            Ok(())
        })
    }

    pub fn edge_count(&self) -> Result<i64> {
        self.with_conn(|conn| {
            conn.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
                .map_err(StorageError::from)
        })
    }

    fn query_edges(&self, sql: &str, id: NodeId) -> Result<Vec<Edge>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(sql)?;
            let rows = stmt.query_map([id.to_string()], row_to_edge)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }
}

fn row_to_edge(row: &rusqlite::Row<'_>) -> rusqlite::Result<Edge> {
    let id: String = row.get(0)?;
    let from: String = row.get(1)?;
    let to: String = row.get(2)?;
    let kind: String = row.get(3)?;
    let metadata: String = row.get(4)?;
    let created_at: i64 = row.get(5)?;
    let validity: String = row.get(6)?;
    Ok(Edge {
        id: parse_id(&id)?,
        from: parse_id(&from)?,
        to: parse_id(&to)?,
        kind: EdgeKind::parse(&kind),
        metadata: serde_json::from_str::<EdgeMetadata>(&metadata)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?,
        created_at: Timestamp::from_millis(created_at),
        validity: serde_json::from_value(serde_json::Value::String(validity))
            .unwrap_or(Validity::Active),
    })
}

fn parse_id<T: FromStr>(value: &str) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    value
        .parse()
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Store;
    use rune_core::{EdgeKind, Node, NodeKind};

    #[test]
    fn edges_require_existing_nodes() {
        let store = Store::open_in_memory().unwrap();
        let a = Node::new(NodeKind::File, Some("a.rs".into()), serde_json::json!({}));
        let b = Node::new(NodeKind::Symbol, Some("main".into()), serde_json::json!({}));
        let edge = Edge::new(a.id, b.id, EdgeKind::Defines);
        assert!(store.upsert_edge(&edge).is_err());
        store.upsert_node(&a).unwrap();
        store.upsert_node(&b).unwrap();
        store.upsert_edge(&edge).unwrap();
        assert_eq!(store.edges_from(a.id).unwrap().len(), 1);
    }
}
