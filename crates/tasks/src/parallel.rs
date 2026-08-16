use rune_core::{EdgeKind, NodeId, NodeKind};
use rune_graph::Graph;
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::cycle::would_cycle;
use crate::error::{Result, TaskError};
use crate::model::Task;
use crate::store::TaskStore;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParallelizationReport {
    pub task_a: NodeId,
    pub task_b: NodeId,
    /// True only when both tasks declare affected resources and those sets are
    /// disjoint, with no dependency relationship. Never true without evidence.
    pub conflict_free: bool,
    pub confidence: f32,
    pub explanation: String,
    pub overlapping_files: Vec<NodeId>,
    pub overlapping_symbols: Vec<NodeId>,
    pub overlapping_schemas: Vec<NodeId>,
    pub dependency_related: bool,
    pub evidence_complete: bool,
}

pub struct Parallelization<'a> {
    store: &'a Store,
}

impl<'a> Parallelization<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn analyze(&self, left_id: NodeId, right_id: NodeId) -> Result<ParallelizationReport> {
        if left_id == right_id {
            return Err(TaskError::invalid(
                "parallelization analysis requires two distinct tasks",
            ));
        }
        let tasks = TaskStore::new(self.store);
        let left = tasks.get(left_id)?;
        let right = tasks.get(right_id)?;
        self.compare(&left, &right)
    }

    pub fn compare(&self, left: &Task, right: &Task) -> Result<ParallelizationReport> {
        let left_files = self.affected(left, ResourceKind::File)?;
        let right_files = self.affected(right, ResourceKind::File)?;
        let left_symbols = self.affected(left, ResourceKind::Symbol)?;
        let right_symbols = self.affected(right, ResourceKind::Symbol)?;
        let left_schemas = self.affected(left, ResourceKind::Schema)?;
        let right_schemas = self.affected(right, ResourceKind::Schema)?;
        let overlapping_files = intersect(&left_files, &right_files);
        let overlapping_symbols = intersect(&left_symbols, &right_symbols);
        let overlapping_schemas = intersect(&left_schemas, &right_schemas);
        let dependency_related = would_cycle(self.store, left.id, right.id)?.is_some()
            || would_cycle(self.store, right.id, left.id)?.is_some()
            || left.dependencies.contains(&right.id)
            || right.dependencies.contains(&left.id)
            || left.blockers.contains(&right.id)
            || right.blockers.contains(&left.id);
        let left_evidence = has_resource_evidence(left);
        let right_evidence = has_resource_evidence(right);
        let evidence_complete = left_evidence && right_evidence;
        let has_overlap = !overlapping_files.is_empty()
            || !overlapping_symbols.is_empty()
            || !overlapping_schemas.is_empty();

        let (conflict_free, confidence, explanation) = if has_overlap {
            (
                false,
                0.95,
                format!(
                    "tasks share affected resources: files={}, symbols={}, schemas={}",
                    overlapping_files.len(),
                    overlapping_symbols.len(),
                    overlapping_schemas.len()
                ),
            )
        } else if dependency_related {
            (
                false,
                0.9,
                "tasks are related by depends_on or blocks and cannot run independently".to_string(),
            )
        } else if !evidence_complete {
            let missing = match (left_evidence, right_evidence) {
                (false, false) => format!("{} and {}", left.id, right.id),
                (false, true) => left.id.to_string(),
                (true, false) => right.id.to_string(),
                (true, true) => unreachable!("evidence_complete is false"),
            };
            (
                false,
                0.0,
                format!(
                    "refusing conflict-free claim: task {missing} lists no affected files, symbols, or schemas"
                ),
            )
        } else {
            (
                true,
                0.7,
                "both tasks declare affected files/symbols/schemas and those sets are disjoint".to_string(),
            )
        };

        Ok(ParallelizationReport {
            task_a: left.id,
            task_b: right.id,
            conflict_free,
            confidence,
            explanation,
            overlapping_files,
            overlapping_symbols,
            overlapping_schemas,
            dependency_related,
            evidence_complete,
        })
    }

    fn affected(&self, task: &Task, kind: ResourceKind) -> Result<Vec<NodeId>> {
        let mut ids = match kind {
            ResourceKind::File => task.affected_files.clone(),
            ResourceKind::Symbol => task.affected_symbols.clone(),
            ResourceKind::Schema => task.affected_schemas.clone(),
        };
        for neighbor in Graph::new(self.store).neighbors(task.id)? {
            if neighbor.edge.kind != EdgeKind::Affects || !neighbor.outgoing {
                continue;
            }
            let matches = match kind {
                ResourceKind::File => {
                    neighbor.node.kind == NodeKind::File
                        || task.affected_files.contains(&neighbor.node.id)
                }
                ResourceKind::Symbol => {
                    matches!(
                        neighbor.node.kind,
                        NodeKind::Symbol
                            | NodeKind::Function
                            | NodeKind::Method
                            | NodeKind::Class
                            | NodeKind::Trait
                            | NodeKind::Type
                    ) || task.affected_symbols.contains(&neighbor.node.id)
                }
                ResourceKind::Schema => task.affected_schemas.contains(&neighbor.node.id),
            };
            if matches {
                ids.push(neighbor.node.id);
            }
        }
        ids.sort_by_key(|id| id.to_string());
        ids.dedup();
        Ok(ids)
    }
}

enum ResourceKind {
    File,
    Symbol,
    Schema,
}

fn has_resource_evidence(task: &Task) -> bool {
    !task.affected_files.is_empty()
        || !task.affected_symbols.is_empty()
        || !task.affected_schemas.is_empty()
}

fn intersect(left: &[NodeId], right: &[NodeId]) -> Vec<NodeId> {
    let right: BTreeSet<_> = right.iter().copied().collect();
    let mut out: Vec<_> = left
        .iter()
        .copied()
        .filter(|id| right.contains(id))
        .collect();
    out.sort_by_key(|id| id.to_string());
    out.dedup();
    out
}
