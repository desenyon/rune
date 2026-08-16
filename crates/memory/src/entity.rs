use rune_core::{Edge, Node, NodeId, Timestamp, Validity};
use rune_graph::Graph;
use rune_storage::Store;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, Result};
use crate::model::merge_kind;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeSnapshot {
    pub edge: Edge,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MergeRecord {
    pub id: NodeId,
    pub survivor: NodeId,
    pub absorbed: NodeId,
    pub merged_at: Timestamp,
    pub absorbed_snapshot: Node,
    pub original_edges: Vec<EdgeSnapshot>,
    pub redirected_edges: Vec<EdgeSnapshot>,
    pub reversed: bool,
}

pub struct EntityResolver<'a> {
    store: &'a Store,
}

impl<'a> EntityResolver<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn merge(&self, survivor: NodeId, absorbed: NodeId) -> Result<MergeRecord> {
        if survivor == absorbed {
            return Err(MemoryError::invalid("cannot merge a node with itself"));
        }
        let survivor_node = self.store.get_node(survivor)?;
        let absorbed_node = self.store.get_node(absorbed)?;
        if survivor_node.kind != absorbed_node.kind {
            return Err(MemoryError::invalid(format!(
                "refusing to merge {} with {}",
                survivor_node.kind, absorbed_node.kind
            )));
        }
        let mut original_edges = Vec::new();
        let mut redirected_edges = Vec::new();
        let incident: Vec<Edge> = Graph::new(self.store)
            .neighbors(absorbed)?
            .into_iter()
            .map(|neighbor| neighbor.edge)
            .collect();
        for mut edge in incident {
            original_edges.push(EdgeSnapshot { edge: edge.clone() });
            let mut changed = false;
            if edge.from == absorbed && edge.to != survivor {
                edge.from = survivor;
                changed = true;
            }
            if edge.to == absorbed && edge.from != survivor {
                edge.to = survivor;
                changed = true;
            }
            if !changed {
                let mut original = original_edges
                    .last()
                    .ok_or_else(|| MemoryError::msg("merge lost original edge snapshot"))?
                    .edge
                    .clone();
                original.validity = Validity::Superseded;
                self.store.upsert_edge(&original)?;
                continue;
            }
            if self
                .store
                .find_edge(edge.from, edge.to, edge.kind.clone())?
                .is_some()
            {
                let mut original = original_edges
                    .last()
                    .ok_or_else(|| MemoryError::msg("merge lost original edge snapshot"))?
                    .edge
                    .clone();
                original.validity = Validity::Superseded;
                self.store.upsert_edge(&original)?;
                continue;
            }
            self.store.upsert_edge(&edge)?;
            redirected_edges.push(EdgeSnapshot { edge });
        }
        let mut absorbed_updated = absorbed_node.clone();
        absorbed_updated.validity = Validity::Superseded;
        absorbed_updated.touch();
        self.store.upsert_node(&absorbed_updated)?;
        let record = MergeRecord {
            id: NodeId::generate(),
            survivor,
            absorbed,
            merged_at: Timestamp::now(),
            absorbed_snapshot: absorbed_node,
            original_edges,
            redirected_edges,
            reversed: false,
        };
        self.persist(&record)?;
        tracing::info!(survivor = %survivor, absorbed = %absorbed, merge = %record.id, "merged entities");
        Ok(record)
    }

    pub fn unmerge(&self, merge_id: NodeId) -> Result<MergeRecord> {
        let mut record = self.get(merge_id)?;
        if record.reversed {
            return Err(MemoryError::invalid(format!(
                "merge {} was already reversed",
                merge_id
            )));
        }
        if record.absorbed_snapshot.id != record.absorbed {
            return Err(MemoryError::IrreversibleMerge(merge_id.to_string()));
        }
        for redirected in &record.redirected_edges {
            self.store.delete_edge(redirected.edge.id)?;
        }
        for original in &record.original_edges {
            self.store.upsert_edge(&original.edge)?;
        }
        self.store.upsert_node(&record.absorbed_snapshot)?;
        record.reversed = true;
        self.persist(&record)?;
        Ok(record)
    }

    pub fn get(&self, merge_id: NodeId) -> Result<MergeRecord> {
        let node = self.store.get_node(merge_id)?;
        if node.kind != merge_kind() {
            return Err(MemoryError::invalid(format!(
                "{} is not an entity merge record",
                merge_id
            )));
        }
        serde_json::from_value(node.payload.clone()).map_err(|err| MemoryError::msg(err.to_string()))
    }

    pub fn resolve(&self, id: NodeId) -> Result<NodeId> {
        let mut current = id;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 64 {
                return Err(MemoryError::invalid(
                    "merge chain exceeded 64 hops; refusing to continue",
                ));
            }
            let node = self.store.get_node(current)?;
            if node.validity != Validity::Superseded {
                return Ok(current);
            }
            let mut found = None;
            for merge in self.store.nodes_of_kind(merge_kind())? {
                let record: MergeRecord = serde_json::from_value(merge.payload.clone())
                    .map_err(|err| MemoryError::msg(err.to_string()))?;
                if !record.reversed && record.absorbed == current {
                    found = Some(record.survivor);
                    break;
                }
            }
            match found {
                Some(next) => current = next,
                None => return Ok(current),
            }
        }
    }

    fn persist(&self, record: &MergeRecord) -> Result<()> {
        let payload =
            serde_json::to_value(record).map_err(|err| MemoryError::msg(err.to_string()))?;
        let mut node = rune_core::Node::new(
            merge_kind(),
            Some(format!("merge {} -> {}", record.absorbed, record.survivor)),
            payload,
        );
        node.id = record.id;
        if record.reversed {
            node.validity = Validity::Archived;
        }
        self.store.upsert_node(&node)?;
        Ok(())
    }
}
