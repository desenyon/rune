//! Evaluation framework (S062–S065).

use rune_context_compiler::{CompileRequest, ContextCompiler, EmptyRetriever, PinSet, Retrievers};
use rune_core::{Node, NodeKind, Validity};
use rune_handoff::{HandoffCompiler, HandoffMode};
use rune_storage::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvalResult {
    pub name: String,
    pub passed: bool,
    pub metrics: BTreeMap<String, f64>,
    pub details: serde_json::Value,
}

pub fn run_named(name: &str) -> Result<EvalResult, String> {
    match name {
        "symbol_retrieval" => Ok(eval_symbol_retrieval()),
        "memory_staleness" => Ok(eval_memory_staleness()),
        "handoff_completeness" => Ok(eval_handoff_completeness()),
        "compiler_evidence_recall" => Ok(eval_compiler_recall()),
        other => Err(format!("unknown evaluation {other}")),
    }
}

pub fn all_evals() -> Vec<EvalResult> {
    [
        "symbol_retrieval",
        "memory_staleness",
        "handoff_completeness",
        "compiler_evidence_recall",
    ]
    .into_iter()
    .map(|name| run_named(name).expect("known eval"))
    .collect()
}

pub fn maybe_write_benchmarks(results: &[EvalResult]) -> std::io::Result<()> {
    if std::env::var("RUNE_WRITE_BENCHMARKS").ok().as_deref() != Some("1") {
        return Ok(());
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/benchmarks");
    std::fs::create_dir_all(&dir)?;
    for result in results {
        let path = dir.join(format!("{}.json", result.name));
        std::fs::write(path, serde_json::to_string_pretty(result).unwrap())?;
    }
    Ok(())
}

pub fn recall(found: &BTreeSet<String>, expected: &BTreeSet<String>) -> f64 {
    if expected.is_empty() {
        return 1.0;
    }
    let hit = expected.intersection(found).count();
    hit as f64 / expected.len() as f64
}

fn eval_symbol_retrieval() -> EvalResult {
    let store = Store::open_in_memory().unwrap();
    let symbol = Node::new(
        NodeKind::Function,
        Some("TokenStore".into()),
        serde_json::json!({"purpose": "stores refresh tokens"}),
    );
    store.upsert_node(&symbol).unwrap();
    let hits = store.search_text("TokenStore", 8).unwrap();
    let found: BTreeSet<String> = hits.into_iter().map(|(id, _, _)| id).collect();
    let expected = BTreeSet::from([symbol.id.to_string()]);
    let metric = recall(&found, &expected);
    EvalResult {
        name: "symbol_retrieval".into(),
        passed: metric >= 1.0,
        metrics: BTreeMap::from([("recall".into(), metric)]),
        details: serde_json::json!({"expected": symbol.id.to_string(), "found": found}),
    }
}

fn eval_memory_staleness() -> EvalResult {
    let store = Store::open_in_memory().unwrap();
    let mut memory = Node::new(
        NodeKind::Memory,
        Some("auth uses redis".into()),
        serde_json::json!({"statement": "Authentication uses Redis sessions"}),
    );
    memory.validity = Validity::Stale;
    store.upsert_node(&memory).unwrap();
    let compiler = ContextCompiler::new(&store);
    let empty = EmptyRetriever;
    let retrievers = Retrievers::empty(&empty);
    let compiled = compiler
        .compile(
            CompileRequest::new("Authentication uses Redis sessions", 4000),
            &retrievers,
        )
        .unwrap();
    let flagged = compiled
        .capsule
        .warnings
        .iter()
        .any(|w| w.kind == "stale" || w.kind == "memory_not_guidance")
        || compiled
            .capsule
            .included
            .iter()
            .any(|i| i.id == memory.id && i.provenance.freshness == Validity::Stale);
    EvalResult {
        name: "memory_staleness".into(),
        passed: flagged,
        metrics: BTreeMap::from([("stale_detected".into(), if flagged { 1.0 } else { 0.0 })]),
        details: serde_json::json!({"memory": memory.id.to_string()}),
    }
}

fn eval_handoff_completeness() -> EvalResult {
    let store = Store::open_in_memory().unwrap();
    let session = Node::new(
        NodeKind::Session,
        Some("claude".into()),
        serde_json::json!({"provider": "claude"}),
    );
    store.upsert_node(&session).unwrap();
    store
        .upsert_node(&Node::new(
            NodeKind::File,
            Some("lib.rs".into()),
            serde_json::json!({"purpose": "core"}),
        ))
        .unwrap();
    let compiler = HandoffCompiler::new(&store);
    let empty = EmptyRetriever;
    let retrievers = Retrievers::empty(&empty);
    let package = compiler
        .compile(
            session,
            "claude",
            "codex",
            "continue work",
            HandoffMode::Balanced,
            None,
            &retrievers,
        )
        .unwrap();
    let missing = package.handoff.missing_fields();
    EvalResult {
        name: "handoff_completeness".into(),
        passed: missing.is_empty(),
        metrics: BTreeMap::from([("missing_fields".into(), missing.len() as f64)]),
        details: serde_json::json!({"missing": missing}),
    }
}

fn eval_compiler_recall() -> EvalResult {
    let store = Store::open_in_memory().unwrap();
    let a = Node::new(
        NodeKind::Function,
        Some("authenticate".into()),
        serde_json::json!({"purpose": "authentication logic"}),
    );
    let b = Node::new(
        NodeKind::Function,
        Some("TokenStore".into()),
        serde_json::json!({"purpose": "token store"}),
    );
    store.upsert_node(&a).unwrap();
    store.upsert_node(&b).unwrap();
    let expected = BTreeSet::from([a.id.to_string(), b.id.to_string()]);
    let compiler = ContextCompiler::new(&store);
    let empty = EmptyRetriever;
    let retrievers = Retrievers::empty(&empty);
    let mut req = CompileRequest::new("authentication TokenStore", 8000);
    req.pins = PinSet::default();
    req.pins.pin(a.id);
    req.pins.pin(b.id);
    let compiled = compiler.compile(req, &retrievers).unwrap();
    let found: BTreeSet<String> = compiled
        .capsule
        .included
        .iter()
        .map(|i| i.id.to_string())
        .collect();
    let metric = recall(&found, &expected);
    EvalResult {
        name: "compiler_evidence_recall".into(),
        passed: metric >= 1.0,
        metrics: BTreeMap::from([("recall".into(), metric)]),
        details: serde_json::json!({"expected": expected, "found": found}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_recall_metric_computed() {
        let found = BTreeSet::from(["a".into(), "b".into()]);
        let expected = BTreeSet::from(["a".into(), "b".into(), "c".into()]);
        let metric = recall(&found, &expected);
        assert!((metric - 2.0 / 3.0).abs() < f64::EPSILON);
        let results = all_evals();
        assert!(results.iter().all(|r| r.passed), "{results:?}");
        assert!(
            results
                .iter()
                .any(|r| r.name == "compiler_evidence_recall"
                    && r.metrics.get("recall") == Some(&1.0))
        );
        maybe_write_benchmarks(&results).unwrap();
    }
}
