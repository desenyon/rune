//! CLI surface for Rune (S077, S078, S074). Commands perform real work.

mod doctor;
mod onboard;
mod retrievers;
mod tui;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::{generate, shells::Shell};
use rune_app::{App, ExportFormat};
use rune_context_compiler::{
    compare_capsules, explain_why, CompileRequest, ContextCompiler, EmptyRetriever, Retrievers,
};
use rune_core::{Node, NodeId, NodeKind};
use rune_docs_context::Context7Provider;
use rune_git_intelligence::{GitIndexReport, GitIndexer, GitIntelError};
use rune_graph::ExpandFilter;
use rune_handoff::{HandoffCompiler, HandoffMode};
use rune_index::{impact_for_files, Indexer, WorkspaceScanReport};
use rune_memory::{CodeChange, FreshnessEngine, FreshnessJudgment, MemoryStore};
use rune_providers::ProviderRegistry;
use rune_search::{SearchEngine, SearchMode, SearchRequest};
use rune_sessions::{import_discovered, DiscoveryContext};
use rune_specs::SpecStore;
use rune_storage::Store;
use rune_tasks::TaskStore;
use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;

use crate::retrievers::{GuidanceMemoryRetriever, StoreSpecRetriever, StoreTaskRetriever};

pub use doctor::{doctor_report, DoctorCheck, DoctorReport};
pub use onboard::{inspect_environment, OnboardingReport};
pub use tui::run_tui;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    App(#[from] rune_app::AppError),
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error(transparent)]
    Compiler(#[from] rune_context_compiler::CompilerError),
    #[error(transparent)]
    Handoff(#[from] rune_handoff::HandoffError),
    #[error(transparent)]
    Index(#[from] rune_index::IndexError),
    #[error(transparent)]
    Search(#[from] rune_search::SearchError),
    #[error(transparent)]
    Memory(#[from] rune_memory::MemoryError),
    #[error(transparent)]
    Specs(#[from] rune_specs::SpecError),
    #[error(transparent)]
    Tasks(#[from] rune_tasks::TaskError),
    #[error(transparent)]
    Sessions(#[from] rune_sessions::SessionError),
    #[error(transparent)]
    Git(#[from] GitIntelError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "rune", version, about = "Rune local-first Context OS")]
pub struct Cli {
    #[arg(long, value_enum, default_value_t = OutputFormat::Text, global = true)]
    pub format: OutputFormat,
    #[arg(long, global = true)]
    pub path: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Index the workspace into the local graph store.
    Index,
    /// Search indexed objects. Mode is inferred unless `--mode` is set.
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Force a retrieval mode: exact, fuzzy, full_text, structural, semantic, graph, temporal, hybrid.
        #[arg(long)]
        mode: Option<String>,
    },
    /// Show graph neighbors or repository graph stats.
    Graph {
        #[arg(long)]
        node: Option<String>,
        #[arg(long, default_value_t = 2)]
        depth: usize,
    },
    /// List persistent memories.
    Memory,
    /// List ingested sessions, or import local agent histories.
    Sessions {
        #[command(subcommand)]
        cmd: Option<SessionsCmd>,
    },
    /// List tasks.
    Tasks,
    /// List specifications and their requirements.
    Specs,
    /// Compile or list handoffs.
    Handoff {
        #[command(subcommand)]
        cmd: HandoffCmd,
    },
    /// Compile or inspect context capsules.
    Context {
        #[command(subcommand)]
        cmd: ContextCmd,
    },
    /// List agent nodes and detected local agent directories.
    Agents,
    /// Diagnose the local environment.
    Doctor,
    /// First-launch environment inspection. No account required.
    Onboard,
    /// Terminal UI (works at basic renderer level without graphics).
    Tui,
    /// Export graph objects without secrets.
    Export {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        kind: Option<String>,
    },
    /// Generate shell completions.
    Completions { shell: Shell },
    /// Run evaluation suites (S062–S065).
    Eval {
        #[arg(long)]
        name: Option<String>,
    },
    /// Show graph impact of changed files (S050).
    Impact {
        #[arg(long)]
        file: Option<String>,
    },
    /// Package a distributable binary for the host target (S085).
    Package {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Write a local sanitized crash bundle (S087).
    Crash {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Check for updates without replacing the running binary (S086).
    Update,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SessionsCmd {
    /// Discover and import local coding-agent session files.
    Import,
}

#[derive(Subcommand, Debug, Clone)]
pub enum HandoffCmd {
    Compile {
        goal: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long, default_value = "balanced")]
        mode: String,
    },
    List,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ContextCmd {
    Compile {
        goal: String,
        #[arg(long, default_value_t = 4000)]
        budget: usize,
    },
    Diff {
        left: String,
        right: String,
    },
    Explain {
        capsule: String,
        object: String,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    run_cli(cli)
}

pub fn run_with_args<I, T>(iter: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(iter);
    run_cli(cli)
}

fn run_cli(cli: Cli) -> Result<()> {
    let command = cli.command.clone().unwrap_or(Commands::Tui);
    match &command {
        Commands::Completions { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            generate(*shell, &mut cmd, "rune", &mut io::stdout());
            return Ok(());
        }
        Commands::Doctor => {
            let workspace = workspace(&cli.path)?;
            let report = match App::open_or_create(&workspace) {
                Ok(app) => doctor_report(Some(&app.store), &workspace, &app.providers),
                Err(_) => doctor_report(None, &workspace, &ProviderRegistry::new()),
            };
            emit(cli.format, &report)?;
            if !report.ok {
                return Err(CliError::Message("doctor found errors".into()));
            }
            return Ok(());
        }
        Commands::Onboard => {
            let workspace = workspace(&cli.path)?;
            let report = inspect_environment(&workspace);
            emit(cli.format, &report)?;
            return Ok(());
        }
        Commands::Tui => {
            let workspace = workspace(&cli.path)?;
            return run_tui(&workspace).map_err(|err| CliError::Message(err));
        }
        _ => {}
    }

    let workspace = workspace(&cli.path)?;
    let mut app = App::open_or_create(&workspace)?;
    app.providers
        .register(Box::new(Context7Provider::default()));

    match command {
        Commands::Index => {
            let stats = index_workspace(&app.store, &workspace)?;
            emit(cli.format, &stats)?;
        }
        Commands::Search {
            query,
            limit,
            mode,
        } => {
            let engine = SearchEngine::new(&app.store);
            let mut request = SearchRequest::new(query);
            request.limit = limit;
            if let Some(mode) = mode {
                request.mode = Some(
                    SearchMode::from_str(&mode).map_err(CliError::Message)?,
                );
            }
            let response = engine.search(request)?;
            emit(cli.format, &response)?;
        }
        Commands::Graph { node, depth } => {
            if let Some(id) = node {
                let nid =
                    NodeId::from_str(&id).map_err(|err| CliError::Message(err.to_string()))?;
                let nodes = app.graph().expand(nid, ExpandFilter::depth(depth))?;
                emit(cli.format, &nodes)?;
            } else {
                let payload = serde_json::json!({
                    "nodes": app.store.node_count()?,
                    "edges": app.store.edge_count()?,
                });
                emit(cli.format, &payload)?;
            }
        }
        Commands::Memory => {
            emit(cli.format, &MemoryStore::new(&app.store).list()?)?;
        }
        Commands::Sessions { cmd } => match cmd {
            None => {
                emit(cli.format, &app.store.nodes_of_kind(NodeKind::Session)?)?;
            }
            Some(SessionsCmd::Import) => {
                let mut ctx = DiscoveryContext::from_env();
                ctx.workspace = Some(workspace.clone());
                let imported = import_discovered(&app.store, &ctx)?;
                emit(
                    cli.format,
                    &imported
                        .iter()
                        .map(|session| {
                            serde_json::json!({
                                "session_id": session.session_id,
                                "turns": session.turn_ids.len(),
                                "extractions": session.extraction_ids.len(),
                                "memories": session.memory_ids.len(),
                            })
                        })
                        .collect::<Vec<_>>(),
                )?;
            }
        }
        Commands::Tasks => {
            emit(cli.format, &TaskStore::new(&app.store).list()?)?;
        }
        Commands::Specs => {
            emit(cli.format, &SpecStore::new(&app.store).list()?)?;
        }
        Commands::Handoff { cmd } => match cmd {
            HandoffCmd::Compile {
                goal,
                from,
                to,
                mode,
            } => {
                let source = require_named_session(&app.store, &from)?;
                let empty = EmptyRetriever;
                let tasks = StoreTaskRetriever;
                let specs = StoreSpecRetriever;
                let memory = GuidanceMemoryRetriever;
                let retrievers = Retrievers {
                    tasks: &tasks,
                    specs: &specs,
                    memory: &memory,
                    history: &empty,
                    git: &empty,
                    docs: &empty,
                };
                let mode = parse_mode(&mode)?;
                let compiler = HandoffCompiler::new(&app.store);
                let package = compiler.compile(source, from, to, goal, mode, None, &retrievers)?;
                app.store.upsert_node(&package.handoff.into_node())?;
                emit(cli.format, &package.handoff)?;
            }
            HandoffCmd::List => {
                emit(cli.format, &app.store.nodes_of_kind(NodeKind::Handoff)?)?;
            }
        },
        Commands::Context { cmd } => match cmd {
            ContextCmd::Compile { goal, budget } => {
                let compiler = ContextCompiler::new(&app.store);
                let empty = EmptyRetriever;
                let tasks = StoreTaskRetriever;
                let specs = StoreSpecRetriever;
                let memory = GuidanceMemoryRetriever;
                let retrievers = Retrievers {
                    tasks: &tasks,
                    specs: &specs,
                    memory: &memory,
                    history: &empty,
                    git: &empty,
                    docs: &empty,
                };
                let mut req = CompileRequest::new(goal, budget);
                req.persist = true;
                let compiled = compiler.compile(req, &retrievers)?;
                emit(cli.format, &compiled.capsule)?;
            }
            ContextCmd::Diff { left, right } => {
                let l = load_capsule(&app.store, &left)?;
                let r = load_capsule(&app.store, &right)?;
                emit(cli.format, &compare_capsules(&l, &r))?;
            }
            ContextCmd::Explain { capsule, object } => {
                let cap = load_capsule(&app.store, &capsule)?;
                let oid =
                    NodeId::from_str(&object).map_err(|e| CliError::Message(e.to_string()))?;
                emit(cli.format, &explain_why(&cap, oid))?;
            }
        },
        Commands::Agents => {
            let nodes = app.store.nodes_of_kind(NodeKind::Agent)?;
            let detected = inspect_environment(&workspace).coding_agents;
            emit(
                cli.format,
                &serde_json::json!({"nodes": nodes, "detected": detected}),
            )?;
        }
        Commands::Export { format, kind } => {
            let fmt = match format.as_str() {
                "json" => ExportFormat::Json,
                "jsonl" => ExportFormat::Jsonl,
                "markdown" | "md" => ExportFormat::Markdown,
                "graph" => ExportFormat::Graph,
                other => return Err(CliError::Message(format!("unknown export format {other}"))),
            };
            let text = if let Some(kind) = kind {
                app.export_kind(fmt, NodeKind::parse(&kind))?
            } else {
                let mut nodes = Vec::new();
                for kind in [
                    NodeKind::File,
                    NodeKind::Function,
                    NodeKind::Memory,
                    NodeKind::Task,
                    NodeKind::Session,
                    NodeKind::ContextCapsule,
                ] {
                    nodes.extend(app.store.nodes_of_kind(kind)?);
                }
                app.export(fmt, &nodes)?
            };
            println!("{text}");
        }
        Commands::Eval { name } => {
            let results = if let Some(name) = name {
                vec![rune_evals::run_named(&name).map_err(CliError::Message)?]
            } else {
                rune_evals::all_evals()
            };
            let _ = rune_evals::maybe_write_benchmarks(&results);
            emit(cli.format, &results)?;
            if results.iter().any(|result| !result.passed) {
                return Err(CliError::Message("evaluation suite failed".into()));
            }
        }
        Commands::Impact { file } => {
            let ids = if let Some(name) = file {
                let node = app
                    .store
                    .find_node_by_name(NodeKind::File, &name)?
                    .ok_or_else(|| CliError::Message(format!("file `{name}` not indexed")))?;
                vec![node.id]
            } else {
                app.store
                    .nodes_of_kind(NodeKind::File)?
                    .into_iter()
                    .take(12)
                    .map(|node| node.id)
                    .collect()
            };
            emit(cli.format, &impact_for_files(&app.store, &ids)?)?;
        }
        Commands::Package { out } => {
            let dest_dir = out.unwrap_or_else(|| workspace.join("dist"));
            std::fs::create_dir_all(&dest_dir)?;
            let dest = dest_dir.join(format!(
                "rune-{}-{}",
                std::env::consts::ARCH,
                std::env::consts::OS
            ));
            app.refuse_replace_running_binary(&dest)?;
            let exe = std::env::current_exe()?;
            std::fs::copy(&exe, &dest)?;
            emit(
                cli.format,
                &serde_json::json!({"binary": dest, "source": exe}),
            )?;
        }
        Commands::Crash { out } => {
            let bundle = app.crash_bundle(None)?;
            if let Some(path) = out {
                std::fs::write(&path, serde_json::to_string_pretty(&bundle).unwrap())?;
            }
            emit(cli.format, &bundle)?;
        }
        Commands::Update => {
            emit(cli.format, &app.check_update(None)?)?;
        }
        Commands::Completions { .. } | Commands::Doctor | Commands::Onboard | Commands::Tui => {
            unreachable!()
        }
    }
    Ok(())
}

fn workspace(path: &Option<PathBuf>) -> Result<PathBuf> {
    Ok(path.clone().unwrap_or(std::env::current_dir()?))
}

fn emit<T: Serialize>(format: OutputFormat, value: &T) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value)
                    .map_err(|e| CliError::Message(e.to_string()))?
            );
        }
        OutputFormat::Text => {
            println!(
                "{}",
                serde_json::to_string_pretty(value)
                    .map_err(|e| CliError::Message(e.to_string()))?
            );
        }
    }
    Ok(())
}

fn parse_mode(mode: &str) -> Result<HandoffMode> {
    match mode {
        "full" => Ok(HandoffMode::Full),
        "balanced" => Ok(HandoffMode::Balanced),
        "compact" => Ok(HandoffMode::Compact),
        "custom" => Ok(HandoffMode::Custom),
        other => Err(CliError::Message(format!("unknown handoff mode {other}"))),
    }
}

fn require_named_session(store: &Store, name: &str) -> Result<Node> {
    store
        .find_node_by_name(NodeKind::Session, name)?
        .ok_or_else(|| {
            CliError::Message(format!(
                "session `{name}` not found; ingest or create it first"
            ))
        })
}

fn load_capsule(store: &Store, id: &str) -> Result<rune_context_compiler::ContextCapsule> {
    let nid = NodeId::from_str(id).map_err(|e| CliError::Message(e.to_string()))?;
    let node = store.get_node(nid)?;
    serde_json::from_value(node.payload).map_err(|e| CliError::Message(e.to_string()))
}

#[derive(Clone, Debug, Serialize)]
pub struct IndexStats {
    pub files_seen: usize,
    pub files_indexed: usize,
    pub files_skipped_unchanged: usize,
    pub languages: Vec<String>,
    pub processes: usize,
    pub git: Option<GitIndexReport>,
    pub stale_memories: usize,
}

pub fn index_workspace(store: &Store, root: &Path) -> Result<IndexStats> {
    let indexer = Indexer::new(store.clone(), root)?;
    let scan: WorkspaceScanReport = indexer.scan_workspace()?;
    let git = match GitIndexer::new(store.clone(), root) {
        Ok(git) => Some(git.index()?),
        Err(GitIntelError::NotARepository(_)) => None,
        Err(err) => return Err(err.into()),
    };
    let stale_memories = apply_freshness(store, &scan)?;
    Ok(IndexStats {
        files_seen: scan.files_seen,
        files_indexed: scan.files_indexed,
        files_skipped_unchanged: scan.files_skipped_unchanged,
        languages: scan.languages,
        processes: scan.processes,
        git,
        stale_memories,
    })
}

fn apply_freshness(store: &Store, scan: &WorkspaceScanReport) -> Result<usize> {
    if scan.changed_files.is_empty() {
        return Ok(0);
    }
    let mut change = CodeChange::default();
    for file in &scan.changed_files {
        change.file_ids.push(file.file_id);
        change.symbol_ids.extend(file.symbol_ids.iter().copied());
        change
            .new_file_hashes
            .insert(file.file_id.to_string(), file.content_hash.clone());
    }
    let reasons = FreshnessEngine::new(store)
        .apply(&change)
        .map_err(|err| CliError::Message(err.to_string()))?;
    Ok(reasons
        .iter()
        .filter(|reason| {
            matches!(
                reason.judgment,
                FreshnessJudgment::PossiblyStale | FreshnessJudgment::LikelyContradicted
            )
        })
        .count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_providers::ProviderRegistry;

    #[test]
    fn doctor_returns_ok_on_memory_db() {
        let store = Store::open_in_memory().unwrap();
        let report = doctor_report(Some(&store), Path::new("/tmp"), &ProviderRegistry::new());
        assert!(report.ok, "{:?}", report.checks);
        assert!(report.checks.iter().any(|c| c.name == "database" && c.ok));
    }

    #[test]
    fn index_workspace_creates_function_nodes() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("lib.rs"), "pub fn authenticate() {}\n").unwrap();
        let store = Store::open_in_memory().unwrap();
        let stats = index_workspace(&store, tmp.path()).unwrap();
        assert!(stats.files_indexed >= 1);
        assert!(stats.git.is_none());
        let functions = store.nodes_of_kind(NodeKind::Function).unwrap();
        assert!(
            functions
                .iter()
                .any(|node| node.name.as_deref() == Some("authenticate")),
            "expected authenticate function, got {functions:?}"
        );
    }

    #[test]
    fn search_finds_indexed_function() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("lib.rs"), "pub fn authenticate() {}\n").unwrap();
        let store = Store::open_in_memory().unwrap();
        index_workspace(&store, tmp.path()).unwrap();
        let response = SearchEngine::new(&store)
            .search(SearchRequest::new("authenticate").with_mode(SearchMode::Exact))
            .unwrap();
        assert!(response
            .hits
            .iter()
            .any(|hit| hit.node.name.as_deref() == Some("authenticate")));
    }

    #[test]
    fn guidance_retriever_omits_stale_memory() {
        use rune_context_compiler::{analyze_intent, MemoryRetriever};
        use rune_core::Validity;
        use rune_memory::{ClaimKind, Extractor};

        let store = Store::open_in_memory().unwrap();
        let memories = MemoryStore::new(&store);
        let verified = memories
            .ingest(
                Extractor::from_human_statement(
                    "dev",
                    "Authentication uses PostgreSQL sessions",
                    Some(ClaimKind::ObservedFact),
                )
                .unwrap(),
            )
            .unwrap();
        let mut stale = memories
            .ingest(
                Extractor::from_human_statement(
                    "dev",
                    "Authentication uses Redis sessions",
                    Some(ClaimKind::ObservedFact),
                )
                .unwrap(),
            )
            .unwrap();
        stale.validity = Validity::Stale;
        memories.persist(stale.clone()).unwrap();

        let intent = analyze_intent("authentication sessions", None);
        let hits = GuidanceMemoryRetriever
            .retrieve(&intent, &store)
            .unwrap();
        assert!(hits.iter().any(|hit| hit.node.id == verified.id));
        assert!(hits.iter().all(|hit| hit.node.id != stale.id));
    }

    #[test]
    fn reindex_marks_related_memory_stale() {
        use rune_core::Validity;
        use rune_memory::{ClaimKind, Extractor, RetrievalMode};

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("auth.rs");
        std::fs::write(&path, "session = redis\n").unwrap();
        let store = Store::open_in_memory().unwrap();
        index_workspace(&store, tmp.path()).unwrap();
        let file = store
            .find_node_by_name(NodeKind::File, "auth.rs")
            .unwrap()
            .expect("indexed auth.rs");
        let memories = MemoryStore::new(&store);
        let mut claim = Extractor::from_human_statement(
            "dev",
            "Authentication uses Redis sessions",
            Some(ClaimKind::ObservedFact),
        )
        .unwrap();
        claim.related_nodes.push(file.id);
        let record = memories.ingest(claim).unwrap();
        assert_eq!(record.validity, Validity::Verified);

        std::fs::write(&path, "session = postgres\n").unwrap();
        let stats = index_workspace(&store, tmp.path()).unwrap();
        assert!(
            stats.stale_memories >= 1,
            "expected stale memories after reindex, stats={stats:?}"
        );
        let updated = memories.get(record.id).unwrap();
        assert_eq!(updated.validity, Validity::Stale);
        assert!(memories
            .retrieve(RetrievalMode::AgentGuidance)
            .unwrap()
            .iter()
            .all(|item| item.id != record.id));
    }
}
