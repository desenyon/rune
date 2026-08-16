//! Historical reasoning graph: sessions, decisions, attempts, failures, commits, code, tasks, specs.

use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind};
use rune_graph::{Graph, Path};
use rune_storage::{Result as StorageResult, Store};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub struct ReasoningGraph<'a> {
    store: &'a Store,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhyPath {
    pub path: Path,
    pub explanation: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailedApproach {
    pub failure: Node,
    pub attempts: Vec<Node>,
    pub sessions: Vec<Node>,
    pub explanation: String,
}

impl<'a> ReasoningGraph<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn connect(&self, from: NodeId, to: NodeId, kind: EdgeKind) -> StorageResult<Edge> {
        if let Some(existing) = self.store.find_edge(from, to, kind.clone())? {
            return Ok(existing);
        }
        let edge = Edge::new(from, to, kind);
        self.store.upsert_edge(&edge)?;
        Ok(edge)
    }

    pub fn discussed_in(&self, item: NodeId, session_or_turn: NodeId) -> StorageResult<Edge> {
        self.connect(item, session_or_turn, EdgeKind::DiscussedIn)
    }

    pub fn decided_in(&self, decision: NodeId, session_or_turn: NodeId) -> StorageResult<Edge> {
        self.connect(decision, session_or_turn, EdgeKind::DecidedIn)
    }

    pub fn attempted_in(&self, attempt: NodeId, session_or_turn: NodeId) -> StorageResult<Edge> {
        self.connect(attempt, session_or_turn, EdgeKind::AttemptedIn)
    }

    pub fn failed_in(&self, failure: NodeId, session_or_turn: NodeId) -> StorageResult<Edge> {
        self.connect(failure, session_or_turn, EdgeKind::FailedIn)
    }

    pub fn changed_by(&self, code: NodeId, commit: NodeId) -> StorageResult<Edge> {
        self.connect(code, commit, EdgeKind::ChangedBy)
    }

    pub fn why_path(&self, from: NodeId, to: NodeId) -> StorageResult<Option<WhyPath>> {
        let graph = Graph::new(self.store);
        let Some(path) = graph.trace_path(from, to, 12)? else {
            return Ok(None);
        };
        let mut explanation = Vec::new();
        for (index, edge) in path.edges.iter().enumerate() {
            let from_node = &path.nodes[index];
            let to_node = &path.nodes[index + 1];
            explanation.push(format!(
                "{} `{}` -[{}]-> {} `{}`",
                from_node.kind,
                from_node.name.as_deref().unwrap_or("unnamed"),
                edge.kind,
                to_node.kind,
                to_node.name.as_deref().unwrap_or("unnamed")
            ));
        }
        Ok(Some(WhyPath { path, explanation }))
    }

    pub fn failed_approaches_for(
        &self,
        symbol_or_topic: &str,
    ) -> StorageResult<Vec<FailedApproach>> {
        let needle = symbol_or_topic.to_lowercase();
        let graph = Graph::new(self.store);
        let mut out = Vec::new();
        for failure in self.store.nodes_of_kind(NodeKind::Failure)? {
            if !matches_topic(&failure, &needle)
                && !self.neighbors_match(&graph, failure.id, &needle)?
            {
                continue;
            }
            let mut attempts = Vec::new();
            let mut sessions = Vec::new();
            for neighbor in graph.neighbors(failure.id)? {
                match neighbor.node.kind {
                    NodeKind::Attempt => attempts.push(neighbor.node),
                    NodeKind::Session | NodeKind::Turn => sessions.push(neighbor.node),
                    _ => {
                        if neighbor.edge.kind == EdgeKind::DiscussedIn
                            || neighbor.edge.kind == EdgeKind::FailedIn
                        {
                            if neighbor.node.kind == NodeKind::Session {
                                sessions.push(neighbor.node);
                            }
                        }
                    }
                }
            }
            let explanation = format!(
                "failure `{}` related to `{symbol_or_topic}` ({} attempts, {} sessions/turns)",
                failure.name.as_deref().unwrap_or("unnamed"),
                attempts.len(),
                sessions.len()
            );
            out.push(FailedApproach {
                failure,
                attempts,
                sessions,
                explanation,
            });
        }
        Ok(out)
    }

    fn neighbors_match(&self, graph: &Graph<'_>, id: NodeId, needle: &str) -> StorageResult<bool> {
        for neighbor in graph.neighbors(id)? {
            if matches_topic(&neighbor.node, needle) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn matches_topic(node: &Node, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if node
        .name
        .as_deref()
        .map(|name| name.to_lowercase().contains(needle))
        .unwrap_or(false)
    {
        return true;
    }
    node.search_body().to_lowercase().contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::Node;
    use rune_storage::Store;

    #[test]
    fn why_path_and_failed_approaches() {
        let store = Store::open_in_memory().unwrap();
        let symbol = Node::new(
            NodeKind::Function,
            Some("TokenStore".into()),
            serde_json::json!({"path": "src/token.rs"}),
        );
        let attempt = Node::new(
            NodeKind::Attempt,
            Some("redis lock".into()),
            serde_json::json!({"statement": "try redis lock for TokenStore"}),
        );
        let failure = Node::new(
            NodeKind::Failure,
            Some("non_atomic_rotation".into()),
            serde_json::json!({"statement": "TokenStore rotation still races"}),
        );
        let session = Node::new(
            NodeKind::Session,
            Some("auth-debug".into()),
            serde_json::json!({}),
        );
        let commit = Node::new(
            NodeKind::Commit,
            Some("abc123".into()),
            serde_json::json!({"sha": "abc123"}),
        );
        store.upsert_node(&symbol).unwrap();
        store.upsert_node(&attempt).unwrap();
        store.upsert_node(&failure).unwrap();
        store.upsert_node(&session).unwrap();
        store.upsert_node(&commit).unwrap();
        let history = ReasoningGraph::new(&store);
        history.attempted_in(attempt.id, session.id).unwrap();
        history.failed_in(failure.id, session.id).unwrap();
        history
            .connect(failure.id, attempt.id, EdgeKind::RelatedTo)
            .unwrap();
        history
            .connect(failure.id, symbol.id, EdgeKind::Affects)
            .unwrap();
        history.changed_by(symbol.id, commit.id).unwrap();

        let why = history.why_path(failure.id, commit.id).unwrap().unwrap();
        assert!(why
            .explanation
            .iter()
            .any(|line| line.contains("changed_by")));

        let failed = history.failed_approaches_for("TokenStore").unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].failure.id, failure.id);
    }
}
