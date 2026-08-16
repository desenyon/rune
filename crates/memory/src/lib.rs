//! Persistent project memory with extraction, freshness, conflicts, and merges.
//!
//! Retrieved memories used for agent guidance exclude stale, contradicted, and
//! superseded records. Historical queries still return them. Agent inferences
//! are stored as candidates and never as verified guidance.

mod conflict;
mod entity;
mod error;
mod extract;
mod freshness;
mod model;
mod store;
mod timeline;

pub use conflict::{ConflictReport, ConflictResolver};
pub use entity::{EntityResolver, MergeRecord};
pub use error::{MemoryError, Result};
pub use extract::Extractor;
pub use freshness::{CodeChange, FreshnessEngine};
pub use model::{
    wrap_evidence, Authority, ClaimKind, ExtractedClaim, FreshnessJudgment, FreshnessReason,
    MemoryCategory, MemoryEvidence, MemoryRecord, MemoryScope, RetrievalMode,
};
pub use store::MemoryStore;
pub use timeline::{MemoryTimeline, TimelineEvent, TimelineEventKind};

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::{ContentHash, Node, NodeKind, Validity};
    use rune_storage::Store;

    #[test]
    fn stale_memory_not_in_guidance_retrieval() {
        let store = Store::open_in_memory().unwrap();
        let memories = MemoryStore::new(&store);
        let verified = memories
            .ingest(Extractor::from_human_statement(
                "dev",
                "Authentication uses PostgreSQL sessions",
                Some(ClaimKind::ObservedFact),
            ).unwrap())
            .unwrap();
        let mut stale = memories
            .ingest(Extractor::from_human_statement(
                "dev",
                "Authentication uses Redis sessions",
                Some(ClaimKind::ObservedFact),
            ).unwrap())
            .unwrap();
        stale.validity = Validity::Stale;
        memories.persist(stale.clone()).unwrap();
        let guidance = memories.retrieve(RetrievalMode::AgentGuidance).unwrap();
        assert!(guidance.iter().any(|item| item.id == verified.id));
        assert!(guidance.iter().all(|item| item.id != stale.id));
        let historical = memories.retrieve(RetrievalMode::Historical).unwrap();
        assert!(historical.iter().any(|item| item.id == stale.id));
    }

    #[test]
    fn agent_inference_stored_as_candidate_not_verified() {
        let store = Store::open_in_memory().unwrap();
        let session = serde_json::json!({
            "session_id": "sess-1",
            "provider": "claude",
            "turns": [{
                "id": "t1",
                "role": "assistant",
                "content": "I think Redis sessions would be faster.",
                "guess": true,
                "claims": [{
                    "statement": "Redis sessions would be faster",
                    "kind": "agent_inference",
                    "category": "temporary_context"
                }]
            }]
        });
        let claims = Extractor::from_session_json(&session).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].claim_kind, ClaimKind::AgentInference);
        let record = MemoryStore::new(&store).ingest(claims.into_iter().next().unwrap()).unwrap();
        assert_eq!(record.validity, Validity::Candidate);
        assert!(!record.may_guide_agents());
        assert!(MemoryStore::new(&store).retrieve(RetrievalMode::AgentGuidance).unwrap().is_empty());
    }

    #[test]
    fn freshness_marks_memory_stale_when_related_file_hash_changes() {
        let store = Store::open_in_memory().unwrap();
        let mut file = Node::new(
            NodeKind::File,
            Some("auth.rs".into()),
            serde_json::json!({"body": "session = redis"}),
        );
        store.upsert_node(&file).unwrap();
        let mut claim = Extractor::from_human_statement(
            "dev",
            "Authentication uses Redis sessions",
            Some(ClaimKind::ObservedFact),
        )
        .unwrap();
        claim.related_nodes.push(file.id);
        let record = MemoryStore::new(&store).ingest(claim).unwrap();
        assert!(record.related_hashes.contains_key(&file.id.to_string()));
        file.payload = serde_json::json!({"body": "session = postgres"});
        file.touch();
        store.upsert_node(&file).unwrap();
        let new_hash = file.content_hash.unwrap();
        let reasons = FreshnessEngine::new(&store)
            .apply(&CodeChange {
                file_ids: vec![file.id],
                symbol_ids: Vec::new(),
                commit_ids: Vec::new(),
                new_file_hashes: [(file.id.to_string(), new_hash.to_hex())]
                    .into_iter()
                    .collect(),
            })
            .unwrap();
        assert!(reasons.iter().any(|reason| {
            reason.memory_id == record.id && reason.judgment == FreshnessJudgment::PossiblyStale
        }));
        let updated = MemoryStore::new(&store).get(record.id).unwrap();
        assert_eq!(updated.validity, Validity::Stale);
        assert!(!updated.may_guide_agents());
        let _ = ContentHash::from_hex(&new_hash.to_hex()).unwrap();
    }

    #[test]
    fn conflicting_memories_both_preserved() {
        let store = Store::open_in_memory().unwrap();
        let memories = MemoryStore::new(&store);
        let left = memories
            .ingest(
                Extractor::from_human_statement(
                    "alice",
                    "Use Redis for sessions",
                    Some(ClaimKind::HumanPreference),
                )
                .unwrap(),
            )
            .unwrap();
        let right = memories
            .ingest(
                Extractor::from_human_statement(
                    "bob",
                    "Use PostgreSQL for sessions",
                    Some(ClaimKind::ObservedFact),
                )
                .unwrap(),
            )
            .unwrap();
        let report = ConflictResolver::new(&store)
            .record_conflict(left.id, right.id)
            .unwrap();
        let left_after = memories.get(left.id).unwrap();
        let right_after = memories.get(right.id).unwrap();
        assert_eq!(left_after.statement, left.statement);
        assert_eq!(right_after.statement, right.statement);
        assert!(
            left_after.validity == Validity::Contradicted
                || right_after.validity == Validity::Contradicted
        );
        assert!(store
            .find_edge(left.id, right.id, rune_core::EdgeKind::Contradicts)
            .unwrap()
            .is_some());
        assert_eq!(report.kept.len(), 2);
        assert_eq!(report.contradicted.len(), 1);
    }

    #[test]
    fn entity_merge_is_reversible() {
        let store = Store::open_in_memory().unwrap();
        let a = Node::new(NodeKind::Symbol, Some("Auth".into()), serde_json::json!({"n": 1}));
        let b = Node::new(NodeKind::Symbol, Some("Authentication".into()), serde_json::json!({"n": 2}));
        let file = Node::new(NodeKind::File, Some("a.rs".into()), serde_json::json!({}));
        store.upsert_node(&a).unwrap();
        store.upsert_node(&b).unwrap();
        store.upsert_node(&file).unwrap();
        store
            .upsert_edge(&rune_core::Edge::new(file.id, b.id, rune_core::EdgeKind::Defines))
            .unwrap();
        let resolver = EntityResolver::new(&store);
        let merge = resolver.merge(a.id, b.id).unwrap();
        assert_eq!(store.get_node(b.id).unwrap().validity, Validity::Superseded);
        assert_eq!(resolver.resolve(b.id).unwrap(), a.id);
        resolver.unmerge(merge.id).unwrap();
        let restored = store.get_node(b.id).unwrap();
        assert_eq!(restored.name.as_deref(), Some("Authentication"));
        assert_eq!(restored.validity, Validity::Active);
        assert!(store
            .find_edge(file.id, b.id, rune_core::EdgeKind::Defines)
            .unwrap()
            .is_some());
    }
}
