use crate::capsule::ContextCapsule;
use rune_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapsuleDiff {
    pub added: Vec<NodeId>,
    pub removed: Vec<NodeId>,
    pub changed_memories: Vec<NodeId>,
    pub changed_assumptions: Vec<String>,
    pub different_code: Vec<NodeId>,
    pub different_tasks: Vec<NodeId>,
    pub different_documentation: Vec<NodeId>,
    pub different_token_allocation: bool,
    pub left_tokens: usize,
    pub right_tokens: usize,
}

pub fn compare_capsules(left: &ContextCapsule, right: &ContextCapsule) -> CapsuleDiff {
    let left_ids: BTreeSet<NodeId> = left.included.iter().map(|i| i.id).collect();
    let right_ids: BTreeSet<NodeId> = right.included.iter().map(|i| i.id).collect();
    let added: Vec<NodeId> = right_ids.difference(&left_ids).copied().collect();
    let removed: Vec<NodeId> = left_ids.difference(&right_ids).copied().collect();

    let mem = |c: &ContextCapsule| -> BTreeSet<NodeId> { c.memory.iter().map(|i| i.id).collect() };
    let changed_memories: Vec<NodeId> = mem(left)
        .symmetric_difference(&mem(right))
        .copied()
        .collect();

    let code =
        |c: &ContextCapsule| -> BTreeSet<NodeId> { c.relevant_code.iter().map(|i| i.id).collect() };
    let docs = |c: &ContextCapsule| -> BTreeSet<NodeId> {
        c.external_documentation.iter().map(|i| i.id).collect()
    };

    CapsuleDiff {
        different_code: code(left)
            .symmetric_difference(&code(right))
            .copied()
            .collect(),
        different_tasks: {
            let lt: BTreeSet<_> = left.task.into_iter().collect();
            let rt: BTreeSet<_> = right.task.into_iter().collect();
            lt.symmetric_difference(&rt).copied().collect()
        },
        different_documentation: docs(left)
            .symmetric_difference(&docs(right))
            .copied()
            .collect(),
        changed_assumptions: {
            let l: BTreeSet<_> = left.open_questions.iter().cloned().collect();
            let r: BTreeSet<_> = right.open_questions.iter().cloned().collect();
            l.symmetric_difference(&r).cloned().collect()
        },
        different_token_allocation: left.budget.by_category != right.budget.by_category
            || left.token_estimate != right.token_estimate,
        left_tokens: left.token_estimate,
        right_tokens: right.token_estimate,
        added,
        removed,
        changed_memories,
    }
}

/// Observable supplied context only. Never claims internal model knowledge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentKnowledgeComparison {
    pub shared_percent: f32,
    pub shared: Vec<NodeId>,
    pub left_only: Vec<NodeId>,
    pub right_only: Vec<NodeId>,
    pub left_label: String,
    pub right_label: String,
}

pub fn compare_knowledge(
    left: &ContextCapsule,
    right: &ContextCapsule,
    left_label: impl Into<String>,
    right_label: impl Into<String>,
) -> AgentKnowledgeComparison {
    let a: BTreeSet<NodeId> = left.included.iter().map(|i| i.id).collect();
    let b: BTreeSet<NodeId> = right.included.iter().map(|i| i.id).collect();
    let shared: Vec<NodeId> = a.intersection(&b).copied().collect();
    let left_only: Vec<NodeId> = a.difference(&b).copied().collect();
    let right_only: Vec<NodeId> = b.difference(&a).copied().collect();
    let union = a.union(&b).count().max(1);
    AgentKnowledgeComparison {
        shared_percent: (shared.len() as f32 / union as f32) * 100.0,
        shared,
        left_only,
        right_only,
        left_label: left_label.into(),
        right_label: right_label.into(),
    }
}
