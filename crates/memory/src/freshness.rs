use rune_core::{ContentHash, NodeId, Validity};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::error::{MemoryError, Result};
use crate::model::{FreshnessJudgment, FreshnessReason, MemoryRecord};
use crate::store::{node_to_record, record_to_node, MemoryStore};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_ids: Vec<NodeId>,
    pub symbol_ids: Vec<NodeId>,
    pub commit_ids: Vec<NodeId>,
    pub new_file_hashes: BTreeMap<String, String>,
}

pub struct FreshnessEngine<'a> {
    store: &'a Store,
}

impl<'a> FreshnessEngine<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn apply(&self, change: &CodeChange) -> Result<Vec<FreshnessReason>> {
        if change.file_ids.is_empty() && change.symbol_ids.is_empty() && change.commit_ids.is_empty()
        {
            return Err(MemoryError::invalid(
                "freshness evaluation requires at least one changed file, symbol, or commit",
            ));
        }
        let files: BTreeSet<NodeId> = change.file_ids.iter().copied().collect();
        let symbols: BTreeSet<NodeId> = change.symbol_ids.iter().copied().collect();
        let commits: BTreeSet<NodeId> = change.commit_ids.iter().copied().collect();
        let memories = MemoryStore::new(self.store).list()?;
        let mut reasons = Vec::new();
        for mut record in memories {
            let related: BTreeSet<NodeId> = record.related_nodes.iter().copied().collect();
            let affected_files: Vec<NodeId> = related.intersection(&files).copied().collect();
            let affected_symbols: Vec<NodeId> = related.intersection(&symbols).copied().collect();
            let changing_commit = related.intersection(&commits).copied().next();
            let hash_changed = file_hash_changed(&record, change, &affected_files)?;
            let judgment = if hash_changed || !affected_files.is_empty() || !affected_symbols.is_empty()
            {
                if record.validity == Validity::Superseded {
                    FreshnessJudgment::Superseded
                } else if hash_changed {
                    FreshnessJudgment::PossiblyStale
                } else {
                    FreshnessJudgment::PossiblyStale
                }
            } else if changing_commit.is_some() {
                FreshnessJudgment::StillSupported
            } else {
                FreshnessJudgment::StillSupported
            };
            let next_validity = match judgment {
                FreshnessJudgment::PossiblyStale | FreshnessJudgment::LikelyContradicted => {
                    Validity::Stale
                }
                FreshnessJudgment::Superseded => Validity::Superseded,
                FreshnessJudgment::StillSupported => record.validity,
            };
            let reason = FreshnessReason {
                memory_id: record.id,
                judgment,
                previous_evidence: record.evidence.clone(),
                previous_hashes: record.related_hashes.clone(),
                changing_commit,
                affected_symbols: affected_symbols.clone(),
                affected_files: affected_files.clone(),
                explanation: explain(&record, judgment, hash_changed, &affected_files, &affected_symbols),
                judged_at: rune_core::Timestamp::now(),
            };
            if judgment != FreshnessJudgment::StillSupported {
                record.validity = next_validity;
            }
            record.freshness_log.push(reason.clone());
            let node = record_to_node(&record)?;
            self.store.upsert_node(&node)?;
            reasons.push(reason);
        }
        Ok(reasons)
    }

    pub fn inspect(&self, memory_id: NodeId) -> Result<Vec<FreshnessReason>> {
        Ok(self.get_record(memory_id)?.freshness_log)
    }

    fn get_record(&self, id: NodeId) -> Result<MemoryRecord> {
        node_to_record(&self.store.get_node(id)?)
    }
}

fn file_hash_changed(
    record: &MemoryRecord,
    change: &CodeChange,
    affected_files: &[NodeId],
) -> Result<bool> {
    for file in affected_files {
        let key = file.to_string();
        let Some(previous) = record.related_hashes.get(&key) else {
            return Ok(true);
        };
        let Some(new_hex) = change.new_file_hashes.get(&key) else {
            return Err(MemoryError::invalid(format!(
                "changed file {key} is missing from new_file_hashes"
            )));
        };
        ContentHash::from_hex(previous).map_err(|err| MemoryError::msg(err.to_string()))?;
        ContentHash::from_hex(new_hex).map_err(|err| MemoryError::msg(err.to_string()))?;
        if previous != new_hex {
            return Ok(true);
        }
    }
    Ok(false)
}

fn explain(
    record: &MemoryRecord,
    judgment: FreshnessJudgment,
    hash_changed: bool,
    files: &[NodeId],
    symbols: &[NodeId],
) -> String {
    match judgment {
        FreshnessJudgment::StillSupported => {
            format!("memory {} remains supported by current evidence", record.id)
        }
        FreshnessJudgment::PossiblyStale if hash_changed => format!(
            "related file content hash changed for memory {}; previous evidence is no longer current",
            record.id
        ),
        FreshnessJudgment::PossiblyStale => format!(
            "related files {:?} or symbols {:?} changed for memory {}",
            files, symbols, record.id
        ),
        FreshnessJudgment::LikelyContradicted => {
            format!("related changes likely contradict memory {}", record.id)
        }
        FreshnessJudgment::Superseded => format!("memory {} is superseded", record.id),
    }
}

