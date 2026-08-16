use crate::fingerprint::ContentHash;
use crate::id::NodeId;
use crate::validity::Validity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilePayload {
    pub path: String,
    pub file_key: String,
    pub language: Option<String>,
    pub size: u64,
    pub content_hash: ContentHash,
    pub start_line: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymbolPayload {
    pub name: String,
    pub kind: String,
    pub file_key: String,
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_line: u32,
    pub end_line: u32,
    pub signature: Option<String>,
    pub is_test: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryPayload {
    pub statement: String,
    pub category: MemoryCategory,
    pub scope: String,
    pub confidence: f32,
    pub evidence: Vec<NodeId>,
    pub last_verified: Option<crate::Timestamp>,
    pub origin: MemoryOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    ArchitecturalDecision,
    ProjectConstraint,
    DeveloperPreference,
    VerifiedFact,
    WorkflowConvention,
    FailurePattern,
    SuccessfulProcedure,
    EnvironmentDetail,
    TemporaryContext,
    ExternalDependencyFact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOrigin {
    ObservedFact,
    HumanPreference,
    AgentInference,
    TemporaryAssumption,
}

impl MemoryOrigin {
    pub fn initial_validity(self) -> Validity {
        match self {
            Self::ObservedFact => Validity::Candidate,
            Self::HumanPreference => Validity::Verified,
            Self::AgentInference => Validity::Candidate,
            Self::TemporaryAssumption => Validity::Candidate,
        }
    }

    pub fn may_auto_verify(self) -> bool {
        matches!(self, Self::HumanPreference)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskPayload {
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Ready,
    Active,
    Blocked,
    Failed,
    Review,
    Complete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapsuleSelection {
    pub node_id: NodeId,
    pub reason: String,
    pub signals: Vec<String>,
    pub token_cost: usize,
    pub pinned: bool,
    pub excluded: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_inference_cannot_auto_verify() {
        assert!(!MemoryOrigin::AgentInference.may_auto_verify());
        assert_eq!(
            MemoryOrigin::AgentInference.initial_validity(),
            Validity::Candidate
        );
        assert!(MemoryOrigin::HumanPreference.may_auto_verify());
    }
}
