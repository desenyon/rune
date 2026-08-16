//! Graph operations over the canonical store.

use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind};
use rune_storage::{Result, Store};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Neighbor {
    pub edge: Edge,
    pub node: Node,
    pub outgoing: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Path {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Default)]
pub struct ExpandFilter {
    pub node_kinds: Vec<NodeKind>,
    pub edge_kinds: Vec<EdgeKind>,
    pub depth: usize,
}

impl ExpandFilter {
    pub fn depth(depth: usize) -> Self {
        Self {
            depth,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy)]
pub struct Graph<'a> {
    store: &'a Store,
}

impl<'a> Graph<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn neighbors(&self, id: NodeId) -> Result<Vec<Neighbor>> {
        let mut out = Vec::new();
        for edge in self.store.edges_from(id)? {
            let node = self.store.get_node(edge.to)?;
            out.push(Neighbor {
                edge,
                node,
                outgoing: true,
            });
        }
        for edge in self.store.edges_to(id)? {
            let node = self.store.get_node(edge.from)?;
            out.push(Neighbor {
                edge,
                node,
                outgoing: false,
            });
        }
        Ok(out)
    }

    pub fn expand(&self, id: NodeId, filter: ExpandFilter) -> Result<Vec<Node>> {
        let depth = filter.depth.max(1);
        let mut seen = HashSet::from([id]);
        let mut frontier = vec![id];
        let mut nodes = vec![self.store.get_node(id)?];
        for _ in 0..depth {
            let mut next = Vec::new();
            for current in frontier {
                for neighbor in self.neighbors(current)? {
                    if !filter.edge_kinds.is_empty()
                        && !filter.edge_kinds.iter().any(|kind| kind == &neighbor.edge.kind)
                    {
                        continue;
                    }
                    if !filter.node_kinds.is_empty()
                        && !filter
                            .node_kinds
                            .iter()
                            .any(|kind| kind == &neighbor.node.kind)
                    {
                        continue;
                    }
                    if seen.insert(neighbor.node.id) {
                        next.push(neighbor.node.id);
                        nodes.push(neighbor.node);
                    }
                }
            }
            frontier = next;
        }
        Ok(nodes)
    }

    pub fn trace_path(&self, from: NodeId, to: NodeId, max_depth: usize) -> Result<Option<Path>> {
        let mut queue = VecDeque::from([from]);
        let mut seen = HashSet::from([from]);
        let mut prev: std::collections::HashMap<NodeId, (NodeId, Edge)> =
            std::collections::HashMap::new();
        let mut depth = std::collections::HashMap::from([(from, 0usize)]);
        while let Some(current) = queue.pop_front() {
            if current == to {
                return self.rebuild_path(from, to, &prev);
            }
            let current_depth = depth[&current];
            if current_depth >= max_depth {
                continue;
            }
            for neighbor in self.neighbors(current)? {
                if seen.insert(neighbor.node.id) {
                    prev.insert(neighbor.node.id, (current, neighbor.edge));
                    depth.insert(neighbor.node.id, current_depth + 1);
                    queue.push_back(neighbor.node.id);
                }
            }
        }
        Ok(None)
    }

    pub fn callers(&self, id: NodeId) -> Result<Vec<Neighbor>> {
        Ok(self
            .neighbors(id)?
            .into_iter()
            .filter(|neighbor| neighbor.edge.kind == EdgeKind::Calls && !neighbor.outgoing)
            .collect())
    }

    pub fn callees(&self, id: NodeId) -> Result<Vec<Neighbor>> {
        Ok(self
            .neighbors(id)?
            .into_iter()
            .filter(|neighbor| neighbor.edge.kind == EdgeKind::Calls && neighbor.outgoing)
            .collect())
    }

    pub fn implementations(&self, id: NodeId) -> Result<Vec<Neighbor>> {
        Ok(self
            .neighbors(id)?
            .into_iter()
            .filter(|neighbor| {
                matches!(
                    neighbor.edge.kind,
                    EdgeKind::Implements | EdgeKind::Extends | EdgeKind::Inherits
                )
            })
            .collect())
    }

    pub fn tests_for(&self, id: NodeId) -> Result<Vec<Neighbor>> {
        Ok(self
            .neighbors(id)?
            .into_iter()
            .filter(|neighbor| neighbor.edge.kind == EdgeKind::Tests)
            .collect())
    }

    pub fn related_of_kind(&self, id: NodeId, kinds: &[NodeKind]) -> Result<Vec<Node>> {
        Ok(self
            .neighbors(id)?
            .into_iter()
            .map(|neighbor| neighbor.node)
            .filter(|node| kinds.iter().any(|kind| kind == &node.kind))
            .collect())
    }

    fn rebuild_path(
        &self,
        from: NodeId,
        to: NodeId,
        prev: &std::collections::HashMap<NodeId, (NodeId, Edge)>,
    ) -> Result<Option<Path>> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut cursor = to;
        nodes.push(self.store.get_node(cursor)?);
        while cursor != from {
            let Some((parent, edge)) = prev.get(&cursor) else {
                return Ok(None);
            };
            edges.push(edge.clone());
            cursor = *parent;
            nodes.push(self.store.get_node(cursor)?);
        }
        nodes.reverse();
        edges.reverse();
        Ok(Some(Path { nodes, edges }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::Node;
    use rune_storage::Store;

    #[test]
    fn traces_path_across_contains_and_defines() {
        let store = Store::open_in_memory().unwrap();
        let repo = Node::new(NodeKind::Repository, Some("repo".into()), serde_json::json!({}));
        let file = Node::new(NodeKind::File, Some("lib.rs".into()), serde_json::json!({}));
        let symbol = Node::new(NodeKind::Function, Some("open".into()), serde_json::json!({}));
        store.upsert_node(&repo).unwrap();
        store.upsert_node(&file).unwrap();
        store.upsert_node(&symbol).unwrap();
        store
            .upsert_edge(&rune_core::Edge::new(repo.id, file.id, EdgeKind::Contains))
            .unwrap();
        store
            .upsert_edge(&rune_core::Edge::new(file.id, symbol.id, EdgeKind::Defines))
            .unwrap();
        let graph = Graph::new(&store);
        let path = graph.trace_path(repo.id, symbol.id, 4).unwrap().unwrap();
        assert_eq!(path.nodes.len(), 3);
        assert_eq!(path.edges.len(), 2);
    }

    #[test]
    fn callers_and_callees_follow_call_edges() {
        let store = Store::open_in_memory().unwrap();
        let caller = Node::new(NodeKind::Function, Some("run".into()), serde_json::json!({}));
        let callee = Node::new(NodeKind::Function, Some("open".into()), serde_json::json!({}));
        store.upsert_node(&caller).unwrap();
        store.upsert_node(&callee).unwrap();
        store
            .upsert_edge(&rune_core::Edge::new(caller.id, callee.id, EdgeKind::Calls).with_confidence(0.8))
            .unwrap();
        let graph = Graph::new(&store);
        assert_eq!(graph.callees(caller.id).unwrap().len(), 1);
        assert_eq!(graph.callers(callee.id).unwrap().len(), 1);
        assert!(graph.callers(caller.id).unwrap().is_empty());
    }
}
