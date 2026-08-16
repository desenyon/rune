use crate::discovery::{self, WorkspaceDiscovery};
use crate::documents;
use crate::error::{IndexError, Result};
use crate::languages::{file_key, language_from_path, relative_posix, SourceLanguage};
use crate::persist::{
    delete_file_record, ensure_edge, load_file_record, load_repo_id, parse_node_id, save_file_record,
    save_repo_id, upsert_named, FileRecord, PendingCall,
};
use crate::process::{self, ProcessInfo};
use crate::structural::{self, ParsedFile};
use rune_core::{
    EdgeKind, Node, NodeId, NodeKind, Provenance, ProvenanceSource, ProvenanceSubject, Timestamp,
};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceScanReport {
    pub repository_id: NodeId,
    pub files_seen: usize,
    pub files_skipped_unchanged: usize,
    pub files_indexed: usize,
    pub nested_repos: Vec<PathBuf>,
    pub worktrees: Vec<crate::languages::WorktreeListing>,
    pub languages: Vec<String>,
    pub package_managers: Vec<String>,
    pub build_systems: Vec<String>,
    pub test_frameworks: Vec<String>,
    pub agent_configs: Vec<String>,
    pub docs_dirs: Vec<PathBuf>,
    pub spec_dirs: Vec<PathBuf>,
    pub rune_state: Option<PathBuf>,
    pub monorepo: Option<crate::languages::MonorepoKind>,
    pub processes: usize,
    pub changed_files: Vec<IndexedChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexedChange {
    pub file_id: NodeId,
    pub content_hash: String,
    pub symbol_ids: Vec<NodeId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestRun {
    pub at: Timestamp,
    pub passed: bool,
    pub run_id: Option<String>,
}

pub struct Indexer {
    store: Store,
    root: PathBuf,
    pause: Arc<AtomicBool>,
}

impl Indexer {
    pub fn new(store: Store, root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.is_dir() {
            return Err(IndexError::NotADirectory(root));
        }
        Ok(Self {
            store,
            root: root.canonicalize().unwrap_or(root),
            pause: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn pause_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.pause)
    }

    pub fn set_paused(&self, paused: bool) {
        self.pause.store(paused, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.pause.load(Ordering::SeqCst)
    }

    pub fn scan_workspace(&self) -> Result<WorkspaceScanReport> {
        let discovered = discovery::discover(&self.root)?;
        let repository_id = self.persist_repository(&discovered)?;
        let mut skipped = 0;
        let mut indexed = 0;
        let mut changed_files = Vec::new();
        for file in &discovered.files {
            if self.is_paused() {
                tracing::info!("index pause flag set; yielding remaining files");
                break;
            }
            match self.index_discovered_file(repository_id, file)? {
                FileIndexOutcome::SkippedUnchanged => skipped += 1,
                FileIndexOutcome::Indexed(change) => {
                    indexed += 1;
                    changed_files.push(change);
                }
            }
        }
        self.resolve_cross_file_calls()?;
        let processes = self.refresh_processes(repository_id)?;
        Ok(WorkspaceScanReport {
            repository_id,
            files_seen: discovered.files.len(),
            files_skipped_unchanged: skipped,
            files_indexed: indexed,
            nested_repos: discovered.nested_repos,
            worktrees: discovered.worktrees,
            languages: discovered.languages,
            package_managers: discovered.package_managers,
            build_systems: discovered.build_systems,
            test_frameworks: discovered.test_frameworks,
            agent_configs: discovered.agent_configs,
            docs_dirs: discovered.docs_dirs,
            spec_dirs: discovered.spec_dirs,
            rune_state: discovered.rune_state,
            monorepo: discovered.monorepo,
            processes: processes.len(),
            changed_files,
        })
    }

    pub fn index_path(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            let key = file_key(&self.root, path);
            delete_file_record(&self.store, &key)?;
            return Ok(());
        }
        if !path.is_file() {
            return Ok(());
        }
        let repo_id = match load_repo_id(&self.store)? {
            Some(id) => id,
            None => {
                let discovered = discovery::discover(&self.root)?;
                self.persist_repository(&discovered)?
            }
        };
        let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let file = crate::discovery::DiscoveredFile {
            relative: relative_posix(&self.root, path),
            path: path.to_path_buf(),
            content_hash: crate::discovery::hash_file(path)?,
            bytes,
            language: language_from_path(path).map(|l| l.as_str().to_string()),
        };
        self.index_discovered_file(repo_id, &file)?;
        self.resolve_cross_file_calls()?;
        Ok(())
    }

    pub fn record_test_run(&self, file_key_value: &str, test_name: &str, run: TestRun) -> Result<Node> {
        let record = load_file_record(&self.store, file_key_value)?
            .ok_or_else(|| IndexError::msg(format!("no indexed file for key {file_key_value}")))?;
        for id in &record.symbol_ids {
            let node_id = parse_node_id(id)?;
            let mut node = self.store.get_node(node_id)?;
            if node.kind == NodeKind::Test && node.name.as_deref() == Some(test_name) {
                node.payload["last_run"] = serde_json::json!({
                    "at": run.at,
                    "passed": run.passed,
                    "run_id": run.run_id,
                });
                node.touch();
                self.store.upsert_node(&node)?;
                self.store.insert_provenance(&Provenance::observed(
                    ProvenanceSubject::Node(node.id),
                    ProvenanceSource::Test {
                        name: test_name.to_string(),
                        run_id: run.run_id,
                    },
                ))?;
                return Ok(node);
            }
        }
        Err(IndexError::msg(format!("test `{test_name}` not found for file")))
    }

    pub fn discover_processes(&self) -> Result<Vec<ProcessInfo>> {
        process::discover_processes(&self.root)
    }

    fn persist_repository(&self, discovered: &WorkspaceDiscovery) -> Result<NodeId> {
        let name = self
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
            .to_string();
        let payload = serde_json::json!({
            "root": self.root,
            "is_git": discovered.is_git,
            "monorepo": discovered.monorepo,
            "languages": discovered.languages,
            "package_managers": discovered.package_managers,
            "build_systems": discovered.build_systems,
            "test_frameworks": discovered.test_frameworks,
            "agent_configs": discovered.agent_configs,
            "docs_dirs": discovered.docs_dirs,
            "spec_dirs": discovered.spec_dirs,
            "rune_state": discovered.rune_state,
            "nested_repos": discovered.nested_repos,
            "worktrees": discovered.worktrees,
        });
        let node = if let Some(id) = load_repo_id(&self.store)? {
            let mut existing = self.store.get_node(id)?;
            existing.payload = payload;
            existing.name = Some(name);
            existing.touch();
            self.store.upsert_node(&existing)?;
            existing
        } else {
            let node = Node::new(NodeKind::Repository, Some(name), payload);
            self.store.upsert_node(&node)?;
            save_repo_id(&self.store, node.id)?;
            self.store.insert_provenance(&Provenance::observed(
                ProvenanceSubject::Node(node.id),
                ProvenanceSource::SourceCode {
                    path: self.root.to_string_lossy().into_owned(),
                    start_byte: None,
                    end_byte: None,
                    start_line: None,
                    end_line: None,
                },
            ))?;
            node
        };
        Ok(node.id)
    }

    fn index_discovered_file(
        &self,
        repository_id: NodeId,
        file: &crate::discovery::DiscoveredFile,
    ) -> Result<FileIndexOutcome> {
        let key = file_key(&self.root, &file.path);
        if let Some(existing) = load_file_record(&self.store, &key)? {
            if existing.content_hash == file.content_hash {
                return Ok(FileIndexOutcome::SkippedUnchanged);
            }
            for id in existing.symbol_ids.iter().chain(existing.import_ids.iter()) {
                if let Ok(node_id) = parse_node_id(id) {
                    let _ = self.store.delete_node(node_id);
                }
            }
        }
        let language = language_from_path(&file.path);
        let payload = serde_json::json!({
            "path": file.relative,
            "file_key": key,
            "language": language.map(|l| l.as_str()),
            "content_hash": file.content_hash,
            "bytes": file.bytes,
        });
        let mut file_node = upsert_named(&self.store, NodeKind::File, &file.relative, payload)?;
        if let Ok(hash) = rune_core::ContentHash::from_hex(&file.content_hash) {
            file_node.content_hash = Some(hash);
            self.store.upsert_node(&file_node)?;
        }
        ensure_edge(&self.store, repository_id, file_node.id, EdgeKind::Contains)?;
        self.store.insert_provenance(&Provenance::observed(
            ProvenanceSubject::Node(file_node.id),
            ProvenanceSource::SourceCode {
                path: file.relative.clone(),
                start_byte: None,
                end_byte: None,
                start_line: None,
                end_line: None,
            },
        ))?;

        let mut symbol_ids = Vec::new();
        let mut import_ids = Vec::new();
        let mut pending_calls = Vec::new();
        if let Some(lang) = language.filter(|l| l.is_indexable()) {
            match std::fs::read_to_string(&file.path) {
                Ok(source) => match structural::parse_source(lang, &file.path, &source) {
                    Ok(parsed) => {
                        let ids = self.persist_parsed(&file_node, &file.relative, &key, parsed)?;
                        symbol_ids = ids.0;
                        import_ids = ids.1;
                        pending_calls = ids.2;
                    }
                    Err(err) => tracing::warn!(error = %err, path = %file.relative, "structural parse failed"),
                },
                Err(err) => tracing::warn!(error = %err, path = %file.relative, "unable to read source"),
            }
        } else if let Some(lang) = language.filter(|l| {
            matches!(
                l,
                SourceLanguage::Markdown
                    | SourceLanguage::Json
                    | SourceLanguage::Toml
                    | SourceLanguage::Yaml
            )
        }) {
            match std::fs::read_to_string(&file.path) {
                Ok(source) => {
                    symbol_ids = self.persist_document(&file_node, &file.relative, &source, lang)?;
                }
                Err(err) => tracing::warn!(error = %err, path = %file.relative, "unable to read document"),
            }
        }
        save_file_record(
            &self.store,
            &key,
            &FileRecord {
                node_id: file_node.id.to_string(),
                content_hash: file.content_hash.clone(),
                path: file.relative.clone(),
                symbol_ids: symbol_ids.iter().map(|id| id.to_string()).collect(),
                import_ids: import_ids.iter().map(|id| id.to_string()).collect(),
                pending_calls,
            },
        )?;
        Ok(FileIndexOutcome::Indexed(IndexedChange {
            file_id: file_node.id,
            content_hash: file.content_hash.clone(),
            symbol_ids,
        }))
    }

    fn persist_parsed(
        &self,
        file_node: &Node,
        relative: &str,
        key: &str,
        parsed: ParsedFile,
    ) -> Result<(Vec<NodeId>, Vec<NodeId>, Vec<PendingCall>)> {
        let mut symbol_ids = Vec::new();
        let mut import_ids = Vec::new();
        let mut by_name: HashMap<String, NodeId> = HashMap::new();
        for symbol in parsed.symbols {
            let payload = serde_json::json!({
                "file_key": key,
                "path": relative,
                "name": symbol.name,
                "kind": symbol.kind.as_str(),
                "start_byte": symbol.start_byte,
                "end_byte": symbol.end_byte,
                "start_line": symbol.start_line,
                "end_line": symbol.end_line,
                "is_test": symbol.is_test,
                "test_framework": symbol.test_framework,
                "last_run": serde_json::Value::Null,
            });
            let node = Node::new(symbol.kind.clone(), Some(symbol.name.clone()), payload);
            self.store.upsert_node(&node)?;
            ensure_edge(&self.store, file_node.id, node.id, EdgeKind::Defines)?;
            if symbol.is_test {
                ensure_edge(&self.store, node.id, file_node.id, EdgeKind::Tests)?;
            }
            self.store.insert_provenance(&Provenance::observed(
                ProvenanceSubject::Node(node.id),
                ProvenanceSource::SourceCode {
                    path: relative.to_string(),
                    start_byte: Some(symbol.start_byte),
                    end_byte: Some(symbol.end_byte),
                    start_line: Some(symbol.start_line),
                    end_line: Some(symbol.end_line),
                },
            ))?;
            by_name.entry(symbol.name.clone()).or_insert(node.id);
            symbol_ids.push(node.id);
        }
        for import in parsed.imports {
            let module = upsert_named(
                &self.store,
                NodeKind::Module,
                &import.source,
                serde_json::json!({ "source": import.source, "imported_from": relative }),
            )?;
            ensure_edge(&self.store, file_node.id, module.id, EdgeKind::Imports)?;
            import_ids.push(module.id);
        }
        let mut pending_calls = Vec::new();
        for call in parsed.calls {
            let from = call
                .caller
                .as_ref()
                .and_then(|name| by_name.get(name).copied())
                .unwrap_or(file_node.id);
            if let Some(callee_id) = by_name.get(&call.callee).copied() {
                ensure_edge(&self.store, from, callee_id, EdgeKind::Calls)?;
            } else {
                pending_calls.push(PendingCall {
                    callee: call.callee,
                    caller_id: from.to_string(),
                });
            }
        }
        Ok((symbol_ids, import_ids, pending_calls))
    }

    fn persist_document(
        &self,
        file_node: &Node,
        relative: &str,
        source: &str,
        language: SourceLanguage,
    ) -> Result<Vec<NodeId>> {
        let parsed = documents::parse_document(relative, source, language.as_str());
        let doc = Node::new(
            NodeKind::Document,
            Some(parsed.title.clone()),
            serde_json::json!({
                "path": relative,
                "kind": parsed.kind,
                "section_count": parsed.sections.len(),
            }),
        );
        self.store.upsert_node(&doc)?;
        ensure_edge(&self.store, file_node.id, doc.id, EdgeKind::Documents)?;
        let mut ids = vec![doc.id];
        for section in parsed.sections {
            let node = Node::new(
                NodeKind::DocumentationSection,
                Some(section.heading.clone()),
                serde_json::json!({
                    "path": relative,
                    "heading": section.heading,
                    "content": section.content,
                    "start_line": section.start_line,
                }),
            );
            self.store.upsert_node(&node)?;
            ensure_edge(&self.store, doc.id, node.id, EdgeKind::Contains)?;
            ids.push(node.id);
        }
        Ok(ids)
    }

    fn resolve_cross_file_calls(&self) -> Result<()> {
        let mut by_name: HashMap<String, Vec<(NodeId, String)>> = HashMap::new();
        for kind in [
            NodeKind::Function,
            NodeKind::Method,
            NodeKind::Class,
            NodeKind::Trait,
            NodeKind::Test,
            NodeKind::Type,
        ] {
            for node in self.store.nodes_of_kind(kind)? {
                let Some(name) = node.name.clone() else {
                    continue;
                };
                let path = node
                    .payload
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                by_name.entry(name).or_default().push((node.id, path));
            }
        }
        let keys = self.store.settings().list_scope("index")?;
        for (key, _) in keys {
            let Some(file_key) = key.strip_prefix("file:") else {
                continue;
            };
            let Some(record) = load_file_record(&self.store, file_key)? else {
                continue;
            };
            if record.pending_calls.is_empty() {
                continue;
            }
            let import_names: Vec<String> = record
                .import_ids
                .iter()
                .filter_map(|id| parse_node_id(id).ok())
                .filter_map(|id| self.store.get_node(id).ok())
                .filter_map(|node| node.name)
                .collect();
            let mut remaining = Vec::new();
            let pending_calls = record.pending_calls.clone();
            for pending in pending_calls {
                let Some(candidates) = by_name.get(&pending.callee) else {
                    remaining.push(pending);
                    continue;
                };
                let caller_id = match parse_node_id(&pending.caller_id) {
                    Ok(id) => id,
                    Err(_) => {
                        remaining.push(pending);
                        continue;
                    }
                };
                let other: Vec<NodeId> = candidates
                    .iter()
                    .filter(|(id, _)| *id != caller_id)
                    .map(|(id, _)| *id)
                    .collect();
                let chosen = if other.len() == 1 {
                    other.first().copied()
                } else if other.len() > 1 {
                    let imported: Vec<NodeId> = candidates
                        .iter()
                        .filter(|(id, path)| {
                            *id != caller_id
                                && import_names.iter().any(|imp| {
                                    path.contains(imp)
                                        || imp.contains(
                                            path.rsplit('/')
                                                .next()
                                                .unwrap_or(path)
                                                .trim_end_matches(".rs")
                                                .trim_end_matches(".py")
                                                .trim_end_matches(".go"),
                                        )
                                })
                        })
                        .map(|(id, _)| *id)
                        .collect();
                    if imported.len() == 1 {
                        imported.first().copied()
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(callee_id) = chosen {
                    ensure_edge(&self.store, caller_id, callee_id, EdgeKind::Calls)?;
                    ensure_edge(&self.store, caller_id, callee_id, EdgeKind::References)?;
                } else {
                    remaining.push(pending);
                }
            }
            let mut updated = record;
            updated.pending_calls = remaining;
            save_file_record(&self.store, file_key, &updated)?;
        }
        Ok(())
    }

    fn refresh_processes(&self, repository_id: NodeId) -> Result<Vec<Node>> {
        for existing in self.store.nodes_of_kind(NodeKind::Process)? {
            let Some(cwd) = existing.payload.get("cwd").and_then(|v| v.as_str()) else {
                continue;
            };
            if Path::new(cwd).starts_with(&self.root) {
                let _ = self.store.delete_node(existing.id);
            }
        }
        let mut nodes = Vec::new();
        for proc in process::discover_processes(&self.root)? {
            let name = format!("pid:{}", proc.pid);
            let payload = serde_json::json!({
                "pid": proc.pid,
                "ppid": proc.ppid,
                "user": proc.user,
                "command": proc.command,
                "cwd": proc.cwd,
                "repository": self.root,
            });
            let node = upsert_named(&self.store, NodeKind::Process, &name, payload)?;
            ensure_edge(&self.store, repository_id, node.id, EdgeKind::Contains)?;
            nodes.push(node);
        }
        Ok(nodes)
    }
}

enum FileIndexOutcome {
    SkippedUnchanged,
    Indexed(IndexedChange),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::ContentHash;

    #[test]
    fn file_key_is_stable_across_content_hash_changes() {
        let root = Path::new("/repo");
        let a = file_key(root, Path::new("/repo/src/lib.rs"));
        let b = file_key(root, Path::new("/repo/src/lib.rs"));
        assert_eq!(a, b);
        assert_ne!(a, ContentHash::hash(b"fn x(){}").to_hex());
    }
}
