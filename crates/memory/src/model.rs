use rune_core::{Node, NodeId, NodeKind, Provenance, ProvenanceSource, Timestamp, Validity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl MemoryCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ArchitecturalDecision => "architectural_decision",
            Self::ProjectConstraint => "project_constraint",
            Self::DeveloperPreference => "developer_preference",
            Self::VerifiedFact => "verified_fact",
            Self::WorkflowConvention => "workflow_convention",
            Self::FailurePattern => "failure_pattern",
            Self::SuccessfulProcedure => "successful_procedure",
            Self::EnvironmentDetail => "environment_detail",
            Self::TemporaryContext => "temporary_context",
            Self::ExternalDependencyFact => "external_dependency_fact",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Project,
    Workspace,
    Repository,
    Session,
    Agent,
    User,
    Global,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    ObservedFact,
    HumanPreference,
    AgentInference,
    TemporaryAssumption,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    Human,
    Observed,
    Agent,
}

impl Authority {
    pub fn rank(self) -> u8 {
        match self {
            Self::Human => 3,
            Self::Observed => 2,
            Self::Agent => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryEvidence {
    pub source: ProvenanceSource,
    pub excerpt: Option<String>,
    pub observed_at: Timestamp,
    pub derived: bool,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: NodeId,
    pub statement: String,
    pub category: MemoryCategory,
    pub scope: MemoryScope,
    pub confidence: f32,
    pub evidence: Vec<MemoryEvidence>,
    pub related_nodes: Vec<NodeId>,
    pub related_hashes: BTreeMap<String, String>,
    pub created_at: Timestamp,
    pub last_verified_at: Option<Timestamp>,
    pub validity: Validity,
    pub claim_kind: ClaimKind,
    pub authority: Authority,
    pub freshness_log: Vec<FreshnessReason>,
}

impl MemoryRecord {
    pub fn from_node(node: &Node) -> Result<Self, serde_json::Error> {
        serde_json::from_value(node.payload.clone())
    }

    pub fn may_guide_agents(&self) -> bool {
        self.validity.may_guide_agents()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    AgentGuidance,
    Historical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessJudgment {
    PossiblyStale,
    LikelyContradicted,
    StillSupported,
    Superseded,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreshnessReason {
    pub memory_id: NodeId,
    pub judgment: FreshnessJudgment,
    pub previous_evidence: Vec<MemoryEvidence>,
    pub previous_hashes: BTreeMap<String, String>,
    pub changing_commit: Option<NodeId>,
    pub affected_symbols: Vec<NodeId>,
    pub affected_files: Vec<NodeId>,
    pub explanation: String,
    pub judged_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtractedClaim {
    pub statement: String,
    pub claim_kind: ClaimKind,
    pub category: MemoryCategory,
    pub scope: MemoryScope,
    pub confidence: f32,
    pub evidence: Vec<MemoryEvidence>,
    pub related_nodes: Vec<NodeId>,
    pub actor: Option<String>,
}

pub fn validity_for_claim(kind: ClaimKind) -> Validity {
    match kind {
        ClaimKind::AgentInference | ClaimKind::TemporaryAssumption => Validity::Candidate,
        ClaimKind::HumanPreference | ClaimKind::ObservedFact => Validity::Verified,
    }
}

pub fn authority_for_claim(kind: ClaimKind, source: Option<&ProvenanceSource>) -> Authority {
    if matches!(kind, ClaimKind::HumanPreference) {
        return Authority::Human;
    }
    if let Some(ProvenanceSource::HumanInput { .. }) = source {
        return Authority::Human;
    }
    if matches!(kind, ClaimKind::AgentInference) || source.map(|s| s.is_derived()).unwrap_or(false)
    {
        return Authority::Agent;
    }
    Authority::Observed
}

pub fn merge_kind() -> NodeKind {
    NodeKind::Unknown("entity_merge".into())
}

pub fn wrap_evidence(source_label: &str, body: &str, source: ProvenanceSource) -> MemoryEvidence {
    let wrapped = rune_security::UntrustedContent::wrap(source_label, body);
    let _ = wrapped.as_instruction();
    let derived = source.is_derived();
    MemoryEvidence {
        source,
        excerpt: Some(wrapped.body),
        observed_at: Timestamp::now(),
        derived,
        confidence: if derived { 0.4 } else { 0.8 },
    }
}

pub fn provenance_from_evidence(node: NodeId, evidence: &MemoryEvidence) -> Provenance {
    let mut provenance = Provenance::observed(rune_core::ProvenanceSubject::Node(node), evidence.source.clone());
    provenance.observed_at = evidence.observed_at;
    provenance.confidence = evidence.confidence;
    provenance.derived = evidence.derived;
    provenance.details = evidence.excerpt.clone();
    provenance
}
