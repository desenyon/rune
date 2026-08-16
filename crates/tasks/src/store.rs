use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind, Timestamp};
use rune_security::UntrustedContent;
use rune_storage::Store;

use crate::cycle::{find_cycle, would_cycle};
use crate::error::{Result, TaskError};
use crate::model::{payload_from_task, task_from_node, Task, TaskStatus};

pub struct TaskStore<'a> {
    store: &'a Store,
}

impl<'a> TaskStore<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn create(&self, mut task: Task) -> Result<Task> {
        self.validate(&task)?;
        let title = UntrustedContent::wrap("task.title", &task.title);
        let description = UntrustedContent::wrap("task.description", &task.description);
        let _ = title.as_instruction();
        let _ = description.as_instruction();
        task.title = title.body;
        task.description = description.body;
        task.updated_at = Timestamp::now();
        self.write_node(&task)?;
        for dep in task.dependencies.clone() {
            self.add_dependency(task.id, dep)?;
        }
        for blocker in task.blockers.clone() {
            self.add_blocker(task.id, blocker)?;
        }
        self.link_related(&task)?;
        self.get(task.id)
    }

    pub fn get(&self, id: NodeId) -> Result<Task> {
        let node = self.store.get_node(id)?;
        if node.kind != NodeKind::Task {
            return Err(TaskError::invalid(format!("{id} is not a task")));
        }
        let dependencies = self
            .store
            .edges_from_kind(id, EdgeKind::DependsOn)?
            .into_iter()
            .map(|edge| edge.to)
            .collect();
        let blockers = self
            .store
            .edges_from_kind(id, EdgeKind::BlockedBy)?
            .into_iter()
            .map(|edge| edge.to)
            .collect();
        task_from_node(&node, dependencies, blockers).map_err(|err| TaskError::msg(err.to_string()))
    }

    pub fn list(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        for node in self.store.nodes_of_kind(NodeKind::Task)? {
            tasks.push(self.get(node.id)?);
        }
        Ok(tasks)
    }

    pub fn set_status(&self, id: NodeId, status: TaskStatus) -> Result<Task> {
        let mut task = self.get(id)?;
        task.status = status;
        task.updated_at = Timestamp::now();
        self.write_node(&task)?;
        self.get(id)
    }

    pub fn add_dependency(&self, task: NodeId, depends_on: NodeId) -> Result<()> {
        self.ensure_task(depends_on)?;
        if let Some(cycle) = would_cycle(self.store, task, depends_on)? {
            return Err(TaskError::cycle(&cycle));
        }
        if self
            .store
            .find_edge(task, depends_on, EdgeKind::DependsOn)?
            .is_none()
        {
            self.store
                .upsert_edge(&Edge::new(task, depends_on, EdgeKind::DependsOn))?;
        }
        if let Some(cycle) = find_cycle(self.store, &[task, depends_on])? {
            return Err(TaskError::cycle(&cycle));
        }
        Ok(())
    }

    pub fn add_blocker(&self, task: NodeId, blocked_by: NodeId) -> Result<()> {
        self.ensure_task(blocked_by)?;
        if let Some(cycle) = would_cycle(self.store, task, blocked_by)? {
            return Err(TaskError::cycle(&cycle));
        }
        if self
            .store
            .find_edge(task, blocked_by, EdgeKind::BlockedBy)?
            .is_none()
        {
            self.store
                .upsert_edge(&Edge::new(task, blocked_by, EdgeKind::BlockedBy))?;
        }
        if self
            .store
            .find_edge(blocked_by, task, EdgeKind::Blocks)?
            .is_none()
        {
            self.store
                .upsert_edge(&Edge::new(blocked_by, task, EdgeKind::Blocks))?;
        }
        if let Some(cycle) = find_cycle(self.store, &[task, blocked_by])? {
            return Err(TaskError::cycle(&cycle));
        }
        Ok(())
    }

    /// Tasks whose dependencies are complete and that are not blocked.
    pub fn actionable(&self) -> Result<Vec<Task>> {
        let tasks = self.list()?;
        let mut by_id = std::collections::HashMap::new();
        for task in &tasks {
            by_id.insert(task.id, task);
        }
        let mut ready = Vec::new();
        for task in &tasks {
            if self.is_actionable(task, &by_id)? {
                ready.push(task.clone());
            }
        }
        ready.sort_by_key(|task| std::cmp::Reverse(task.priority));
        Ok(ready)
    }

    pub fn is_actionable(
        &self,
        task: &Task,
        by_id: &std::collections::HashMap<NodeId, &Task>,
    ) -> Result<bool> {
        if !task.status.is_open() || task.status == TaskStatus::Blocked {
            return Ok(false);
        }
        for dep in &task.dependencies {
            let Some(other) = by_id.get(dep) else {
                return Err(TaskError::NotFound(dep.to_string()));
            };
            if !other.status.is_complete() {
                return Ok(false);
            }
        }
        for blocker in &task.blockers {
            let Some(other) = by_id.get(blocker) else {
                return Err(TaskError::NotFound(blocker.to_string()));
            };
            if !other.status.is_complete() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn write_node(&self, task: &Task) -> Result<()> {
        let payload =
            serde_json::to_value(payload_from_task(task)).map_err(|err| TaskError::msg(err.to_string()))?;
        let mut node = Node::new(NodeKind::Task, Some(task.title.clone()), payload);
        node.id = task.id;
        node.created_at = task.created_at;
        node.updated_at = task.updated_at;
        self.store.upsert_node(&node)?;
        Ok(())
    }

    fn link_related(&self, task: &Task) -> Result<()> {
        for file in &task.affected_files {
            self.ensure_exists(*file)?;
            upsert_edge(self.store, task.id, *file, EdgeKind::Affects)?;
        }
        for symbol in &task.affected_symbols {
            self.ensure_exists(*symbol)?;
            upsert_edge(self.store, task.id, *symbol, EdgeKind::Affects)?;
        }
        for schema in &task.affected_schemas {
            self.ensure_exists(*schema)?;
            upsert_edge(self.store, task.id, *schema, EdgeKind::Affects)?;
        }
        for spec in &task.spec_links {
            self.ensure_exists(*spec)?;
            upsert_edge(self.store, task.id, *spec, EdgeKind::ImplementsSpec)?;
        }
        if let Some(agent) = task.assigned_agent {
            self.ensure_exists(agent)?;
            upsert_edge(self.store, task.id, agent, EdgeKind::AssignedTo)?;
        }
        Ok(())
    }

    fn validate(&self, task: &Task) -> Result<()> {
        if task.title.trim().is_empty() {
            return Err(TaskError::invalid("task title must not be empty"));
        }
        Ok(())
    }

    fn ensure_task(&self, id: NodeId) -> Result<()> {
        let node = self.store.get_node(id)?;
        if node.kind != NodeKind::Task {
            return Err(TaskError::invalid(format!("{id} is not a task")));
        }
        Ok(())
    }

    fn ensure_exists(&self, id: NodeId) -> Result<()> {
        self.store.get_node(id)?;
        Ok(())
    }
}

fn upsert_edge(store: &Store, from: NodeId, to: NodeId, kind: EdgeKind) -> Result<()> {
    if store.find_edge(from, to, kind.clone())?.is_none() {
        store.upsert_edge(&Edge::new(from, to, kind))?;
    }
    Ok(())
}
