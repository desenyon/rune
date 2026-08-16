use crate::budget::BudgetCategory;
use crate::intent::Intent;
use crate::Result;
use rune_core::{Node, NodeKind, Validity};
use rune_storage::Store;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub node: Node,
    pub category: BudgetCategory,
    pub query_relevance: f32,
    pub structural_proximity: f32,
    pub source: String,
    pub retrieval_path: Vec<String>,
    pub content: String,
}

pub fn category_for_kind(kind: &NodeKind) -> BudgetCategory {
    match kind {
        NodeKind::Task => BudgetCategory::Task,
        NodeKind::Specification | NodeKind::Requirement => BudgetCategory::Specification,
        NodeKind::File
        | NodeKind::Symbol
        | NodeKind::Function
        | NodeKind::Method
        | NodeKind::Class
        | NodeKind::Interface
        | NodeKind::Trait
        | NodeKind::Type
        | NodeKind::Variable
        | NodeKind::Module => BudgetCategory::Code,
        NodeKind::Directory | NodeKind::Repository | NodeKind::Package | NodeKind::Dependency => {
            BudgetCategory::Structure
        }
        NodeKind::Memory | NodeKind::Constraint | NodeKind::Preference => BudgetCategory::Memory,
        NodeKind::Session
        | NodeKind::Turn
        | NodeKind::Decision
        | NodeKind::Attempt
        | NodeKind::Failure
        | NodeKind::Discovery
        | NodeKind::Handoff => BudgetCategory::History,
        NodeKind::Test => BudgetCategory::Tests,
        NodeKind::Document | NodeKind::ExternalDocument | NodeKind::DocumentationSection => {
            BudgetCategory::Documentation
        }
        NodeKind::Commit | NodeKind::Branch | NodeKind::Tag | NodeKind::Worktree => {
            BudgetCategory::Git
        }
        _ => BudgetCategory::Conversation,
    }
}

pub fn candidate_from_node(
    node: Node,
    source: &str,
    path: Vec<String>,
    relevance: f32,
) -> Candidate {
    let category = category_for_kind(&node.kind);
    let content = node.search_body();
    Candidate {
        node,
        category,
        query_relevance: relevance,
        structural_proximity: 0.0,
        source: source.to_string(),
        retrieval_path: path,
        content,
    }
}

pub trait TaskRetriever: Send + Sync {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>>;
}

pub trait SpecRetriever: Send + Sync {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>>;
}

pub trait MemoryRetriever: Send + Sync {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>>;
}

pub trait HistoryRetriever: Send + Sync {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>>;
}

pub trait GitRetriever: Send + Sync {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>>;
}

pub trait DocsRetriever: Send + Sync {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>>;
}

/// Explicit missing-subsystem implementation. Returns no extra candidates.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyRetriever;

impl TaskRetriever for EmptyRetriever {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>> {
        kind_search(store, intent, NodeKind::Task, "task_retriever")
    }
}

impl SpecRetriever for EmptyRetriever {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>> {
        let mut out = kind_search(store, intent, NodeKind::Specification, "spec_retriever")?;
        out.extend(kind_search(
            store,
            intent,
            NodeKind::Requirement,
            "spec_retriever",
        )?);
        Ok(out)
    }
}

impl MemoryRetriever for EmptyRetriever {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>> {
        kind_search(store, intent, NodeKind::Memory, "memory_retriever")
    }
}

impl HistoryRetriever for EmptyRetriever {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>> {
        let mut out = kind_search(store, intent, NodeKind::Decision, "history_retriever")?;
        out.extend(kind_search(
            store,
            intent,
            NodeKind::Attempt,
            "history_retriever",
        )?);
        out.extend(kind_search(
            store,
            intent,
            NodeKind::Failure,
            "history_retriever",
        )?);
        out.extend(kind_search(
            store,
            intent,
            NodeKind::Session,
            "history_retriever",
        )?);
        Ok(out)
    }
}

impl GitRetriever for EmptyRetriever {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>> {
        kind_search(store, intent, NodeKind::Commit, "git_retriever")
    }
}

impl DocsRetriever for EmptyRetriever {
    fn retrieve(&self, intent: &Intent, store: &Store) -> Result<Vec<Candidate>> {
        let mut out = kind_search(store, intent, NodeKind::Document, "docs_retriever")?;
        out.extend(kind_search(
            store,
            intent,
            NodeKind::ExternalDocument,
            "docs_retriever",
        )?);
        Ok(out)
    }
}

fn kind_search(
    store: &Store,
    intent: &Intent,
    kind: NodeKind,
    source: &str,
) -> Result<Vec<Candidate>> {
    let mut out = Vec::new();
    for node in store.nodes_of_kind(kind)? {
        if node.validity == Validity::Archived {
            continue;
        }
        let hay = node.search_body().to_ascii_lowercase();
        let hits = intent
            .keywords
            .iter()
            .filter(|k| hay.contains(&k.to_ascii_lowercase()))
            .count();
        if hits == 0 && !intent.keywords.is_empty() {
            continue;
        }
        let relevance = if intent.keywords.is_empty() {
            0.2
        } else {
            hits as f32 / intent.keywords.len() as f32
        };
        out.push(candidate_from_node(
            node,
            source,
            vec![source.to_string()],
            relevance,
        ));
    }
    Ok(out)
}
