//! Context compiler (S024–S027, S089–S094, plus S023/S032 types).
//!
//! Domain-only: no terminal renderer coupling.

mod budget;
mod capsule;
mod compiler;
mod diff;
mod exclude;
mod inspect;
mod intent;
mod rank;
mod retrieve;
mod tokens;

pub use budget::{allocate_budget, BudgetAllocation, BudgetCategory, TaskType};
pub use capsule::{
    CapsuleItem, ContextCapsule, ProvenanceView, RepositoryState, SelectionReason, Warning,
};
pub use compiler::{CompileRequest, CompiledContext, ContextCompiler, Retrievers};
pub use diff::{compare_capsules, compare_knowledge, AgentKnowledgeComparison, CapsuleDiff};
pub use exclude::{Exclusion, ExclusionScope, PinSet};
pub use inspect::{explain_why, Inspection};
pub use intent::{analyze_intent, Intent, RetrievalMode};
pub use rank::{rank_candidates, RankingWeights, ScoredCandidate};
pub use retrieve::{
    candidate_from_node, Candidate, DocsRetriever, EmptyRetriever, GitRetriever, HistoryRetriever,
    MemoryRetriever, SpecRetriever, TaskRetriever,
};
pub use tokens::estimate_tokens;

pub use compiler::{
    archive_branch, branch_context, merge_branches, snapshot_context, ContextBranch,
    ContextController, ContextPack, PackKind,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error(transparent)]
    Compression(#[from] rune_compression::CompressionError),
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, CompilerError>;
