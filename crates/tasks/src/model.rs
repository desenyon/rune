use rune_core::{Node, NodeId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Ready,
    Active,
    Blocked,
    Failed,
    Review,
    Complete,
}

impl TaskStatus {
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }

    pub fn is_open(self) -> bool {
        !matches!(self, Self::Complete | Self::Failed)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: NodeId,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: i32,
    pub dependencies: Vec<NodeId>,
    pub blockers: Vec<NodeId>,
    pub affected_files: Vec<NodeId>,
    pub affected_symbols: Vec<NodeId>,
    pub affected_schemas: Vec<NodeId>,
    pub spec_links: Vec<NodeId>,
    pub assigned_agent: Option<NodeId>,
    pub worktree: Option<NodeId>,
    pub branch: Option<String>,
    pub sessions: Vec<NodeId>,
    pub commits: Vec<NodeId>,
    pub tests: Vec<NodeId>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Task {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Timestamp::now();
        Self {
            id: NodeId::generate(),
            title: title.into(),
            description: description.into(),
            status: TaskStatus::Ready,
            priority: 0,
            dependencies: Vec::new(),
            blockers: Vec::new(),
            affected_files: Vec::new(),
            affected_symbols: Vec::new(),
            affected_schemas: Vec::new(),
            spec_links: Vec::new(),
            assigned_agent: None,
            worktree: None,
            branch: None,
            sessions: Vec::new(),
            commits: Vec::new(),
            tests: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskPayload {
    pub description: String,
    pub status: TaskStatus,
    pub priority: i32,
    pub affected_files: Vec<NodeId>,
    pub affected_symbols: Vec<NodeId>,
    pub affected_schemas: Vec<NodeId>,
    pub spec_links: Vec<NodeId>,
    pub assigned_agent: Option<NodeId>,
    pub worktree: Option<NodeId>,
    pub branch: Option<String>,
    pub sessions: Vec<NodeId>,
    pub commits: Vec<NodeId>,
    pub tests: Vec<NodeId>,
}

pub fn payload_from_task(task: &Task) -> TaskPayload {
    TaskPayload {
        description: task.description.clone(),
        status: task.status,
        priority: task.priority,
        affected_files: task.affected_files.clone(),
        affected_symbols: task.affected_symbols.clone(),
        affected_schemas: task.affected_schemas.clone(),
        spec_links: task.spec_links.clone(),
        assigned_agent: task.assigned_agent,
        worktree: task.worktree,
        branch: task.branch.clone(),
        sessions: task.sessions.clone(),
        commits: task.commits.clone(),
        tests: task.tests.clone(),
    }
}

pub fn task_from_node(
    node: &Node,
    dependencies: Vec<NodeId>,
    blockers: Vec<NodeId>,
) -> Result<Task, serde_json::Error> {
    let payload: TaskPayload = serde_json::from_value(node.payload.clone())?;
    Ok(Task {
        id: node.id,
        title: node.name.clone().unwrap_or_default(),
        description: payload.description,
        status: payload.status,
        priority: payload.priority,
        dependencies,
        blockers,
        affected_files: payload.affected_files,
        affected_symbols: payload.affected_symbols,
        affected_schemas: payload.affected_schemas,
        spec_links: payload.spec_links,
        assigned_agent: payload.assigned_agent,
        worktree: payload.worktree,
        branch: payload.branch,
        sessions: payload.sessions,
        commits: payload.commits,
        tests: payload.tests,
        created_at: node.created_at,
        updated_at: node.updated_at,
    })
}
