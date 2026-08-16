use crate::id::{EdgeId, NodeId};
use crate::time::Timestamp;
use crate::validity::Validity;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Contains,
    Defines,
    References,
    Calls,
    Imports,
    Exports,
    Implements,
    Extends,
    Inherits,
    Tests,
    DependsOn,
    RequiredBy,
    ChangedBy,
    CreatedBy,
    DeletedBy,
    IntroducedBy,
    DiscussedIn,
    DecidedIn,
    AttemptedIn,
    FailedIn,
    DiscoveredIn,
    VerifiedBy,
    Contradicts,
    Supersedes,
    DerivedFrom,
    Supports,
    Blocks,
    BlockedBy,
    ImplementsSpec,
    SatisfiesRequirement,
    ViolatesRequirement,
    AssignedTo,
    ExecutedBy,
    HandedFrom,
    HandedTo,
    Uses,
    Documents,
    RelatedTo,
    Affects,
    OwnedBy,
    GeneratedBy,
    RunsOn,
    ListensOn,
    #[serde(untagged)]
    Unknown(String),
}

impl EdgeKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Contains => "contains",
            Self::Defines => "defines",
            Self::References => "references",
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::Exports => "exports",
            Self::Implements => "implements",
            Self::Extends => "extends",
            Self::Inherits => "inherits",
            Self::Tests => "tests",
            Self::DependsOn => "depends_on",
            Self::RequiredBy => "required_by",
            Self::ChangedBy => "changed_by",
            Self::CreatedBy => "created_by",
            Self::DeletedBy => "deleted_by",
            Self::IntroducedBy => "introduced_by",
            Self::DiscussedIn => "discussed_in",
            Self::DecidedIn => "decided_in",
            Self::AttemptedIn => "attempted_in",
            Self::FailedIn => "failed_in",
            Self::DiscoveredIn => "discovered_in",
            Self::VerifiedBy => "verified_by",
            Self::Contradicts => "contradicts",
            Self::Supersedes => "supersedes",
            Self::DerivedFrom => "derived_from",
            Self::Supports => "supports",
            Self::Blocks => "blocks",
            Self::BlockedBy => "blocked_by",
            Self::ImplementsSpec => "implements_spec",
            Self::SatisfiesRequirement => "satisfies_requirement",
            Self::ViolatesRequirement => "violates_requirement",
            Self::AssignedTo => "assigned_to",
            Self::ExecutedBy => "executed_by",
            Self::HandedFrom => "handed_from",
            Self::HandedTo => "handed_to",
            Self::Uses => "uses",
            Self::Documents => "documents",
            Self::RelatedTo => "related_to",
            Self::Affects => "affects",
            Self::OwnedBy => "owned_by",
            Self::GeneratedBy => "generated_by",
            Self::RunsOn => "runs_on",
            Self::ListensOn => "listens_on",
            Self::Unknown(name) => name,
        }
    }

    pub fn parse(value: &str) -> Self {
        serde_json::from_value(serde_json::Value::String(value.to_string()))
            .unwrap_or_else(|_| EdgeKind::Unknown(value.to_string()))
    }
}

impl Display for EdgeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct EdgeMetadata {
    pub confidence: Option<f32>,
    pub source: Option<String>,
    pub timestamp: Option<Timestamp>,
    pub provenance: Option<String>,
    pub version: Option<String>,
    pub validity: Option<Validity>,
    pub weight: Option<f32>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
    pub metadata: EdgeMetadata,
    pub created_at: Timestamp,
    pub validity: Validity,
}

impl Edge {
    pub fn new(from: NodeId, to: NodeId, kind: EdgeKind) -> Self {
        Self {
            id: EdgeId::generate(),
            from,
            to,
            kind,
            metadata: EdgeMetadata::default(),
            created_at: Timestamp::now(),
            validity: Validity::Active,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.metadata.confidence = Some(confidence);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_edge_kind_survives() {
        let kind = EdgeKind::parse("mentions");
        assert_eq!(kind.as_str(), "mentions");
    }
}
