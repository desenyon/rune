//! Diff intelligence: graph impact of changed files (S050).

use rune_core::{EdgeKind, NodeId, NodeKind};
use rune_graph::Graph;
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::error::Result;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiffImpact {
    pub changed_files: Vec<NodeId>,
    pub changed_symbols: Vec<NodeId>,
    pub dependent_symbols: Vec<NodeId>,
    pub affected_tests: Vec<NodeId>,
    pub related_tasks: Vec<NodeId>,
    pub related_specs: Vec<NodeId>,
    pub potentially_stale_memories: Vec<NodeId>,
    pub related_decisions: Vec<NodeId>,
}

pub fn impact_for_files(store: &Store, file_ids: &[NodeId]) -> Result<DiffImpact> {
    let graph = Graph::new(store);
    let mut impact = DiffImpact {
        changed_files: file_ids.to_vec(),
        ..DiffImpact::default()
    };
    let mut seen_dep = BTreeSet::new();
    let mut seen_tests = BTreeSet::new();
    for file_id in file_ids {
        for neighbor in graph.neighbors(*file_id)? {
            match neighbor.node.kind {
                NodeKind::Function
                | NodeKind::Method
                | NodeKind::Class
                | NodeKind::Trait
                | NodeKind::Type
                | NodeKind::Symbol => {
                    if neighbor.outgoing && neighbor.edge.kind == EdgeKind::Defines {
                        impact.changed_symbols.push(neighbor.node.id);
                    }
                }
                NodeKind::Test => {
                    if seen_tests.insert(neighbor.node.id) {
                        impact.affected_tests.push(neighbor.node.id);
                    }
                }
                NodeKind::Task => impact.related_tasks.push(neighbor.node.id),
                NodeKind::Specification | NodeKind::Requirement => {
                    impact.related_specs.push(neighbor.node.id);
                }
                NodeKind::Memory => impact.potentially_stale_memories.push(neighbor.node.id),
                NodeKind::Decision => impact.related_decisions.push(neighbor.node.id),
                _ => {}
            }
        }
    }
    for symbol_id in impact.changed_symbols.clone() {
        for neighbor in graph.neighbors(symbol_id)? {
            if neighbor.edge.kind == EdgeKind::Calls && !neighbor.outgoing {
                if seen_dep.insert(neighbor.node.id) {
                    impact.dependent_symbols.push(neighbor.node.id);
                }
            }
            if neighbor.node.kind == NodeKind::Test && seen_tests.insert(neighbor.node.id) {
                impact.affected_tests.push(neighbor.node.id);
            }
            if neighbor.node.kind == NodeKind::Memory {
                impact.potentially_stale_memories.push(neighbor.node.id);
            }
            if neighbor.node.kind == NodeKind::Task {
                impact.related_tasks.push(neighbor.node.id);
            }
            if matches!(
                neighbor.node.kind,
                NodeKind::Specification | NodeKind::Requirement
            ) {
                impact.related_specs.push(neighbor.node.id);
            }
            if neighbor.node.kind == NodeKind::Decision {
                impact.related_decisions.push(neighbor.node.id);
            }
        }
    }
    for kind in [NodeKind::Task, NodeKind::Memory, NodeKind::Specification] {
        for node in store.nodes_of_kind(kind.clone())? {
            let related = node
                .payload
                .get("affected_files")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let hit = related.iter().any(|v| {
                v.as_str()
                    .map(|s| file_ids.iter().any(|id| id.to_string() == s))
                    .unwrap_or(false)
            });
            if hit {
                match kind {
                    NodeKind::Task => impact.related_tasks.push(node.id),
                    NodeKind::Memory => impact.potentially_stale_memories.push(node.id),
                    NodeKind::Specification => impact.related_specs.push(node.id),
                    _ => {}
                }
            }
        }
    }
    impact.related_tasks.sort();
    impact.related_tasks.dedup();
    impact.related_specs.sort();
    impact.related_specs.dedup();
    impact.potentially_stale_memories.sort();
    impact.potentially_stale_memories.dedup();
    impact.related_decisions.sort();
    impact.related_decisions.dedup();
    Ok(impact)
}
