use crate::db::Store;
use crate::error::{Result, StorageError};
use rune_core::{
    EdgeId, NodeId, Provenance, ProvenanceId, ProvenanceSource, ProvenanceSubject, Timestamp,
};
use rusqlite::params;
use std::str::FromStr;

impl Store {
    pub fn insert_provenance(&self, provenance: &Provenance) -> Result<()> {
        let (node_id, edge_id) = match provenance.subject {
            ProvenanceSubject::Node(id) => (Some(id.to_string()), None),
            ProvenanceSubject::Edge(id) => (None, Some(id.to_string())),
        };
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO provenance
                    (id, node_id, edge_id, source_kind, source_ref, source_payload, observed_at, confidence, derived, details)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    provenance.id.to_string(),
                    node_id,
                    edge_id,
                    provenance.source.kind_name(),
                    provenance.source.reference(),
                    serde_json::to_string(&provenance.source)?,
                    provenance.observed_at.as_millis(),
                    provenance.confidence as f64,
                    provenance.derived as i64,
                    provenance.details,
                ],
            )?;
            Ok(())
        })
    }

    pub fn provenance_for_node(&self, id: NodeId) -> Result<Vec<Provenance>> {
        self.query_provenance(
            "SELECT id, node_id, edge_id, source_payload, observed_at, confidence, derived, details
             FROM provenance WHERE node_id = ?1 ORDER BY observed_at",
            id.to_string(),
        )
    }

    pub fn provenance_for_edge(&self, id: EdgeId) -> Result<Vec<Provenance>> {
        self.query_provenance(
            "SELECT id, node_id, edge_id, source_payload, observed_at, confidence, derived, details
             FROM provenance WHERE edge_id = ?1 ORDER BY observed_at",
            id.to_string(),
        )
    }

    fn query_provenance(&self, sql: &str, id: String) -> Result<Vec<Provenance>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(sql)?;
            let rows = stmt.query_map([id], row_to_provenance)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }
}

fn row_to_provenance(row: &rusqlite::Row<'_>) -> rusqlite::Result<Provenance> {
    let id: String = row.get(0)?;
    let node_id: Option<String> = row.get(1)?;
    let edge_id: Option<String> = row.get(2)?;
    let source_payload: String = row.get(3)?;
    let observed_at: i64 = row.get(4)?;
    let confidence: f64 = row.get(5)?;
    let derived: i64 = row.get(6)?;
    let details: Option<String> = row.get(7)?;
    let subject = if let Some(node) = node_id {
        ProvenanceSubject::Node(NodeId::from_str(&node).map_err(to_sql_err)?)
    } else if let Some(edge) = edge_id {
        ProvenanceSubject::Edge(EdgeId::from_str(&edge).map_err(to_sql_err)?)
    } else {
        return Err(rusqlite::Error::InvalidQuery);
    };
    Ok(Provenance {
        id: ProvenanceId::from_str(&id).map_err(to_sql_err)?,
        subject,
        source: serde_json::from_str::<ProvenanceSource>(&source_payload).map_err(to_sql_err)?,
        observed_at: Timestamp::from_millis(observed_at),
        confidence: confidence as f32,
        derived: derived != 0,
        details,
    })
}

fn to_sql_err<E: std::error::Error + Send + Sync + 'static>(err: E) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}

#[cfg(test)]
mod tests {
    use crate::db::Store;
    use rune_core::{Node, NodeKind, Provenance, ProvenanceSource, ProvenanceSubject};

    #[test]
    fn derived_and_observed_provenance_are_distinct() {
        let store = Store::open_in_memory().unwrap();
        let node = Node::new(NodeKind::Memory, Some("auth".into()), serde_json::json!({"statement": "uses redis"}));
        store.upsert_node(&node).unwrap();
        let observed = Provenance::observed(
            ProvenanceSubject::Node(node.id),
            ProvenanceSource::HumanInput {
                actor: "dev".into(),
            },
        );
        let derived = Provenance::inferred(
            ProvenanceSubject::Node(node.id),
            "session_extract",
            vec!["session-1".into()],
        );
        store.insert_provenance(&observed).unwrap();
        store.insert_provenance(&derived).unwrap();
        let all = store.provenance_for_node(node.id).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|p| p.derived));
        assert!(all.iter().any(|p| !p.derived));
    }
}
