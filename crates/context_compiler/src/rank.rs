use crate::budget::TaskType;
use crate::retrieve::Candidate;
use rune_core::{NodeKind, Validity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankingWeights {
    pub query_relevance: f32,
    pub structural_proximity: f32,
    pub task_relevance: f32,
    pub specification_relevance: f32,
    pub temporal_relevance: f32,
    pub memory_validity: f32,
    pub source_confidence: f32,
    pub historical_importance: f32,
    pub test_relevance: f32,
    pub git_proximity: f32,
    pub agent_compatibility: f32,
    pub redundancy_penalty: f32,
    pub staleness_penalty: f32,
    pub contradiction_penalty: f32,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            query_relevance: 1.0,
            structural_proximity: 0.7,
            task_relevance: 0.6,
            specification_relevance: 0.6,
            temporal_relevance: 0.4,
            memory_validity: 0.8,
            source_confidence: 0.5,
            historical_importance: 0.4,
            test_relevance: 0.7,
            git_proximity: 0.4,
            agent_compatibility: 0.2,
            redundancy_penalty: 0.5,
            staleness_penalty: 0.9,
            contradiction_penalty: 1.2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScoredCandidate {
    pub candidate: Candidate,
    pub score: f32,
    pub signals: std::collections::BTreeMap<String, f32>,
}

pub fn rank_candidates(
    candidates: Vec<Candidate>,
    weights: &RankingWeights,
    task_type: TaskType,
) -> Vec<ScoredCandidate> {
    let mut scored: Vec<ScoredCandidate> = candidates
        .into_iter()
        .map(|candidate| score_one(candidate, weights, task_type))
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
}

fn score_one(
    candidate: Candidate,
    weights: &RankingWeights,
    task_type: TaskType,
) -> ScoredCandidate {
    let mut signals = std::collections::BTreeMap::new();
    let query = candidate.query_relevance;
    let structural = candidate.structural_proximity;
    let kind = &candidate.node.kind;
    let task_rel = if matches!(kind, NodeKind::Task) {
        1.0
    } else {
        0.1
    };
    let spec_rel = if matches!(kind, NodeKind::Specification | NodeKind::Requirement) {
        1.0
    } else {
        0.1
    };
    let test_rel = if matches!(kind, NodeKind::Test | NodeKind::Failure) {
        1.0
    } else {
        0.1
    };
    let git_rel = if matches!(kind, NodeKind::Commit | NodeKind::Branch) {
        1.0
    } else {
        0.1
    };
    let hist = if matches!(
        kind,
        NodeKind::Decision | NodeKind::Attempt | NodeKind::Failure | NodeKind::Session
    ) {
        1.0
    } else {
        0.1
    };
    let validity = match candidate.node.validity {
        Validity::Verified | Validity::Stable => 1.0,
        Validity::Active => 0.7,
        Validity::Candidate => 0.4,
        Validity::Stale => 0.15,
        Validity::Contradicted | Validity::Superseded => 0.05,
        Validity::Archived | Validity::Invalid => 0.0,
    };
    let stale_pen = if candidate.node.validity == Validity::Stale {
        1.0
    } else {
        0.0
    };
    let contrad_pen = if candidate.node.validity == Validity::Contradicted {
        1.0
    } else {
        0.0
    };
    let type_boost = match (task_type, kind) {
        (TaskType::Debugging, NodeKind::Test | NodeKind::Failure) => 0.4,
        (TaskType::Architecture, NodeKind::Specification | NodeKind::Decision) => 0.4,
        (TaskType::Review, NodeKind::Commit | NodeKind::File) => 0.3,
        _ => 0.0,
    };

    let score = weights.query_relevance * query
        + weights.structural_proximity * structural
        + weights.task_relevance * task_rel
        + weights.specification_relevance * spec_rel
        + weights.temporal_relevance * 0.3
        + weights.memory_validity * validity
        + weights.source_confidence * 0.6
        + weights.historical_importance * hist
        + weights.test_relevance * test_rel
        + weights.git_proximity * git_rel
        + weights.agent_compatibility * 0.2
        + type_boost
        - weights.staleness_penalty * stale_pen
        - weights.contradiction_penalty * contrad_pen;

    signals.insert("query_relevance".into(), query);
    signals.insert("structural_proximity".into(), structural);
    signals.insert("memory_validity".into(), validity);
    signals.insert("staleness_penalty".into(), stale_pen);
    signals.insert("contradiction_penalty".into(), contrad_pen);
    signals.insert("type_boost".into(), type_boost);

    ScoredCandidate {
        candidate,
        score,
        signals,
    }
}
