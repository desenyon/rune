use crate::error::{GitIntelError, Result};
use crate::git::{git_output, is_git_repo};
use crate::parse::{
    parse_log, parse_refs, parse_status_v2, parse_worktrees, CommitRecord, RefKind, WorkingTreeStatus,
};
use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind, Provenance, ProvenanceSource, ProvenanceSubject};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCOPE: &str = "git_intelligence";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitIndexReport {
    pub repository_id: NodeId,
    pub commits_indexed: usize,
    pub commits_skipped: usize,
    pub branches: usize,
    pub tags: usize,
    pub worktrees: usize,
    pub authors: usize,
}

pub struct GitIndexer {
    store: Store,
    root: PathBuf,
}

impl GitIndexer {
    pub fn new(store: Store, root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !is_git_repo(&root) {
            return Err(GitIntelError::NotARepository(root));
        }
        Ok(Self {
            store,
            root: root.canonicalize().unwrap_or(root),
        })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn index(&self) -> Result<GitIndexReport> {
        let repo = self.ensure_repository()?;
        let mut commits_indexed = 0;
        let mut commits_skipped = 0;
        let mut authors = 0;
        let log = git_output(
            &self.root,
            &[
                "log",
                "--all",
                "--pretty=format:COMMIT%x1f%H%x1f%an%x1f%ae%x1f%aI%x1f%P%x1f%s",
                "--name-status",
            ],
        )?;
        for commit in parse_log(&log) {
            if self.store.find_node_by_name(NodeKind::Commit, &commit.sha)?.is_some() {
                commits_skipped += 1;
                continue;
            }
            self.persist_commit(repo.id, &commit)?;
            authors += 1;
            commits_indexed += 1;
        }
        let refs = git_output(
            &self.root,
            &[
                "for-each-ref",
                "--format=%(objectname)%01%(objecttype)%01%(refname)%01%(refname:short)",
                "refs/heads",
                "refs/tags",
            ],
        )?;
        let mut branches = 0;
        let mut tags = 0;
        for reference in parse_refs(&refs) {
            let kind = match reference.kind {
                RefKind::Branch => {
                    branches += 1;
                    NodeKind::Branch
                }
                RefKind::Tag => {
                    tags += 1;
                    NodeKind::Tag
                }
            };
            let node = upsert(
                &self.store,
                kind,
                &reference.short_name,
                serde_json::json!({
                    "sha": reference.sha,
                    "full_name": reference.full_name,
                    "short_name": reference.short_name,
                }),
            )?;
            ensure_edge(&self.store, repo.id, node.id, EdgeKind::Contains)?;
            if let Some(commit) = self.store.find_node_by_name(NodeKind::Commit, &reference.sha)? {
                ensure_edge(&self.store, node.id, commit.id, EdgeKind::CreatedBy)?;
            }
        }
        let worktrees_raw = git_output(&self.root, &["worktree", "list", "--porcelain"])?;
        let worktrees = parse_worktrees(&worktrees_raw);
        for wt in &worktrees {
            let name = wt.path.to_string_lossy().into_owned();
            let node = upsert(
                &self.store,
                NodeKind::Worktree,
                &name,
                serde_json::json!({
                    "path": wt.path,
                    "head": wt.head,
                    "branch": wt.branch,
                    "bare": wt.bare,
                    "detached": wt.detached,
                }),
            )?;
            ensure_edge(&self.store, repo.id, node.id, EdgeKind::Contains)?;
            if let Some(head) = &wt.head {
                if let Some(commit) = self.store.find_node_by_name(NodeKind::Commit, head)? {
                    ensure_edge(&self.store, node.id, commit.id, EdgeKind::CreatedBy)?;
                }
            }
        }
        let status = self.working_tree_status()?;
        let mut repo_node = self.store.get_node(repo.id)?;
        repo_node.payload["working_tree"] = serde_json::to_value(&status)?;
        repo_node.touch();
        self.store.upsert_node(&repo_node)?;
        Ok(GitIndexReport {
            repository_id: repo.id,
            commits_indexed,
            commits_skipped,
            branches,
            tags,
            worktrees: worktrees.len(),
            authors,
        })
    }

    pub fn working_tree_status(&self) -> Result<WorkingTreeStatus> {
        let raw = git_output(
            &self.root,
            &["status", "--porcelain=v2", "--branch", "--untracked-files=all"],
        )?;
        Ok(parse_status_v2(&raw))
    }

    pub fn commits_touching_path(&self, path: &str) -> Result<Vec<Node>> {
        let raw = git_output(
            &self.root,
            &["log", "--pretty=format:%H", "--", path],
        )?;
        let mut nodes = Vec::new();
        for sha in raw.lines().filter(|l| !l.is_empty()) {
            if let Some(node) = self.store.find_node_by_name(NodeKind::Commit, sha)? {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    /// Blame-lite: history of a path, optionally filtered by a pickaxe string (`git log -S`).
    pub fn blame_lite(&self, path: &str, needle: Option<&str>) -> Result<Vec<CommitRecord>> {
        let mut args = vec!["log", "--pretty=format:COMMIT%x1f%H%x1f%an%x1f%ae%x1f%aI%x1f%P%x1f%s", "--name-status"];
        let needle_owned;
        if let Some(value) = needle {
            args.push("-S");
            needle_owned = value.to_string();
            args.push(&needle_owned);
        }
        args.push("--");
        args.push(path);
        let raw = git_output(&self.root, &args)?;
        Ok(parse_log(&raw))
    }

    fn ensure_repository(&self) -> Result<Node> {
        let name = self
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repository")
            .to_string();
        if let Some(id_val) = self.store.settings().get(SCOPE, "repository")? {
            if let Some(id) = id_val.as_str() {
                if let Ok(parsed) = id.parse::<NodeId>() {
                    if let Ok(node) = self.store.get_node(parsed) {
                        return Ok(node);
                    }
                }
            }
        }
        let node = Node::new(
            NodeKind::Repository,
            Some(name),
            serde_json::json!({ "root": self.root }),
        );
        self.store.upsert_node(&node)?;
        self.store.settings().set(
            SCOPE,
            "repository",
            &serde_json::Value::String(node.id.to_string()),
        )?;
        Ok(node)
    }

    fn persist_commit(&self, repo_id: NodeId, commit: &CommitRecord) -> Result<()> {
        let node = Node::new(
            NodeKind::Commit,
            Some(commit.sha.clone()),
            serde_json::json!({
                "sha": commit.sha,
                "author_name": commit.author_name,
                "author_email": commit.author_email,
                "authored_at": commit.authored_at,
                "parents": commit.parents,
                "subject": commit.subject,
                "files": commit.files,
            }),
        );
        self.store.upsert_node(&node)?;
        ensure_edge(&self.store, repo_id, node.id, EdgeKind::Contains)?;
        self.store.insert_provenance(&Provenance::observed(
            ProvenanceSubject::Node(node.id),
            ProvenanceSource::GitCommit {
                sha: commit.sha.clone(),
            },
        ))?;
        let author = upsert(
            &self.store,
            NodeKind::Author,
            &commit.author_email,
            serde_json::json!({
                "name": commit.author_name,
                "email": commit.author_email,
            }),
        )?;
        ensure_edge(&self.store, node.id, author.id, EdgeKind::CreatedBy)?;
        for change in &commit.files {
            let file = upsert(
                &self.store,
                NodeKind::File,
                &change.path,
                serde_json::json!({ "path": change.path }),
            )?;
            ensure_edge(&self.store, file.id, node.id, EdgeKind::ChangedBy)?;
        }
        Ok(())
    }
}

fn upsert(store: &Store, kind: NodeKind, name: &str, payload: serde_json::Value) -> Result<Node> {
    if let Some(mut existing) = store.find_node_by_name(kind.clone(), name)? {
        existing.payload = payload;
        existing.touch();
        store.upsert_node(&existing)?;
        return Ok(existing);
    }
    let node = Node::new(kind, Some(name.to_string()), payload);
    store.upsert_node(&node)?;
    Ok(node)
}

fn ensure_edge(store: &Store, from: NodeId, to: NodeId, kind: EdgeKind) -> Result<()> {
    if store.find_edge(from, to, kind.clone())?.is_some() {
        return Ok(());
    }
    store.upsert_edge(&Edge::new(from, to, kind))?;
    Ok(())
}
