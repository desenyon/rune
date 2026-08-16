use rune_core::{Edge, EdgeKind, Validity};
use rune_storage::Store;

use crate::error::{MemoryError, Result};
use crate::model::{FreshnessJudgment, FreshnessReason, MemoryRecord};
use crate::store::{authority_rank, record_to_node, MemoryStore};

pub struct ConflictReport {
    pub kept: Vec<MemoryRecord>,
    pub contradicted: Vec<MemoryRecord>,
    pub ranking: Vec<(rune_core::NodeId, u8, i64)>,
}

pub struct ConflictResolver<'a> {
    store: &'a Store,
}

impl<'a> ConflictResolver<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// Preserve both claims, mark a conflict, rank by authority then freshness.
    /// Never overwrites either statement.
    pub fn record_conflict(
        &self,
        left_id: rune_core::NodeId,
        right_id: rune_core::NodeId,
    ) -> Result<ConflictReport> {
        if left_id == right_id {
            return Err(MemoryError::invalid("a memory cannot conflict with itself"));
        }
        let memories = MemoryStore::new(self.store);
        let mut left = memories.get(left_id)?;
        let mut right = memories.get(right_id)?;
        let original_left = left.statement.clone();
        let original_right = right.statement.clone();
        if original_left == original_right {
            return Err(MemoryError::invalid(
                "identical statements are not a conflict",
            ));
        }
        if self.store.find_edge(left_id, right_id, EdgeKind::Contradicts)?.is_none() {
            self.store
                .upsert_edge(&Edge::new(left_id, right_id, EdgeKind::Contradicts))?;
        }
        if self.store.find_edge(right_id, left_id, EdgeKind::Contradicts)?.is_none() {
            self.store
                .upsert_edge(&Edge::new(right_id, left_id, EdgeKind::Contradicts))?;
        }
        let left_rank = authority_rank(&left, self.store)?;
        let right_rank = authority_rank(&right, self.store)?;
        let left_fresh = left.last_verified_at.unwrap_or(left.created_at).as_millis();
        let right_fresh = right.last_verified_at.unwrap_or(right.created_at).as_millis();
        let left_wins = (left_rank, left_fresh) >= (right_rank, right_fresh);
        let (winner, loser) = if left_wins {
            (&mut left, &mut right)
        } else {
            (&mut right, &mut left)
        };
        loser.validity = Validity::Contradicted;
        let reason = FreshnessReason {
            memory_id: loser.id,
            judgment: FreshnessJudgment::LikelyContradicted,
            previous_evidence: loser.evidence.clone(),
            previous_hashes: loser.related_hashes.clone(),
            changing_commit: None,
            affected_symbols: Vec::new(),
            affected_files: Vec::new(),
            explanation: format!(
                "conflicts with {}; ranked lower by authority {} vs {} and freshness",
                winner.id,
                authority_rank(loser, self.store)?,
                authority_rank(winner, self.store)?
            ),
            judged_at: rune_core::Timestamp::now(),
        };
        loser.freshness_log.push(reason);
        self.store.upsert_node(&record_to_node(winner)?)?;
        self.store.upsert_node(&record_to_node(loser)?)?;
        let left = memories.get(left_id)?;
        let right = memories.get(right_id)?;
        if left.statement != original_left || right.statement != original_right {
            return Err(MemoryError::RefuseOverwrite(left_id.to_string()));
        }
        let ranking = vec![
            (left.id, authority_rank(&left, self.store)?, left_fresh),
            (right.id, authority_rank(&right, self.store)?, right_fresh),
        ];
        let kept = vec![left.clone(), right.clone()];
        let contradicted = kept
            .iter()
            .filter(|record| record.validity == Validity::Contradicted)
            .cloned()
            .collect();
        Ok(ConflictReport {
            kept,
            contradicted,
            ranking,
        })
    }
}
