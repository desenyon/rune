//! S080: session → memory → symbol change → stale memory → spec/task →
//! compiled context → worktree edit → tests → commit → handoff → coverage.

use rune_context_compiler::{CompileRequest, ContextCompiler, EmptyRetriever, Retrievers};
use rune_core::{EdgeKind, Node, NodeKind, Timestamp};
use rune_evals::recall;
use rune_graph::Graph;
use rune_handoff::{HandoffCompiler, HandoffMode};
use rune_index::{file_key, impact_for_files, Indexer, TestRun};
use rune_memory::{ClaimKind, CodeChange, ExtractedClaim, FreshnessEngine, MemoryCategory, MemoryScope, MemoryStore};
use rune_specs::{new_requirement, new_specification, Coverage, SpecStore};
use rune_storage::Store;
use rune_tasks::{Task, TaskStore};
use rune_worktrees::{CreateWorktree, WorktreeManager};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=Rune Test",
            "-c",
            "user.email=rune@test.local",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Rune Test")
        .env("GIT_AUTHOR_EMAIL", "rune@test.local")
        .env("GIT_COMMITTER_NAME", "Rune Test")
        .env("GIT_COMMITTER_EMAIL", "rune@test.local")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn s080_full_cross_subsystem_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    git(&root, &["init", "--quiet"]);
    fs::write(root.join("README.md"), "token race\n").unwrap();
    git(&root, &["add", "README.md"]);
    git(&root, &["commit", "-m", "init"]);
    fs::write(
        root.join("store.rs"),
        "pub fn rotate_token() {}\n#[test]\nfn concurrent_refresh_test() { rotate_token(); }\n",
    )
    .unwrap();
    fs::write(
        root.join("controller.rs"),
        "pub fn refresh() { rotate_token(); }\n",
    )
    .unwrap();

    let store = Store::open_in_memory().unwrap();
    let indexer = Indexer::new(store.clone(), &root).unwrap();
    indexer.scan_workspace().unwrap();

    let functions = store.nodes_of_kind(NodeKind::Function).unwrap();
    let rotate = functions
        .iter()
        .find(|n| n.name.as_deref() == Some("rotate_token"))
        .expect("rotate_token indexed");
    let refresh = functions
        .iter()
        .find(|n| n.name.as_deref() == Some("refresh"))
        .expect("refresh indexed");
    assert!(store
        .find_edge(refresh.id, rotate.id, EdgeKind::Calls)
        .unwrap()
        .is_some());

    let session = Node::new(
        NodeKind::Session,
        Some("claude".into()),
        serde_json::json!({"provider": "claude", "goal": "fix refresh token race"}),
    );
    store.upsert_node(&session).unwrap();
    let memories = MemoryStore::new(&store);
    let memory = memories
        .ingest(ExtractedClaim {
            statement: "Authentication uses Redis sessions".into(),
            claim_kind: ClaimKind::ObservedFact,
            category: MemoryCategory::VerifiedFact,
            scope: MemoryScope::Repository,
            confidence: 0.9,
            evidence: Vec::new(),
            related_nodes: vec![rotate.id],
            actor: Some("human".into()),
        })
        .unwrap();

    fs::write(
        root.join("store.rs"),
        "pub fn rotate_token() { /* cookie jar */ }\n#[test]\nfn concurrent_refresh_test() { rotate_token(); }\n",
    )
    .unwrap();
    indexer.scan_workspace().unwrap();
    let mut change = CodeChange::default();
    change.symbol_ids = vec![rotate.id];
    FreshnessEngine::new(&store).apply(&change).unwrap();
    let stale = memories.get(memory.id).unwrap();
    assert_eq!(stale.validity, rune_core::Validity::Stale);

    let tests = store.nodes_of_kind(NodeKind::Test).unwrap();
    let test_node = tests
        .iter()
        .find(|n| n.name.as_deref() == Some("concurrent_refresh_test"))
        .expect("test indexed")
        .clone();

    let functions = store.nodes_of_kind(NodeKind::Function).unwrap();
    let rotate_now = functions
        .iter()
        .find(|n| n.name.as_deref() == Some("rotate_token"))
        .expect("rotate_token reindexed")
        .clone();

    let mut spec = new_specification("auth", "Need atomic refresh token rotation");
    spec.requirements = vec![new_requirement("REQ_4", "Rotation is atomic")];
    let spec = SpecStore::new(&store).create(spec).unwrap();
    let mut task = Task::new("AUTH_21", "Fix refresh token race");
    task.spec_links = vec![spec.id];
    task.affected_symbols = vec![rotate_now.id];
    let task = TaskStore::new(&store).create(task).unwrap();

    let files = store.nodes_of_kind(NodeKind::File).unwrap();
    let store_file = files
        .iter()
        .find(|n| n.name.as_deref() == Some("store.rs"))
        .unwrap();
    let impact = impact_for_files(&store, &[store_file.id]).unwrap();
    assert!(!impact.changed_symbols.is_empty());

    let compiler = ContextCompiler::new(&store);
    let empty = EmptyRetriever;
    let retrievers = Retrievers::empty(&empty);
    let mut req = CompileRequest::new("fix refresh token race", 4000);
    req.pins.pin(rotate_now.id);
    req.pins.pin(task.id);
    req.pins.pin(spec.id);
    let compiled = compiler.compile(req, &retrievers).unwrap();
    let found: BTreeSet<String> = compiled
        .capsule
        .included
        .iter()
        .map(|i| i.id.to_string())
        .collect();
    let expected = BTreeSet::from([rotate_now.id.to_string(), task.id.to_string(), spec.id.to_string()]);
    assert!(recall(&found, &expected) >= 1.0);

    let wt = WorktreeManager::new(store.clone(), &root).unwrap();
    let wt_path = root.join("wt-auth");
    wt.create(CreateWorktree {
        path: wt_path.clone(),
        branch: "task/auth-21".into(),
        create_branch: true,
        base_commit: None,
        task: Some(task.title.clone()),
        agent: Some("codex".into()),
    })
    .unwrap();
    fs::write(wt_path.join("store.rs"), "pub fn rotate_token() { atomic(); }\n").unwrap();

    indexer
        .record_test_run(
            &file_key(&root, &root.join("store.rs")),
            "concurrent_refresh_test",
            TestRun {
                at: Timestamp::now(),
                passed: true,
                run_id: Some("s080".into()),
            },
        )
        .unwrap();

    git(&root, &["add", "store.rs"]);
    git(&root, &["commit", "-m", "atomic rotation"]);
    let commit = Node::new(
        NodeKind::Commit,
        Some("atomic rotation".into()),
        serde_json::json!({"message": "atomic rotation"}),
    );
    store.upsert_node(&commit).unwrap();
    store
        .upsert_edge(&rune_core::Edge::new(session.id, commit.id, EdgeKind::ChangedBy))
        .unwrap();

    let handoff = HandoffCompiler::new(&store)
        .compile(
            session.clone(),
            "claude",
            "codex",
            "continue atomic rotation tests",
            HandoffMode::Balanced,
            None,
            &retrievers,
        )
        .unwrap();
    let target = Node::new(
        NodeKind::Session,
        Some("codex".into()),
        serde_json::json!({"provider": "codex", "goal": handoff.handoff.goal}),
    );
    store.upsert_node(&target).unwrap();
    HandoffCompiler::new(&store)
        .transfer(&handoff, &target)
        .unwrap();
    let neighbors = Graph::new(&store).neighbors(handoff.handoff.id).unwrap();
    assert!(neighbors.iter().any(|n| n.edge.kind == EdgeKind::HandedTo));

    SpecStore::new(&store)
        .link_satisfies_requirement(test_node.id, spec.requirements[0].id)
        .unwrap();
    let report = Coverage::new(&store)
        .for_specification(spec.id)
        .unwrap();
    assert!(report.requirements.iter().any(|item| item.covered));
}
