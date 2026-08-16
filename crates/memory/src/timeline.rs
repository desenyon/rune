use rune_core::{NodeId, Timestamp, Validity};
use rune_storage::Store;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::MemoryRecord;
use crate::store::MemoryStore;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventKind {
    Created,
    Verified,
    BecameStale,
    Contradicted,
    Superseded,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub at: Timestamp,
    pub kind: TimelineEventKind,
    pub memory_id: NodeId,
    pub statement: String,
    pub evidence_count: usize,
}

pub struct MemoryTimeline<'a> {
    store: &'a Store,
}

impl<'a> MemoryTimeline<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn events(&self) -> Result<Vec<TimelineEvent>> {
        let mut events = Vec::new();
        for record in MemoryStore::new(self.store).list()? {
            events.extend(events_for(&record));
        }
        events.sort_by_key(|event| event.at.as_millis());
        Ok(events)
    }

    pub fn for_memory(&self, id: NodeId) -> Result<Vec<TimelineEvent>> {
        let record = MemoryStore::new(self.store).get(id)?;
        let mut events = events_for(&record);
        events.sort_by_key(|event| event.at.as_millis());
        Ok(events)
    }
}

fn events_for(record: &MemoryRecord) -> Vec<TimelineEvent> {
    let mut events = vec![TimelineEvent {
        at: record.created_at,
        kind: TimelineEventKind::Created,
        memory_id: record.id,
        statement: record.statement.clone(),
        evidence_count: record.evidence.len(),
    }];
    if let Some(verified) = record.last_verified_at {
        events.push(TimelineEvent {
            at: verified,
            kind: TimelineEventKind::Verified,
            memory_id: record.id,
            statement: record.statement.clone(),
            evidence_count: record.evidence.len(),
        });
    }
    if let Some(kind) = validity_event(record.validity) {
        let at = record
            .freshness_log
            .last()
            .map(|reason| reason.judged_at)
            .unwrap_or(record.created_at);
        events.push(TimelineEvent {
            at,
            kind,
            memory_id: record.id,
            statement: record.statement.clone(),
            evidence_count: record.evidence.len(),
        });
    }
    events
}

fn validity_event(validity: Validity) -> Option<TimelineEventKind> {
    match validity {
        Validity::Stale => Some(TimelineEventKind::BecameStale),
        Validity::Contradicted => Some(TimelineEventKind::Contradicted),
        Validity::Superseded => Some(TimelineEventKind::Superseded),
        Validity::Archived => Some(TimelineEventKind::Archived),
        _ => None,
    }
}
