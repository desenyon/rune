//! Cross-agent handoff (S021) and handoff compiler (S022).

use rune_context_compiler::{
    CompileRequest, ContextCapsule, ContextCompiler, Retrievers, TaskType,
};
use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind, Timestamp};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HandoffError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error(transparent)]
    Compiler(#[from] rune_context_compiler::CompilerError),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, HandoffError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffMode {
    Full,
    Balanced,
    Compact,
    Custom,
}

impl HandoffMode {
    pub fn token_budget(self, custom: Option<usize>) -> usize {
        match self {
            Self::Full => 24_000,
            Self::Balanced => 8_000,
            Self::Compact => 2_000,
            Self::Custom => custom.unwrap_or(8_000),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Handoff {
    pub id: NodeId,
    pub source_agent: String,
    pub target_agent: String,
    pub goal: String,
    pub current_state: String,
    pub task: Option<NodeId>,
    pub working_tree_state: String,
    pub diff: Option<String>,
    pub relevant_files: Vec<NodeId>,
    pub relevant_symbols: Vec<NodeId>,
    pub decisions: Vec<NodeId>,
    pub failed_attempts: Vec<NodeId>,
    pub unresolved_questions: Vec<String>,
    pub remaining_work: Vec<String>,
    pub tests: Vec<NodeId>,
    pub constraints: Vec<NodeId>,
    pub memories: Vec<NodeId>,
    pub historical_context: Vec<NodeId>,
    pub environment: serde_json::Value,
    pub recommended_next_actions: Vec<String>,
    pub source_session: NodeId,
    pub target_session: Option<NodeId>,
    pub created_at: Timestamp,
}

impl Handoff {
    pub fn into_node(&self) -> Node {
        let mut node = Node::new(
            NodeKind::Handoff,
            Some(format!("{} → {}", self.source_agent, self.target_agent)),
            serde_json::to_value(self).unwrap_or(serde_json::json!({})),
        );
        node.id = self.id;
        node
    }

    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.source_agent.is_empty() {
            missing.push("source_agent");
        }
        if self.target_agent.is_empty() {
            missing.push("target_agent");
        }
        if self.goal.is_empty() {
            missing.push("goal");
        }
        if self.current_state.is_empty() {
            missing.push("current_state");
        }
        missing
    }
}

/// Mutable package that can be inspected and edited before transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandoffPackage {
    pub handoff: Handoff,
    pub capsule: ContextCapsule,
    pub mode: HandoffMode,
    pub edited: bool,
}

impl HandoffPackage {
    pub fn set_goal(&mut self, goal: impl Into<String>) {
        self.handoff.goal = goal.into();
        self.capsule.goal = self.handoff.goal.clone();
        self.edited = true;
    }

    pub fn add_remaining_work(&mut self, item: impl Into<String>) {
        self.handoff.remaining_work.push(item.into());
        self.edited = true;
    }

    pub fn set_target_session(&mut self, session: NodeId) {
        self.handoff.target_session = Some(session);
        self.edited = true;
    }
}

pub struct HandoffCompiler<'a> {
    store: &'a Store,
}

