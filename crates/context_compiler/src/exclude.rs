use rune_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PinSet {
    pub ids: BTreeSet<NodeId>,
}

impl PinSet {
    pub fn pin(&mut self, id: NodeId) {
        self.ids.insert(id);
    }

    pub fn unpin(&mut self, id: &NodeId) {
        self.ids.remove(id);
    }

    pub fn contains(&self, id: &NodeId) -> bool {
        self.ids.contains(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionScope {
    Session,
    Task,
    Workspace,
    User,
}

impl ExclusionScope {
    pub fn is_permanent_preference(self) -> bool {
        matches!(self, Self::Workspace | Self::User)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exclusion {
    pub object_id: NodeId,
    pub scope: ExclusionScope,
    pub reason: Option<String>,
}

impl Exclusion {
    pub fn session(id: NodeId) -> Self {
        Self {
            object_id: id,
            scope: ExclusionScope::Session,
            reason: None,
        }
    }
}
