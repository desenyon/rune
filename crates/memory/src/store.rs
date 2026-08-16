use rune_core::{
    Edge, EdgeKind, Node, NodeId, NodeKind, ProvenanceSubject, ProvenanceSource, Timestamp, Validity,
};
use rune_storage::Store;
use std::collections::BTreeMap;

use crate::error::{MemoryError, Result};
use crate::model::{
    authority_for_claim, provenance_from_evidence, validity_for_claim, Authority, ExtractedClaim,
    MemoryRecord, RetrievalMode,
};

pub struct MemoryStore<'a> {
    store: &'a Store,
}

impl<'a> MemoryStore<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn persist(&self, mut record: MemoryRecord) -> Result<MemoryRecord> {
        if record.statement.trim().is_empty() {
            return Err(MemoryError::invalid("memory statement must not be empty"));
        }
        if matches!(record.claim_kind, crate::ClaimKind::AgentInference)
            && record.validity.may_guide_agents()
        {
            return Err(MemoryError::invalid(
                "agent inferences cannot be stored as verified or stable guidance",
            ));
        }
        if record.related_hashes.is_empty() {
            record.related_hashes = self.snapshot_hashes(&record.related_nodes)?;
        }
        let node = record_to_node(&record)?;
        self.store.upsert_node(&node)?;
        for related in &record.related_nodes {
            if self.store.find_edge(record.id, *related, EdgeKind::RelatedTo)?.is_none() {
                self.store
                    .upsert_edge(&Edge::new(record.id, *related, EdgeKind::RelatedTo))?;
            }
        }
        tracing::debug!(id = %record.id, validity = %record.validity, "persisted memory");
        Ok(record)
    }

    pub fn ingest(&self, claim: ExtractedClaim) -> Result<MemoryRecord> {
        let validity = if matches!(claim.claim_kind, crate::ClaimKind::AgentInference) {
            Validity::Candidate
        } else {
            validity_for_claim(claim.claim_kind.clone())
        };
        let authority = authority_for_claim(
            claim.claim_kind.clone(),
            claim.evidence.first().map(|e| &e.source),
        );
        let last_verified_at = if validity.may_guide_agents() {
            Some(Timestamp::now())
        } else {
            None
        };
        let record = MemoryRecord {
            id: NodeId::generate(),
            statement: claim.statement,
            category: claim.category,
            scope: claim.scope,
            confidence: claim.confidence,
            evidence: claim.evidence,
            related_nodes: claim.related_nodes,
            related_hashes: BTreeMap::new(),
            created_at: Timestamp::now(),
            last_verified_at,
            validity,
            claim_kind: claim.claim_kind,
            authority,
            freshness_log: Vec::new(),
        };
        let record = self.persist(record)?;
        for evidence in &record.evidence {
            self.store
                .insert_provenance(&provenance_from_evidence(record.id, evidence))?;
        }
        Ok(record)
    }

    pub fn get(&self, id: NodeId) -> Result<MemoryRecord> {
        let node = self.store.get_node(id)?;
        node_to_record(&node)
    }

    pub fn list(&self) -> Result<Vec<MemoryRecord>> {
        let mut records = Vec::new();
        for node in self.store.nodes_of_kind(NodeKind::Memory)? {
            records.push(node_to_record(&node)?);
        }
        Ok(records)
    }

    /// Agent-guidance retrieval excludes stale, contradicted, superseded, archived,
    /// candidate, and any other non-guiding validity. Historical retrieval returns
    /// every stored memory, including those that must not guide agents.
    pub fn retrieve(&self, mode: RetrievalMode) -> Result<Vec<MemoryRecord>> {
        let mut records = self.list()?;
        match mode {
            RetrievalMode::AgentGuidance => {
                records.retain(|record| record.may_guide_agents());
            }
            RetrievalMode::Historical => {}
        }
        Ok(records)
    }

    pub fn set_validity(&self, id: NodeId, validity: Validity) -> Result<MemoryRecord> {
        let mut record = self.get(id)?;
        record.validity = validity;
        if validity.may_guide_agents() {
            record.last_verified_at = Some(Timestamp::now());
        }
        self.persist(record)
    }

    pub fn record_human_decision(
        &self,
        statement: impl Into<String>,
        actor: impl Into<String>,
        related_nodes: Vec<NodeId>,
    ) -> Result<(Node, MemoryRecord)> {
        let statement = statement.into();
        let actor = actor.into();
        if statement.trim().is_empty() {
            return Err(MemoryError::invalid("human decision statement must not be empty"));
        }
        let mut decision = Node::new(
            NodeKind::Decision,
            Some(statement.clone()),
            serde_json::json!({
                "statement": statement,
                "authority": "human",
                "actor": actor,
            }),
        );
        decision.validity = Validity::Verified;
        self.store.upsert_node(&decision)?;
        self.store.insert_provenance(&rune_core::Provenance::observed(
            ProvenanceSubject::Node(decision.id),
            ProvenanceSource::HumanInput { actor: actor.clone() },
        ))?;
        let mut related = related_nodes;
        related.push(decision.id);
        let record = self.ingest(ExtractedClaim {
            statement,
            claim_kind: crate::ClaimKind::HumanPreference,
            category: crate::MemoryCategory::ArchitecturalDecision,
            scope: crate::MemoryScope::Project,
            confidence: 1.0,
            evidence: vec![crate::wrap_evidence(
                "human_decision",
                &actor,
                ProvenanceSource::HumanInput { actor: actor.clone() },
            )],
            related_nodes: related,
            actor: None,
        })?;
        self.store
            .upsert_edge(&Edge::new(record.id, decision.id, EdgeKind::DecidedIn))?;
        Ok((decision, record))
    }

    fn snapshot_hashes(&self, related: &[NodeId]) -> Result<BTreeMap<String, String>> {
        let mut hashes = BTreeMap::new();
        for id in related {
            let node = self.store.get_node(*id)?;
            if let Some(hash) = node.content_hash {
                hashes.insert(id.to_string(), hash.to_hex());
            }
        }
        Ok(hashes)
    }
}

