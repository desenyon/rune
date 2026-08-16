use crate::error::{Result, WorktreeError};
use crate::git::{git_output, git_status_ok, is_git_repo};
use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const SCOPE: &str = "worktrees";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateWorktree {
    pub path: PathBuf,
    pub branch: String,
    pub create_branch: bool,
    pub base_commit: Option<String>,
    pub task: Option<String>,
    pub agent: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub task: Option<String>,
    pub agent: Option<String>,
    pub base_commit: Option<String>,
    pub current_commit: Option<String>,
    pub working_state: WorkingState,
    pub bare: bool,
    pub detached: bool,
    pub node_id: Option<NodeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingState {
    Clean,
    Dirty,
    Missing,
}

#[derive(Clone, Debug)]
pub struct StaleCriteria {
    pub idle_for: Duration,
    pub no_new_commits: bool,
}

impl Default for StaleCriteria {
    fn default() -> Self {
        Self {
            idle_for: Duration::from_secs(14 * 24 * 60 * 60),
            no_new_commits: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaleWorktree {
    pub info: WorktreeInfo,
    pub reasons: Vec<String>,
}

pub struct WorktreeManager {
    store: Store,
    repo: PathBuf,
}

impl WorktreeManager {
    pub fn new(store: Store, repo: impl Into<PathBuf>) -> Result<Self> {
        let repo = repo.into();
        if !is_git_repo(&repo) {
            return Err(WorktreeError::NotARepository(repo));
        }
        Ok(Self {
            store,
            repo: repo.canonicalize().unwrap_or(repo),
        })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn repo(&self) -> &Path {
        &self.repo
    }

    pub fn create(&self, request: CreateWorktree) -> Result<WorktreeInfo> {
        let mut args = vec!["worktree".to_string(), "add".to_string()];
        if request.create_branch {
            args.push("-b".into());
            args.push(request.branch.clone());
        }
        args.push(request.path.to_string_lossy().into_owned());
        if let Some(base) = &request.base_commit {
            args.push(base.clone());
        } else if !request.create_branch {
            args.push(request.branch.clone());
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        git_status_ok(&self.repo, &arg_refs)?;
        let head = git_output(&request.path, &["rev-parse", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string());
        let base = request.base_commit.clone().or_else(|| head.clone());
        let info = WorktreeInfo {
            path: request.path.clone(),
            branch: Some(request.branch.clone()),
            head: head.clone(),
            task: request.task.clone(),
            agent: request.agent.clone(),
            base_commit: base,
            current_commit: head,
            working_state: self.working_state(&request.path),
            bare: false,
            detached: false,
            node_id: None,
        };
        let stored = self.persist(&info)?;
        Ok(stored)
    }

    pub fn list(&self) -> Result<Vec<WorktreeInfo>> {
        let raw = git_output(&self.repo, &["worktree", "list", "--porcelain"])?;
        let mut out = Vec::new();
        for listed in parse_porcelain(&raw) {
            let meta = self.load_meta(&listed.path)?;
            let current = listed.head.clone();
            out.push(WorktreeInfo {
                working_state: self.working_state(&listed.path),
                task: meta.as_ref().and_then(|m| m.task.clone()),
                agent: meta.as_ref().and_then(|m| m.agent.clone()),
                base_commit: meta.as_ref().and_then(|m| m.base_commit.clone()),
                current_commit: current.clone(),
                node_id: meta.as_ref().and_then(|m| m.node_id.parse().ok()),
                path: listed.path,
                branch: listed.branch,
                head: listed.head,
                bare: listed.bare,
                detached: listed.detached,
            });
        }
        Ok(out)
    }

    pub fn inspect(&self, path: &Path) -> Result<WorktreeInfo> {
        self.list()?
            .into_iter()
            .find(|wt| wt.path == path || wt.path == path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
            .ok_or_else(|| WorktreeError::Message(format!("worktree not found: {}", path.display())))
    }

    /// Delete a worktree. Requires `confirm: true`. Never deletes user work otherwise.
    pub fn remove(&self, path: &Path, confirm: bool, force: bool) -> Result<()> {
        if !confirm {
            return Err(WorktreeError::DeleteRequiresConfirm {
                path: path.to_path_buf(),
            });
        }
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        let path_str = path.to_string_lossy().into_owned();
        args.push(&path_str);
        git_status_ok(&self.repo, &args)?;
        self.store.settings().set(
            SCOPE,
            &meta_key(path),
            &serde_json::Value::Null,
        )?;
        if let Some(existing) = self.store.find_node_by_name(NodeKind::Worktree, &path.to_string_lossy())? {
            let _ = self.store.delete_node(existing.id);
        }
        Ok(())
    }

    pub fn detect_stale(&self, criteria: &StaleCriteria) -> Result<Vec<StaleWorktree>> {
        let mut stale = Vec::new();
        let primary = self.repo.canonicalize().unwrap_or_else(|_| self.repo.clone());
        for info in self.list()? {
            if info.path == primary {
                continue;
            }
            let mut reasons = Vec::new();
            if !info.path.exists() {
                reasons.push("path missing".into());
            } else if let Ok(mtime) = dir_mtime(&info.path) {
                if mtime.elapsed().unwrap_or_default() >= criteria.idle_for {
                    reasons.push(format!("idle for {:?}", criteria.idle_for));
                }
            }
            if criteria.no_new_commits {
                if let (Some(base), Some(current)) = (&info.base_commit, &info.current_commit) {
                    if base == current {
                        reasons.push("no commits since creation".into());
                    }
                }
            }
            if !reasons.is_empty() {
                stale.push(StaleWorktree { info, reasons });
            }
        }
        Ok(stale)
    }

    fn working_state(&self, path: &Path) -> WorkingState {
        if !path.exists() {
            return WorkingState::Missing;
        }
        match git_output(path, &["status", "--porcelain=v2"]) {
            Ok(out) if out.lines().any(|l| !l.starts_with('#') && !l.is_empty()) => WorkingState::Dirty,
            Ok(_) => WorkingState::Clean,
            Err(_) => WorkingState::Missing,
        }
    }

    fn persist(&self, info: &WorktreeInfo) -> Result<WorktreeInfo> {
        let payload = serde_json::json!({
            "path": info.path,
            "branch": info.branch,
            "head": info.head,
            "task": info.task,
            "agent": info.agent,
            "base_commit": info.base_commit,
            "current_commit": info.current_commit,
            "working_state": info.working_state,
        });
        let name = info.path.to_string_lossy().into_owned();
        let node = if let Some(mut existing) = self.store.find_node_by_name(NodeKind::Worktree, &name)? {
            existing.payload = payload;
            existing.touch();
            self.store.upsert_node(&existing)?;
            existing
        } else {
            let node = Node::new(NodeKind::Worktree, Some(name), payload);
            self.store.upsert_node(&node)?;
            node
        };
        if let Some(repo) = self.store.nodes_of_kind(NodeKind::Repository)?.into_iter().next() {
            if self.store.find_edge(repo.id, node.id, EdgeKind::Contains)?.is_none() {
                self.store.upsert_edge(&Edge::new(repo.id, node.id, EdgeKind::Contains))?;
            }
        }
        self.store.settings().set(
            SCOPE,
            &meta_key(&info.path),
            &serde_json::json!({
                "node_id": node.id.to_string(),
                "task": info.task,
                "agent": info.agent,
                "base_commit": info.base_commit,
            }),
        )?;
        if let Ok(canon) = info.path.canonicalize() {
            if canon != info.path {
                self.store.settings().set(
                    SCOPE,
                    &meta_key(&canon),
                    &serde_json::json!({
                        "node_id": node.id.to_string(),
                        "task": info.task,
                        "agent": info.agent,
                        "base_commit": info.base_commit,
                    }),
                )?;
            }
        }
        let mut stored = info.clone();
        stored.node_id = Some(node.id);
        Ok(stored)
    }

    fn load_meta(&self, path: &Path) -> Result<Option<StoredMeta>> {
        let mut candidates = vec![meta_key(path)];
        if let Ok(canon) = path.canonicalize() {
            candidates.push(meta_key(&canon));
        }
        for key in candidates {
            if let Some(value) = self.store.settings().get(SCOPE, &key)? {
                if value.is_null() {
                    continue;
                }
                if let Ok(meta) = serde_json::from_value(value) {
                    return Ok(Some(meta));
                }
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Debug, Deserialize)]
struct StoredMeta {
    node_id: String,
    task: Option<String>,
    agent: Option<String>,
    base_commit: Option<String>,
}

struct Listed {
    path: PathBuf,
    head: Option<String>,
    branch: Option<String>,
    bare: bool,
    detached: bool,
}

fn parse_porcelain(output: &str) -> Vec<Listed> {
    let mut out = Vec::new();
    let mut current: Option<Listed> = None;
    for line in output.lines() {
        if line.is_empty() {
            if let Some(item) = current.take() {
                out.push(item);
            }
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(item) = current.take() {
                out.push(item);
            }
            current = Some(Listed {
                path: PathBuf::from(path),
                head: None,
                branch: None,
                bare: false,
                detached: false,
            });
        } else if let Some(item) = current.as_mut() {
            if let Some(head) = line.strip_prefix("HEAD ") {
                item.head = Some(head.to_string());
            } else if let Some(branch) = line.strip_prefix("branch ") {
                item.branch = Some(branch.trim_start_matches("refs/heads/").to_string());
            } else if line == "bare" {
                item.bare = true;
            } else if line == "detached" {
                item.detached = true;
            }
        }
    }
    if let Some(item) = current {
        out.push(item);
    }
    out
}

fn meta_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn dir_mtime(path: &Path) -> std::io::Result<SystemTime> {
    std::fs::metadata(path)?.modified()
}
