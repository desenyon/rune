use crate::budget::{BudgetAllocation, BudgetCategory};
use crate::tokens::estimate_tokens;
use rune_core::{Node, NodeId, NodeKind, Timestamp, Validity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectionReason {
    pub object_id: NodeId,
    pub stage: String,
    pub explanation: String,
    pub signals: BTreeMap<String, f32>,
    pub retrieval_path: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvenanceView {
    pub source: String,
    pub retrieval_path: Vec<String>,
    pub reason: String,
    pub confidence: f32,
    pub freshness: Validity,
    pub token_cost: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Warning {
    pub object_id: Option<NodeId>,
    pub kind: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapsuleItem {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: Option<String>,
    pub category: BudgetCategory,
    pub content: String,
    pub tokens: usize,
    pub score: f32,
    pub reason: SelectionReason,
    pub provenance: ProvenanceView,
    pub pinned: bool,
    pub warnings: Vec<Warning>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepositoryState {
    pub path: Option<String>,
    pub node_count: i64,
    pub edge_count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextCapsule {
    pub identifier: NodeId,
    pub goal: String,
    pub task: Option<NodeId>,
    pub agent: Option<String>,
    pub created_at: Timestamp,
    pub repository_state: RepositoryState,
    pub budget: BudgetAllocation,
    pub summary: String,
    pub requirements: Vec<CapsuleItem>,
    pub current_state: String,
    pub relevant_code: Vec<CapsuleItem>,
    pub structural_context: Vec<CapsuleItem>,
    pub tests: Vec<CapsuleItem>,
    pub memory: Vec<CapsuleItem>,
    pub history: Vec<CapsuleItem>,
    pub decisions: Vec<CapsuleItem>,
    pub failed_attempts: Vec<CapsuleItem>,
    pub external_documentation: Vec<CapsuleItem>,
    pub working_tree: Vec<CapsuleItem>,
    pub constraints: Vec<CapsuleItem>,
    pub open_questions: Vec<String>,
    pub recommended_next_actions: Vec<String>,
    pub provenance: Vec<ProvenanceView>,
    pub included: Vec<CapsuleItem>,
    pub excluded_candidates: Vec<(NodeId, String)>,
    pub duplicates_removed: usize,
    pub token_estimate: usize,
    pub warnings: Vec<Warning>,
}

impl ContextCapsule {
    pub fn all_items(&self) -> Vec<&CapsuleItem> {
        let mut items = Vec::new();
        items.extend(self.included.iter());
        items
    }

    pub fn contains(&self, id: &NodeId) -> bool {
        self.included.iter().any(|item| &item.id == id)
    }

    pub fn into_node(self) -> Node {
        let payload = serde_json::to_value(&self).unwrap_or(serde_json::json!({}));
        let mut node = Node::new(NodeKind::ContextCapsule, Some(self.goal.clone()), payload);
        node.id = self.identifier;
        node
    }
}

pub fn bucket_item(item: CapsuleItem, capsule: &mut ContextCapsule) {
    match item.kind {
        NodeKind::Requirement => capsule.requirements.push(item.clone()),
        NodeKind::File
        | NodeKind::Function
        | NodeKind::Symbol
        | NodeKind::Method
        | NodeKind::Class => capsule.relevant_code.push(item.clone()),
        NodeKind::Directory | NodeKind::Module | NodeKind::Repository => {
            capsule.structural_context.push(item.clone())
        }
        NodeKind::Test => capsule.tests.push(item.clone()),
        NodeKind::Memory => capsule.memory.push(item.clone()),
        NodeKind::Decision => capsule.decisions.push(item.clone()),
        NodeKind::Attempt | NodeKind::Failure => capsule.failed_attempts.push(item.clone()),
        NodeKind::ExternalDocument | NodeKind::Document | NodeKind::DocumentationSection => {
            capsule.external_documentation.push(item.clone())
        }
        NodeKind::Worktree | NodeKind::Branch => capsule.working_tree.push(item.clone()),
        NodeKind::Constraint => capsule.constraints.push(item.clone()),
        NodeKind::Session | NodeKind::Turn | NodeKind::Commit => capsule.history.push(item.clone()),
        _ => {}
    }
    capsule.included.push(item);
}

pub fn retokenize(capsule: &mut ContextCapsule) {
    capsule.token_estimate = capsule
        .included
        .iter()
        .map(|item| item.tokens)
        .sum::<usize>()
        + estimate_tokens(&capsule.summary)
        + estimate_tokens(&capsule.current_state);
    capsule.budget.used = capsule.token_estimate;
}
