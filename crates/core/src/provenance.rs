use crate::id::{EdgeId, NodeId, ProvenanceId};
use crate::time::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceSubject {
    Node(NodeId),
    Edge(EdgeId),
}

/// Origin of a fact. Derived inferences must set `derived` on [`crate::Provenance`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProvenanceSource {
    SourceCode {
        path: String,
        start_byte: Option<u32>,
        end_byte: Option<u32>,
        start_line: Option<u32>,
        end_line: Option<u32>,
    },
    GitCommit {
        sha: String,
    },
    AgentSession {
        session_id: String,
        turn_id: Option<String>,
        provider: Option<String>,
    },
    HumanInput {
        actor: String,
    },
    Test {
        name: String,
        run_id: Option<String>,
    },
    Specification {
        spec_id: String,
        requirement_id: Option<String>,
    },
    Documentation {
        doc_id: String,
        section: Option<String>,
    },
    ExternalApi {
        provider: String,
        endpoint: Option<String>,
    },
    DerivedInference {
        method: String,
        inputs: Vec<String>,
    },
}

impl ProvenanceSource {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::SourceCode { .. } => "source_code",
            Self::GitCommit { .. } => "git_commit",
            Self::AgentSession { .. } => "agent_session",
            Self::HumanInput { .. } => "human_input",
            Self::Test { .. } => "test",
            Self::Specification { .. } => "specification",
            Self::Documentation { .. } => "documentation",
            Self::ExternalApi { .. } => "external_api",
            Self::DerivedInference { .. } => "derived_inference",
        }
    }

    pub fn is_derived(&self) -> bool {
        matches!(self, Self::DerivedInference { .. })
    }

    pub fn reference(&self) -> String {
        match self {
            Self::SourceCode { path, start_line, .. } => match start_line {
                Some(line) => format!("{path}:{line}"),
                None => path.clone(),
            },
            Self::GitCommit { sha } => sha.clone(),
            Self::AgentSession { session_id, turn_id, .. } => match turn_id {
                Some(turn) => format!("{session_id}#{turn}"),
                None => session_id.clone(),
            },
            Self::HumanInput { actor } => actor.clone(),
            Self::Test { name, run_id } => match run_id {
                Some(run) => format!("{name}@{run}"),
                None => name.clone(),
            },
            Self::Specification { spec_id, requirement_id } => match requirement_id {
                Some(req) => format!("{spec_id}/{req}"),
                None => spec_id.clone(),
            },
            Self::Documentation { doc_id, section } => match section {
                Some(section) => format!("{doc_id}#{section}"),
                None => doc_id.clone(),
            },
            Self::ExternalApi { provider, endpoint } => match endpoint {
                Some(endpoint) => format!("{provider}:{endpoint}"),
                None => provider.clone(),
            },
            Self::DerivedInference { method, .. } => method.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub id: ProvenanceId,
    pub subject: ProvenanceSubject,
    pub source: ProvenanceSource,
    pub observed_at: Timestamp,
    pub confidence: f32,
    /// True when the fact was inferred rather than directly observed.
    pub derived: bool,
    pub details: Option<String>,
}

impl Provenance {
    pub fn observed(subject: ProvenanceSubject, source: ProvenanceSource) -> Self {
        let derived = source.is_derived();
        Self {
            id: ProvenanceId::generate(),
            subject,
            source,
            observed_at: Timestamp::now(),
            confidence: if derived { 0.4 } else { 1.0 },
            derived,
            details: None,
        }
    }

    pub fn inferred(subject: ProvenanceSubject, method: impl Into<String>, inputs: Vec<String>) -> Self {
        Self::observed(
            subject,
            ProvenanceSource::DerivedInference {
                method: method.into(),
                inputs,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_inference_is_marked_derived() {
        let p = Provenance::inferred(
            ProvenanceSubject::Node(NodeId::generate()),
            "call_graph",
            vec!["sym_a".into(), "sym_b".into()],
        );
        assert!(p.derived);
        assert!(p.confidence < 1.0);
    }

    #[test]
    fn human_input_is_not_derived() {
        let p = Provenance::observed(
            ProvenanceSubject::Node(NodeId::generate()),
            ProvenanceSource::HumanInput {
                actor: "developer".into(),
            },
        );
        assert!(!p.derived);
        assert_eq!(p.confidence, 1.0);
    }
}
