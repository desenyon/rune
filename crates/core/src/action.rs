use crate::id::NodeId;
use serde::{Deserialize, Serialize};

/// Semantic actions bound by the command palette and keybindings.
/// Components bind to actions, not raw keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Open,
    Inspect,
    SearchReferences,
    ShowHistory,
    CompileContext,
    AssignAgent,
    Handoff,
    RunTests,
    OpenWorktree,
    Compare,
    Export,
    Archive,
    Pin,
    Unpin,
    Exclude,
    Include,
    VerifyMemory,
    InvalidateMemory,
    RepairGraph,
    CommandPalette,
    FocusSearch,
    Quit,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub kind: ActionKind,
    pub target: Option<NodeId>,
    pub payload: serde_json::Value,
}

impl Action {
    pub fn new(kind: ActionKind) -> Self {
        Self {
            kind,
            target: None,
            payload: serde_json::Value::Null,
        }
    }

    pub fn on(mut self, target: NodeId) -> Self {
        self.target = Some(target);
        self
    }
}
