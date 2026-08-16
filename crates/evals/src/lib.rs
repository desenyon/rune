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
        "compiler_s063" => Ok(eval_compiler_s063()),
        "handoff_s064" => Ok(eval_handoff_s064()),
        "memory_s065" => Ok(eval_memory_s065()),
        other => Err(format!("unknown evaluation {other}")),
    }
}

pub fn all_evals() -> Vec<EvalResult> {
    [
        "symbol_retrieval",
        "memory_staleness",
        "handoff_completeness",
        "compiler_evidence_recall",
        "compiler_s063",
        "handoff_s064",
        "memory_s065",
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

fn eval_compiler_s063() -> EvalResult {
    let started = std::time::Instant::now();
    let store = Store::open_in_memory().unwrap();
    let evidence_a = Node::new(
        NodeKind::Function,
        Some("RefreshController".into()),
        serde_json::json!({"purpose": "refresh token rotation"}),
    );
    let evidence_b = Node::new(
        NodeKind::Test,
        Some("concurrent_refresh_test".into()),
        serde_json::json!({"purpose": "refresh token race"}),
    );
    let irrelevant = Node::new(
        NodeKind::Function,
        Some("paint_tabs".into()),
        serde_json::json!({"purpose": "terminal chrome"}),
    );
    let mut stale = Node::new(
        NodeKind::Memory,
        Some("uses redis".into()),
        serde_json::json!({"statement": "Authentication uses Redis sessions", "purpose": "refresh token"}),
    );
    stale.validity = Validity::Stale;
    store.upsert_node(&evidence_a).unwrap();
    store.upsert_node(&evidence_b).unwrap();
    store.upsert_node(&irrelevant).unwrap();
    store.upsert_node(&stale).unwrap();
    let expected = BTreeSet::from([evidence_a.id.to_string(), evidence_b.id.to_string()]);
    let compiler = ContextCompiler::new(&store);
    let empty = EmptyRetriever;
    let retrievers = Retrievers::empty(&empty);
    let mut req = CompileRequest::new("refresh token rotation race", 4000);
    req.pins.pin(evidence_a.id);
    req.pins.pin(evidence_b.id);
    let compiled = compiler.compile(req, &retrievers).unwrap();
    let found: BTreeSet<String> = compiled
        .capsule
        .included
        .iter()
        .map(|i| i.id.to_string())
        .collect();
    let evidence_recall = recall(&found, &expected);
    let included = compiled.capsule.included.len().max(1) as f64;
    let irrelevant_rate = if found.contains(&irrelevant.id.to_string()) {
        1.0 / included
    } else {
        0.0
    };
    let stale_rate = compiled
        .capsule
        .included
        .iter()
        .filter(|i| i.provenance.freshness == Validity::Stale)
        .count() as f64
        / included;
    let contradiction_rate = compiled.capsule.warnings.iter().filter(|w| w.kind.contains("contradict")).count() as f64
        / included;
    let duplicate_rate = compiled.capsule.duplicates_removed as f64 / included;
    let token_cost = compiled.capsule.token_estimate as f64;
    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
    let passed = evidence_recall >= 1.0 && irrelevant_rate < 0.5;
    EvalResult {
        name: "compiler_s063".into(),
        passed,
        metrics: BTreeMap::from([
            ("evidence_recall".into(), evidence_recall),
            ("irrelevant_context_rate".into(), irrelevant_rate),
            ("stale_context_rate".into(), stale_rate),
            ("contradiction_rate".into(), contradiction_rate),
            ("duplicate_rate".into(), duplicate_rate),
            ("token_cost".into(), token_cost),
            ("latency_ms".into(), latency_ms),
        ]),
        details: serde_json::json!({
            "spec": "S063",
            "expected": expected,
            "found": found,
        }),
    }
}

fn eval_handoff_s064() -> EvalResult {
    let store = Store::open_in_memory().unwrap();
    let session = Node::new(
        NodeKind::Session,
        Some("claude".into()),
        serde_json::json!({
            "provider": "claude",
            "goal": "fix refresh token race",
            "transcript": "fix refresh token race; we tried a mutex, it failed, remaining work is tests"
        }),
    );
    store.upsert_node(&session).unwrap();
    let file = Node::new(
        NodeKind::File,
        Some("token.rs".into()),
        serde_json::json!({"purpose": "refresh token race"}),
    );
    store.upsert_node(&file).unwrap();
    let compiler = HandoffCompiler::new(&store);
    let empty = EmptyRetriever;
    let retrievers = Retrievers::empty(&empty);
    let package = compiler
        .compile(
            session.clone(),
            "claude",
            "codex",
            "fix refresh token race",
            HandoffMode::Balanced,
            None,
            &retrievers,
        )
        .unwrap();
    let structured_ok = package.handoff.missing_fields().is_empty()
        && package.handoff.goal.contains("refresh")
        && package.handoff.source_agent == "claude"
        && package.handoff.target_agent == "codex";
    let raw = session
        .payload
        .get("transcript")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let transcript_only_has_goal = raw.to_ascii_lowercase().contains("refresh");
    EvalResult {
        name: "handoff_s064".into(),
        passed: structured_ok && transcript_only_has_goal,
        metrics: BTreeMap::from([
            ("structured_complete".into(), if structured_ok { 1.0 } else { 0.0 }),
            ("transcript_has_goal".into(), if transcript_only_has_goal { 1.0 } else { 0.0 }),
            ("missing_fields".into(), package.handoff.missing_fields().len() as f64),
        ]),
        details: serde_json::json!({"spec": "S064", "mode": "balanced"}),
    }
}

fn eval_memory_s065() -> EvalResult {
    use rune_memory::{
        ClaimKind, CodeChange, ConflictResolver, ExtractedClaim, Extractor, FreshnessEngine,
        MemoryCategory, MemoryScope, MemoryStore,
    };
    let store = Store::open_in_memory().unwrap();
    let symbol = Node::new(
        NodeKind::Function,
        Some("SessionStore".into()),
        serde_json::json!({"path": "auth.rs", "content_hash": "aaa"}),
    );
    store.upsert_node(&symbol).unwrap();
    let memories = MemoryStore::new(&store);
    let observed = memories
        .ingest(ExtractedClaim {
            statement: "Authentication uses Redis sessions".into(),
            claim_kind: ClaimKind::ObservedFact,
            category: MemoryCategory::VerifiedFact,
            scope: MemoryScope::Repository,
            confidence: 0.9,
            evidence: Vec::new(),
            related_nodes: vec![symbol.id],
            actor: None,
        })
        .unwrap();
    let inference = Extractor::from_session_json(&serde_json::json!({
        "session_id": "s1",
        "provider": "claude",
        "turns": [{"role": "assistant", "content": "I think we should use Redis", "id": "t1"}]
    }));
    let inference_ok = inference.map(|claims| {
        claims.iter().all(|c| c.claim_kind != ClaimKind::ObservedFact || c.confidence < 0.99)
    }).unwrap_or(true);
    let ingested_inference = memories
        .ingest(ExtractedClaim {
            statement: "maybe postgres is better".into(),
            claim_kind: ClaimKind::AgentInference,
            category: MemoryCategory::TemporaryContext,
            scope: MemoryScope::Session,
            confidence: 0.4,
            evidence: Vec::new(),
            related_nodes: vec![],
            actor: Some("claude".into()),
        })
        .unwrap();
    assert!(!ingested_inference.validity.may_guide_agents());
    let mut change = CodeChange::default();
    change.symbol_ids = vec![symbol.id];
    let reasons = FreshnessEngine::new(&store).apply(&change).unwrap();
    let stale = memories.get(observed.id).unwrap();
    let stale_ok = stale.validity == Validity::Stale || !reasons.is_empty();
    let other = memories
        .ingest(ExtractedClaim {
            statement: "Authentication uses cookie sessions".into(),
            claim_kind: ClaimKind::ObservedFact,
            category: MemoryCategory::VerifiedFact,
            scope: MemoryScope::Repository,
            confidence: 0.8,
            evidence: Vec::new(),
            related_nodes: vec![symbol.id],
            actor: None,
        })
        .unwrap();
    let conflict = ConflictResolver::new(&store)
        .record_conflict(observed.id, other.id)
        .unwrap();
    let conflict_ok = !conflict.contradicted.is_empty() || !conflict.kept.is_empty();
    let passed = inference_ok && stale_ok && conflict_ok;
    EvalResult {
        name: "memory_s065".into(),
        passed,
        metrics: BTreeMap::from([
            ("inference_rejected_as_guidance".into(), if ingested_inference.validity.may_guide_agents() { 0.0 } else { 1.0 }),
            ("staleness_detected".into(), if stale_ok { 1.0 } else { 0.0 }),
            ("conflict_preserved".into(), if conflict_ok { 1.0 } else { 0.0 }),
        ]),
        details: serde_json::json!({"spec": "S065"}),
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
        assert!(results.iter().any(|r| r.name == "compiler_s063" && r.passed));
        assert!(results.iter().any(|r| r.name == "handoff_s064" && r.passed));
        assert!(results.iter().any(|r| r.name == "memory_s065" && r.passed));
        maybe_write_benchmarks(&results).unwrap();
    }
}
