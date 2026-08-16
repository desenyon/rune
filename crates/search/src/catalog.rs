use rune_core::{Node, NodeKind};
use rune_storage::Store;

use crate::error::Result;

/// Every known canonical kind. Unknown future kinds are loaded only when the
/// caller names them in `SearchRequest.kinds`.
pub fn searchable_kinds() -> Vec<NodeKind> {
    vec![
        NodeKind::Project,
        NodeKind::Workspace,
        NodeKind::Repository,
        NodeKind::File,
        NodeKind::Directory,
        NodeKind::Module,
        NodeKind::Symbol,
        NodeKind::Function,
        NodeKind::Method,
        NodeKind::Class,
        NodeKind::Interface,
        NodeKind::Trait,
        NodeKind::Type,
        NodeKind::Variable,
        NodeKind::Test,
        NodeKind::Diagnostic,
        NodeKind::Dependency,
        NodeKind::Package,
        NodeKind::Commit,
        NodeKind::Branch,
        NodeKind::Tag,
        NodeKind::PullRequest,
        NodeKind::Issue,
        NodeKind::Worktree,
        NodeKind::Session,
        NodeKind::Turn,
        NodeKind::Agent,
        NodeKind::Decision,
        NodeKind::Attempt,
        NodeKind::Failure,
        NodeKind::Discovery,
        NodeKind::Memory,
        NodeKind::Constraint,
        NodeKind::Preference,
        NodeKind::Specification,
        NodeKind::Requirement,
        NodeKind::Task,
        NodeKind::Handoff,
        NodeKind::ContextCapsule,
        NodeKind::Document,
        NodeKind::ExternalDocument,
        NodeKind::DocumentationSection,
        NodeKind::Command,
        NodeKind::Tool,
        NodeKind::Process,
        NodeKind::Port,
        NodeKind::Container,
        NodeKind::RemoteHost,
        NodeKind::Artifact,
        NodeKind::Benchmark,
        NodeKind::Evaluation,
    ]
}

pub fn load_nodes(store: &Store, kinds: &[NodeKind]) -> Result<Vec<Node>> {
    let kinds = if kinds.is_empty() {
        searchable_kinds()
    } else {
        kinds.to_vec()
    };
    let mut nodes = Vec::new();
    for kind in kinds {
        nodes.extend(store.nodes_of_kind(kind)?);
    }
    Ok(nodes)
}

pub fn haystack(node: &Node) -> String {
    let name = node.name.as_deref().unwrap_or("");
    format!("{name} {} {}", node.id, node.kind)
}