impl<'a> HandoffCompiler<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn compile(
        &self,
        source_session: Node,
        source_agent: impl Into<String>,
        target_agent: impl Into<String>,
        goal: impl Into<String>,
        mode: HandoffMode,
        custom_budget: Option<usize>,
        retrievers: &Retrievers<'_>,
    ) -> Result<HandoffPackage> {
        if source_session.kind != NodeKind::Session {
            return Err(HandoffError::Message(
                "source must be a Session node for lineage session → handoff → session".into(),
            ));
        }
        let goal = goal.into();
        let source_agent = source_agent.into();
        let target_agent = target_agent.into();
        let mut request = CompileRequest::new(&goal, mode.token_budget(custom_budget));
        request.agent = Some(target_agent.clone());
        request.task_type = Some(TaskType::Implementation);
        request.persist = false;
        let compiler = ContextCompiler::new(self.store);
        let compiled = compiler.compile(request, retrievers)?;
        let capsule = compiled.capsule;

        let ids_of = |kind: NodeKind| -> Vec<NodeId> {
            capsule
                .included
                .iter()
                .filter(|i| i.kind == kind)
                .map(|i| i.id)
                .collect()
        };

        let handoff = Handoff {
            id: NodeId::generate(),
            source_agent,
            target_agent,
            goal: goal.clone(),
            current_state: capsule.current_state.clone(),
            task: capsule.task,
            working_tree_state: capsule
                .working_tree
                .iter()
                .filter_map(|i| i.name.clone())
                .collect::<Vec<_>>()
                .join(", "),
            diff: None,
            relevant_files: ids_of(NodeKind::File),
            relevant_symbols: capsule
                .relevant_code
                .iter()
                .filter(|i| i.kind != NodeKind::File)
                .map(|i| i.id)
                .collect(),
            decisions: ids_of(NodeKind::Decision),
            failed_attempts: capsule.failed_attempts.iter().map(|i| i.id).collect(),
            unresolved_questions: capsule.open_questions.clone(),
            remaining_work: capsule.recommended_next_actions.clone(),
            tests: ids_of(NodeKind::Test),
            constraints: ids_of(NodeKind::Constraint),
            memories: ids_of(NodeKind::Memory),
            historical_context: ids_of(NodeKind::Session),
            environment: serde_json::json!({"cwd": self.store.path().display().to_string()}),
            recommended_next_actions: capsule.recommended_next_actions.clone(),
            source_session: source_session.id,
            target_session: None,
            created_at: Timestamp::now(),
        };

        Ok(HandoffPackage {
            handoff,
            capsule,
            mode,
            edited: false,
        })
    }

    /// Persist handoff and lineage: session A --handed_from--> handoff --handed_to--> session B.
    pub fn transfer(&self, package: &HandoffPackage, target_session: &Node) -> Result<Handoff> {
        if target_session.kind != NodeKind::Session {
            return Err(HandoffError::Message(
                "target must be a Session node".into(),
            ));
        }
        let mut handoff = package.handoff.clone();
        handoff.target_session = Some(target_session.id);
        let node = handoff.into_node();
        self.store.upsert_node(&node)?;
        let from = Edge::new(handoff.source_session, handoff.id, EdgeKind::HandedFrom);
        let to = Edge::new(handoff.id, target_session.id, EdgeKind::HandedTo);
        self.store.upsert_edge(&from)?;
        self.store.upsert_edge(&to)?;
        Ok(handoff)
    }

    pub fn lineage(&self, handoff_id: NodeId) -> Result<(Option<Node>, Node, Option<Node>)> {
        let handoff = self.store.get_node(handoff_id)?;
        let source = self
            .store
            .edges_to(handoff_id)?
            .into_iter()
            .find(|e| e.kind == EdgeKind::HandedFrom)
            .and_then(|e| self.store.get_node(e.from).ok());
        let target = self
            .store
            .edges_from_kind(handoff_id, EdgeKind::HandedTo)?
            .into_iter()
            .next()
            .and_then(|e| self.store.get_node(e.to).ok());
        Ok((source, handoff, target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_context_compiler::EmptyRetriever;
    use rune_core::Node;

    #[test]
    fn handoff_lineage_session_a_to_b() {
        let store = Store::open_in_memory().unwrap();
        let session_a = Node::new(
            NodeKind::Session,
            Some("claude".into()),
            serde_json::json!({"provider": "claude"}),
        );
        let session_b = Node::new(
            NodeKind::Session,
            Some("codex".into()),
            serde_json::json!({"provider": "codex"}),
        );
        store.upsert_node(&session_a).unwrap();
        store.upsert_node(&session_b).unwrap();
        let file = Node::new(
            NodeKind::File,
            Some("auth.rs".into()),
            serde_json::json!({"purpose": "authentication"}),
        );
        store.upsert_node(&file).unwrap();

        let compiler = HandoffCompiler::new(&store);
        let empty = EmptyRetriever;
        let retrievers = Retrievers::empty(&empty);
        let mut package = compiler
            .compile(
                session_a.clone(),
                "claude",
                "codex",
                "continue authentication work",
                HandoffMode::Compact,
                None,
                &retrievers,
            )
            .unwrap();
        package.add_remaining_work("write rotation tests");
        package.set_goal("fix refresh token race");
        assert!(package.edited);
        assert!(package.handoff.missing_fields().is_empty());

        let transferred = compiler.transfer(&package, &session_b).unwrap();
        let (from, node, to) = compiler.lineage(transferred.id).unwrap();
        assert_eq!(from.unwrap().id, session_a.id);
        assert_eq!(node.kind, NodeKind::Handoff);
        assert_eq!(to.unwrap().id, session_b.id);
        assert_eq!(transferred.source_session, session_a.id);
        assert_eq!(transferred.target_session, Some(session_b.id));
    }
}
