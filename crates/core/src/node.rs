use crate::fingerprint::ContentHash;
use crate::id::NodeId;
use crate::time::Timestamp;
use crate::validity::Validity;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Canonical node kinds. Unknown future kinds deserialize as [`NodeKind::Unknown`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Project,
    Workspace,
    Repository,
    File,
    Directory,
    Module,
    Symbol,
    Function,
    Method,
    Class,
    Interface,
    Trait,
    Type,
    Variable,
    Test,
    Diagnostic,
    Dependency,
    Package,
    Commit,
    Branch,
    Tag,
    PullRequest,
    Issue,
    Worktree,
    Session,
    Turn,
    Agent,
    Author,
    Decision,
    Attempt,
    Failure,
    Discovery,
    Memory,
    Constraint,
    Preference,
    Specification,
    Requirement,
    Task,
    Handoff,
    ContextCapsule,
    Document,
    ExternalDocument,
    DocumentationSection,
    Command,
    Tool,
    Process,
    Port,
    Container,
    RemoteHost,
    Artifact,
    Benchmark,
    Evaluation,
    #[serde(untagged)]
    Unknown(String),
}

impl NodeKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::Workspace => "workspace",
            Self::Repository => "repository",
            Self::File => "file",
            Self::Directory => "directory",
            Self::Module => "module",
            Self::Symbol => "symbol",
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Type => "type",
            Self::Variable => "variable",
            Self::Test => "test",
            Self::Diagnostic => "diagnostic",
            Self::Dependency => "dependency",
            Self::Package => "package",
            Self::Commit => "commit",
            Self::Branch => "branch",
            Self::Tag => "tag",
            Self::PullRequest => "pull_request",
            Self::Issue => "issue",
            Self::Worktree => "worktree",
            Self::Session => "session",
            Self::Turn => "turn",
            Self::Agent => "agent",
            Self::Author => "author",
            Self::Decision => "decision",
            Self::Attempt => "attempt",
            Self::Failure => "failure",
            Self::Discovery => "discovery",
            Self::Memory => "memory",
            Self::Constraint => "constraint",
            Self::Preference => "preference",
            Self::Specification => "specification",
            Self::Requirement => "requirement",
            Self::Task => "task",
            Self::Handoff => "handoff",
            Self::ContextCapsule => "context_capsule",
            Self::Document => "document",
            Self::ExternalDocument => "external_document",
            Self::DocumentationSection => "documentation_section",
            Self::Command => "command",
            Self::Tool => "tool",
            Self::Process => "process",
            Self::Port => "port",
            Self::Container => "container",
            Self::RemoteHost => "remote_host",
            Self::Artifact => "artifact",
            Self::Benchmark => "benchmark",
            Self::Evaluation => "evaluation",
            Self::Unknown(name) => name,
        }
    }

    pub fn parse(value: &str) -> Self {
        serde_json::from_value(serde_json::Value::String(value.to_string()))
            .unwrap_or_else(|_| NodeKind::Unknown(value.to_string()))
    }
}

impl Display for NodeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub content_hash: Option<ContentHash>,
    pub validity: Validity,
}

impl Node {
    pub fn new(kind: NodeKind, name: impl Into<Option<String>>, payload: serde_json::Value) -> Self {
        let now = Timestamp::now();
        let name = name.into();
        let content_hash = payload_hash(&payload);
        Self {
            id: NodeId::generate(),
            kind,
            name,
            payload,
            created_at: now,
            updated_at: now,
            content_hash: Some(content_hash),
            validity: Validity::Active,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Timestamp::now();
        self.content_hash = Some(payload_hash(&self.payload));
    }

    pub fn search_body(&self) -> String {
        let payload = serde_json::to_string(&self.payload).unwrap_or_default();
        match &self.name {
            Some(name) => format!("{name}\n{payload}"),
            None => payload,
        }
    }
}

fn payload_hash(payload: &serde_json::Value) -> ContentHash {
    let canonical = serde_json::to_vec(payload).unwrap_or_else(|_| payload.to_string().into_bytes());
    ContentHash::hash(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_kind_roundtrips() {
        let kind = NodeKind::parse("future_widget");
        assert_eq!(kind, NodeKind::Unknown("future_widget".into()));
        let json = serde_json::to_string(&kind).unwrap();
        let back: NodeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, kind);
    }

    #[test]
    fn known_kind_roundtrips() {
        let json = serde_json::to_string(&NodeKind::ContextCapsule).unwrap();
        let back: NodeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, NodeKind::ContextCapsule);
        assert_eq!(NodeKind::parse("author"), NodeKind::Author);
    }

    #[test]
    fn new_node_has_hash() {
        let node = Node::new(NodeKind::File, Some("main.rs".into()), serde_json::json!({"path": "src/main.rs"}));
        assert!(node.content_hash.is_some());
        assert_eq!(node.kind.as_str(), "file");
    }
}
