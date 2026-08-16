use crate::db::Store;
use crate::error::{Result, StorageError};
use rune_core::{EdgeId, NodeId, Timestamp};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityKind {
    DanglingEdge,
    MissingObject,
    DuplicateIdentity,
    OrphanedSessionReference,
    InvalidTaskDependency,
    InvalidProvenance,
    MigrationInconsistency,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntegritySeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityFinding {
    pub id: String,
    pub kind: IntegrityKind,
    pub severity: IntegritySeverity,
    pub subject_id: Option<String>,
    pub message: String,
    pub repair_action: Option<String>,
    pub created_at: Timestamp,
    pub resolved_at: Option<Timestamp>,
}

impl Store {
    pub fn check_integrity(&self) -> Result<Vec<IntegrityFinding>> {
        let mut findings = Vec::new();
        self.with_conn(|conn| {
            let mut dangling = conn.prepare(
                "SELECT id, from_id, to_id FROM edges
                 WHERE from_id NOT IN (SELECT id FROM nodes)
                    OR to_id NOT IN (SELECT id FROM nodes)",
            )?;
            let rows = dangling.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (id, from, to) = row?;
                findings.push(IntegrityFinding {
                    id: format!("dangling:{id}"),
                    kind: IntegrityKind::DanglingEdge,
                    severity: IntegritySeverity::Error,
                    subject_id: Some(id),
                    message: format!("edge references missing node {from} or {to}"),
                    repair_action: Some("delete_edge".into()),
                    created_at: Timestamp::now(),
                    resolved_at: None,
                });
            }
            Ok(())
        })?;
        self.with_conn(|conn| {
            let mut orphan_prov = conn.prepare(
                "SELECT id FROM provenance
                 WHERE (node_id IS NOT NULL AND node_id NOT IN (SELECT id FROM nodes))
                    OR (edge_id IS NOT NULL AND edge_id NOT IN (SELECT id FROM edges))",
            )?;
            let rows = orphan_prov.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                let id = row?;
                findings.push(IntegrityFinding {
                    id: format!("prov:{id}"),
                    kind: IntegrityKind::InvalidProvenance,
                    severity: IntegritySeverity::Error,
                    subject_id: Some(id),
                    message: "provenance references a missing subject".into(),
                    repair_action: Some("delete_provenance".into()),
                    created_at: Timestamp::now(),
                    resolved_at: None,
                });
            }
            Ok(())
        })?;
        self.persist_findings(&findings)?;
        Ok(findings)
    }

    pub fn repair_finding(&self, finding: &IntegrityFinding) -> Result<()> {
        match finding.kind {
            IntegrityKind::DanglingEdge => {
                if let Some(id) = &finding.subject_id {
                    let edge_id: EdgeId = id.parse().map_err(|err: rune_core::id::IdParseError| {
                        StorageError::Id(err.to_string())
                    })?;
                    self.delete_edge(edge_id)?;
                }
            }
            IntegrityKind::InvalidProvenance => {
                if let Some(id) = &finding.subject_id {
                    self.with_conn(|conn| {
                        conn.execute("DELETE FROM provenance WHERE id = ?1", [id])?;
                        Ok(())
                    })?;
                }
            }
            IntegrityKind::MissingObject
            | IntegrityKind::DuplicateIdentity
            | IntegrityKind::OrphanedSessionReference
            | IntegrityKind::InvalidTaskDependency
            | IntegrityKind::MigrationInconsistency => {
                return Err(StorageError::msg(format!(
                    "repair for {:?} requires an explicit operator action",
                    finding.kind
                )));
            }
        }
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE integrity_findings SET resolved_at = ?1 WHERE id = ?2",
                params![Timestamp::now().as_millis(), finding.id],
            )?;
            Ok(())
        })
    }

    fn persist_findings(&self, findings: &[IntegrityFinding]) -> Result<()> {
        self.with_conn(|conn| {
            for finding in findings {
                conn.execute(
                    "INSERT INTO integrity_findings
                        (id, kind, severity, subject_id, message, repair_action, created_at, resolved_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT(id) DO UPDATE SET
                        message = excluded.message,
                        resolved_at = excluded.resolved_at",
                    params![
                        finding.id,
                        serde_json::to_string(&finding.kind)?,
                        serde_json::to_string(&finding.severity)?,
                        finding.subject_id,
                        finding.message,
                        finding.repair_action,
                        finding.created_at.as_millis(),
                        finding.resolved_at.map(|t| t.as_millis()),
                    ],
                )?;
            }
            Ok(())
        })
    }

    pub fn count_nodes_linking_to(&self, id: NodeId) -> Result<i64> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM edges WHERE to_id = ?1 OR from_id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(StorageError::from)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Store;
    use rune_core::{Node, NodeKind};

    #[test]
    fn healthy_graph_has_no_findings() {
        let store = Store::open_in_memory().unwrap();
        let node = Node::new(NodeKind::Project, Some("rune".into()), serde_json::json!({}));
        store.upsert_node(&node).unwrap();
        assert!(store.check_integrity().unwrap().is_empty());
    }
}
