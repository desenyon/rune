use crate::capsule::ContextCapsule;
use rune_core::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inspection {
    pub object_id: NodeId,
    pub included: bool,
    pub explanation: String,
    pub evidence: Vec<String>,
    pub signals: std::collections::BTreeMap<String, f32>,
    pub retrieval_path: Vec<String>,
    pub freshness: String,
    pub token_cost: usize,
    pub pinned: bool,
}

/// Deterministic inspection (S089): evidence, not opaque scores alone.
pub fn explain_why(capsule: &ContextCapsule, object_id: NodeId) -> Inspection {
    if let Some(item) = capsule.included.iter().find(|i| i.id == object_id) {
        let mut evidence = vec![item.reason.explanation.clone()];
        evidence.push(format!(
            "retrieval path: {}",
            item.reason.retrieval_path.join(" → ")
        ));
        evidence.push(format!("source: {}", item.provenance.source));
        evidence.push(format!("freshness: {:?}", item.provenance.freshness));
        if item.pinned {
            evidence.push("object was pinned and therefore not dropped by ranking".into());
        }
        for warning in &item.warnings {
            evidence.push(format!("warning: {}", warning.message));
        }
        return Inspection {
            object_id,
            included: true,
            explanation: item.reason.explanation.clone(),
            evidence,
            signals: item.reason.signals.clone(),
            retrieval_path: item.reason.retrieval_path.clone(),
            freshness: format!("{:?}", item.provenance.freshness),
            token_cost: item.tokens,
            pinned: item.pinned,
        };
    }
    if let Some((id, why)) = capsule
        .excluded_candidates
        .iter()
        .find(|(id, _)| *id == object_id)
    {
        return Inspection {
            object_id: *id,
            included: false,
            explanation: why.clone(),
            evidence: vec![
                why.clone(),
                "object was a candidate but not selected".into(),
            ],
            signals: Default::default(),
            retrieval_path: vec![],
            freshness: "unknown".into(),
            token_cost: 0,
            pinned: false,
        };
    }
    Inspection {
        object_id,
        included: false,
        explanation: "object was not a candidate for this capsule".into(),
        evidence: vec!["no selection record for this object_id".into()],
        signals: Default::default(),
        retrieval_path: vec![],
        freshness: "unknown".into(),
        token_cost: 0,
        pinned: false,
    }
}
