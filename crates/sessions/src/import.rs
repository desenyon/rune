use crate::error::Result;
use crate::extract::{extract, SessionIntelligence};
use crate::model::NormalizedSession;
use rune_core::{
    Edge, EdgeKind, Node, NodeId, NodeKind, Provenance, ProvenanceSource, ProvenanceSubject,
    Validity,
};
use rune_memory::{ClaimKind, Extractor, MemoryStore};
use rune_storage::Store;

#[derive(Clone, Debug)]
pub struct PersistedSession {
    pub session_id: NodeId,
    pub turn_ids: Vec<NodeId>,
    pub extraction_ids: Vec<NodeId>,
    pub memory_ids: Vec<NodeId>,
    pub intelligence: SessionIntelligence,
}

/// Import a normalized session. Provenance is always `AgentSession`. Extractions are derived.
pub fn persist(store: &Store, session: &NormalizedSession) -> Result<PersistedSession> {
    let blob = store
        .blobs()
        .put(session.raw.as_bytes(), Some("text/plain"))?;
    store.record_blob(&blob)?;
    let intelligence = extract(&session.turns);
    let payload = serde_json::json!({
        "provider": session.provider,
        "external_id": session.external_id,
        "source_path": session.source_path,
        "cwd": session.cwd,
        "goal": intelligence.goal,
        "files_touched": intelligence.files_touched,
        "raw": session.raw,
        "blob_hash": blob.hash.to_hex(),
    });
    let node = Node::new(
        NodeKind::Session,
        session
            .title
            .clone()
            .or_else(|| Some(session.external_id.clone())),
        payload,
    );
    store.upsert_node(&node)?;
    insert_session_provenance(store, node.id, session, None)?;

    let mut turn_ids = Vec::new();
    for turn in &session.turns {
        let turn_payload = serde_json::json!({
            "provider": session.provider,
            "session_external_id": session.external_id,
            "turn_external_id": turn.external_id,
            "role": turn.role,
            "text": turn.text,
            "raw": turn.raw,
            "timestamp": turn.timestamp,
        });
        let turn_node = Node::new(
            NodeKind::Turn,
            Some(format!("{}:{}", turn.role, turn.external_id)),
            turn_payload,
        );
        store.upsert_node(&turn_node)?;
        store.upsert_edge(&Edge::new(node.id, turn_node.id, EdgeKind::Contains))?;
        insert_session_provenance(store, turn_node.id, session, Some(&turn.external_id))?;
        turn_ids.push(turn_node.id);
    }

    let mut extraction_ids = Vec::new();
    for item in &intelligence.items {
        let Some(turn_id) = turn_ids.get(item.source_turn_index).copied() else {
            continue;
        };
        let mut extracted = Node::new(
            item.kind.clone(),
            Some(item.statement.chars().take(80).collect::<String>()),
            serde_json::json!({
                "extracted_as": item.extracted_as,
                "statement": item.statement,
                "source_turn_external_id": item.source_turn_external_id,
                "heuristic": true,
            }),
        );
        extracted.validity = Validity::Candidate;
        store.upsert_node(&extracted)?;
        let edge_kind = match item.extracted_as.as_str() {
            "attempt" => EdgeKind::AttemptedIn,
            "failure" => EdgeKind::FailedIn,
            "decision" => EdgeKind::DecidedIn,
            "discovery" => EdgeKind::DiscoveredIn,
            _ => EdgeKind::DiscussedIn,
        };
        store.upsert_edge(&Edge::new(extracted.id, turn_id, edge_kind))?;
        store.upsert_edge(&Edge::new(extracted.id, node.id, EdgeKind::DiscussedIn))?;
        let provenance = Provenance::inferred(
            ProvenanceSubject::Node(extracted.id),
            "session_heuristic_extract",
            vec![turn_id.to_string(), node.id.to_string()],
        );
        store.insert_provenance(&provenance)?;
        extraction_ids.push(extracted.id);
    }

    let memory_ids = ingest_session_memories(store, session, node.id)?;

    Ok(PersistedSession {
        session_id: node.id,
        turn_ids,
        extraction_ids,
        memory_ids,
        intelligence,
    })
}

fn ingest_session_memories(
    store: &Store,
    session: &NormalizedSession,
    session_id: NodeId,
) -> Result<Vec<NodeId>> {
    let payload = serde_json::json!({
        "session_id": session.external_id,
        "provider": session.provider,
        "turns": session
            .turns
            .iter()
            .map(|turn| {
                serde_json::json!({
                    "id": turn.external_id,
                    "role": turn.role,
                    "content": turn.text,
                })
            })
            .collect::<Vec<_>>(),
    });
    let claims = Extractor::from_session_json(&payload).map_err(|err| {
        crate::error::SessionError::Message(err.to_string())
    })?;
    let memories = MemoryStore::new(store);
    let mut ids = Vec::new();
    for mut claim in claims {
        // Auto-ingest only non-verified kinds. Observed facts from transcripts stay
        // out of guidance until a human confirms them.
        if !matches!(
            claim.claim_kind,
            ClaimKind::AgentInference | ClaimKind::TemporaryAssumption | ClaimKind::HumanPreference
        ) {
            continue;
        }
        claim.related_nodes.push(session_id);
        let record = memories
            .ingest(claim)
            .map_err(|err| crate::error::SessionError::Message(err.to_string()))?;
        ids.push(record.id);
    }
    Ok(ids)
}

fn insert_session_provenance(
    store: &Store,
    node_id: NodeId,
    session: &NormalizedSession,
    turn_id: Option<&str>,
) -> Result<()> {
    let provenance = Provenance::observed(
        ProvenanceSubject::Node(node_id),
        ProvenanceSource::AgentSession {
            session_id: session.external_id.clone(),
            turn_id: turn_id.map(str::to_string),
            provider: Some(session.provider.clone()),
        },
    );
    store.insert_provenance(&provenance)?;
    Ok(())
}