pub fn record_to_node(record: &MemoryRecord) -> Result<Node> {
    let payload = serde_json::to_value(record).map_err(|err| MemoryError::msg(err.to_string()))?;
    let mut node = Node::new(
        NodeKind::Memory,
        Some(record.statement.chars().take(80).collect()),
        payload,
    );
    node.id = record.id;
    node.created_at = record.created_at;
    node.updated_at = Timestamp::now();
    node.validity = record.validity;
    Ok(node)
}

pub fn node_to_record(node: &Node) -> Result<MemoryRecord> {
    if node.kind != NodeKind::Memory {
        return Err(MemoryError::invalid(format!(
            "expected memory node, found {}",
            node.kind
        )));
    }
    let mut record: MemoryRecord =
        serde_json::from_value(node.payload.clone()).map_err(|err| MemoryError::msg(err.to_string()))?;
    record.id = node.id;
    record.validity = node.validity;
    record.created_at = node.created_at;
    Ok(record)
}

pub fn human_authority_from_decision(node: &Node) -> bool {
    node.kind == NodeKind::Decision
        && node
            .payload
            .get("authority")
            .and_then(|value| value.as_str())
            == Some("human")
}

pub fn authority_rank(record: &MemoryRecord, store: &Store) -> Result<u8> {
    let mut rank = record.authority.rank();
    for related in &record.related_nodes {
        if let Ok(node) = store.get_node(*related) {
            if human_authority_from_decision(&node) {
                rank = rank.max(Authority::Human.rank());
            }
        }
    }
    for edge in store.edges_from_kind(record.id, EdgeKind::DecidedIn)? {
        if let Ok(node) = store.get_node(edge.to) {
            if human_authority_from_decision(&node) {
                rank = rank.max(Authority::Human.rank());
            }
        }
    }
    Ok(rank)
}
