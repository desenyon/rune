use rune_core::{EdgeKind, NodeId};
use rune_storage::Store;
use std::collections::HashSet;

use crate::error::{Result, TaskError};

pub fn successors(store: &Store, id: NodeId) -> Result<Vec<NodeId>> {
    let mut out = Vec::new();
    for edge in store.edges_from_kind(id, EdgeKind::DependsOn)? {
        out.push(edge.to);
    }
    for edge in store.edges_from_kind(id, EdgeKind::Blocks)? {
        out.push(edge.to);
    }
    for edge in store.edges_from_kind(id, EdgeKind::BlockedBy)? {
        out.push(edge.to);
    }
    Ok(out)
}

/// Returns a cycle path if one exists, including the closing node.
pub fn find_cycle(store: &Store, roots: &[NodeId]) -> Result<Option<Vec<NodeId>>> {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    for root in roots {
        if let Some(cycle) = dfs(store, *root, &mut visiting, &mut visited, &mut stack)? {
            return Ok(Some(cycle));
        }
    }
    Ok(None)
}

pub fn would_cycle(store: &Store, from: NodeId, to: NodeId) -> Result<Option<Vec<NodeId>>> {
    if from == to {
        return Ok(Some(vec![from, to]));
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut stack = vec![from];
    dfs(store, to, &mut visiting, &mut visited, &mut stack)
}

fn dfs(
    store: &Store,
    current: NodeId,
    visiting: &mut HashSet<NodeId>,
    visited: &mut HashSet<NodeId>,
    stack: &mut Vec<NodeId>,
) -> Result<Option<Vec<NodeId>>> {
    if visiting.contains(&current) {
        let start = stack
            .iter()
            .position(|id| *id == current)
            .ok_or_else(|| TaskError::msg("cycle stack missing back-edge node"))?;
        let mut cycle = stack[start..].to_vec();
        cycle.push(current);
        return Ok(Some(cycle));
    }
    if visited.contains(&current) {
        return Ok(None);
    }
    visiting.insert(current);
    stack.push(current);
    for next in successors(store, current)? {
        if let Some(cycle) = dfs(store, next, visiting, visited, stack)? {
            return Ok(Some(cycle));
        }
    }
    stack.pop();
    visiting.remove(&current);
    visited.insert(current);
    Ok(None)
}
