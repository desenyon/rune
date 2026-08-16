use crate::error::{IndexError, Result};
use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const SCOPE: &str = "index";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingCall {
    pub callee: String,
    pub caller_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileRecord {
    pub node_id: String,
    pub content_hash: String,
    pub path: String,
    pub symbol_ids: Vec<String>,
    pub import_ids: Vec<String>,
    #[serde(default)]
    pub pending_calls: Vec<PendingCall>,
}

pub fn load_file_record(store: &Store, file_key: &str) -> Result<Option<FileRecord>> {
    match store.settings().get(SCOPE, &format!("file:{file_key}"))? {
        None => Ok(None),
        Some(value) if value.is_null() => Ok(None),
        Some(value) => Ok(Some(serde_json::from_value(value)?)),
    }
}

pub fn save_file_record(store: &Store, file_key: &str, record: &FileRecord) -> Result<()> {
    store
        .settings()
        .set(SCOPE, &format!("file:{file_key}"), &serde_json::to_value(record)?)?;
    Ok(())
}

pub fn delete_file_record(store: &Store, file_key: &str) -> Result<()> {
    if let Some(record) = load_file_record(store, file_key)? {
        for id in record.symbol_ids.iter().chain(record.import_ids.iter()) {
            if let Ok(node_id) = NodeId::from_str(id) {
                let _ = store.delete_node(node_id);
            }
        }
        if let Ok(node_id) = NodeId::from_str(&record.node_id) {
            let _ = store.delete_node(node_id);
        }
    }
    store
        .settings()
        .set(SCOPE, &format!("file:{file_key}"), &serde_json::Value::Null)?;
    Ok(())
}

pub fn load_repo_id(store: &Store) -> Result<Option<NodeId>> {
    let Some(value) = store.settings().get(SCOPE, "repository")? else {
        return Ok(None);
    };
    let Some(text) = value.as_str() else {
        return Ok(None);
    };
    NodeId::from_str(text)
        .map(Some)
        .map_err(|err| IndexError::Id(err.to_string()))
}

pub fn save_repo_id(store: &Store, id: NodeId) -> Result<()> {
    store
        .settings()
        .set(SCOPE, "repository", &serde_json::Value::String(id.to_string()))?;
    Ok(())
}

pub fn ensure_edge(store: &Store, from: NodeId, to: NodeId, kind: EdgeKind) -> Result<Edge> {
    if let Some(existing) = store.find_edge(from, to, kind.clone())? {
        return Ok(existing);
    }
    let edge = Edge::new(from, to, kind);
    store.upsert_edge(&edge)?;
    Ok(edge)
}

pub fn upsert_named(store: &Store, kind: NodeKind, name: &str, payload: serde_json::Value) -> Result<Node> {
    if let Some(mut existing) = store.find_node_by_name(kind.clone(), name)? {
        existing.payload = payload;
        existing.touch();
        store.upsert_node(&existing)?;
        return Ok(existing);
    }
    let node = Node::new(kind, Some(name.to_string()), payload);
    store.upsert_node(&node)?;
    Ok(node)
}

pub fn parse_node_id(value: &str) -> Result<NodeId> {
    NodeId::from_str(value).map_err(|err| IndexError::Id(err.to_string()))
}
