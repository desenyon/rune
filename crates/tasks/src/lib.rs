//! Dependency-aware tasks with cycle detection and parallelization analysis.

mod cycle;
mod error;
mod model;
mod parallel;
mod store;

pub use cycle::{find_cycle, would_cycle};
pub use error::{Result, TaskError};
pub use model::{Task, TaskStatus};
pub use parallel::{Parallelization, ParallelizationReport};
pub use store::TaskStore;

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::{Node, NodeKind};
    use rune_storage::Store;

    fn file(store: &Store, name: &str) -> Node {
        let node = Node::new(NodeKind::File, Some(name.into()), serde_json::json!({}));
        store.upsert_node(&node).unwrap();
        node
    }

    #[test]
    fn task_cycle_detected() {
        let store = Store::open_in_memory().unwrap();
        let tasks = TaskStore::new(&store);
        let a = tasks.create(Task::new("A", "first")).unwrap();
        let b = tasks.create(Task::new("B", "second")).unwrap();
        tasks.add_dependency(a.id, b.id).unwrap();
        let err = tasks.add_dependency(b.id, a.id).unwrap_err();
        match err {
            TaskError::Cycle(path) => {
                assert!(path.contains(&a.id.to_string()));
                assert!(path.contains(&b.id.to_string()));
            }
            other => panic!("expected cycle, got {other}"),
        }
    }

    #[test]
    fn parallelization_refuses_conflict_free_claim_when_same_file_listed() {
        let store = Store::open_in_memory().unwrap();
        let shared = file(&store, "auth.rs");
        let other = file(&store, "db.rs");
        let tasks = TaskStore::new(&store);
        let mut left = Task::new("rotate tokens", "change auth.rs");
        left.affected_files = vec![shared.id];
        let mut right = Task::new("rewrite sessions", "also auth.rs");
        right.affected_files = vec![shared.id, other.id];
        let left = tasks.create(left).unwrap();
        let right = tasks.create(right).unwrap();
        let report = Parallelization::new(&store).analyze(left.id, right.id).unwrap();
        assert!(!report.conflict_free);
        assert!(report.overlapping_files.contains(&shared.id));
        assert!(report.confidence >= 0.9);
    }

    #[test]
    fn parallelization_refuses_without_resource_evidence() {
        let store = Store::open_in_memory().unwrap();
        let tasks = TaskStore::new(&store);
        let left = tasks.create(Task::new("A", "no files")).unwrap();
        let right = tasks.create(Task::new("B", "also none")).unwrap();
        let report = Parallelization::new(&store).analyze(left.id, right.id).unwrap();
        assert!(!report.conflict_free);
        assert_eq!(report.confidence, 0.0);
        assert!(report.explanation.contains("refusing conflict-free claim"));
    }

    #[test]
    fn actionable_requires_complete_dependencies() {
        let store = Store::open_in_memory().unwrap();
        let tasks = TaskStore::new(&store);
        let dep = tasks.create(Task::new("dep", "must finish")).unwrap();
        let mut child = Task::new("child", "blocked on dep");
        child.dependencies = vec![dep.id];
        let child = tasks.create(child).unwrap();
        let ready = tasks.actionable().unwrap();
        assert!(ready.iter().any(|task| task.id == dep.id));
        assert!(ready.iter().all(|task| task.id != child.id));
        tasks.set_status(dep.id, TaskStatus::Complete).unwrap();
        let ready = tasks.actionable().unwrap();
        assert!(ready.iter().any(|task| task.id == child.id));
    }
}
