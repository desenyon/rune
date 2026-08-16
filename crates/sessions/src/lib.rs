//! Agent session discovery, import, and deterministic transcript intelligence.

pub mod adapters;
pub mod error;
pub mod extract;
pub mod import;
pub mod io;
pub mod model;
pub mod provider;

pub use adapters::all_sources;
pub use error::{Result, SessionError};
pub use extract::{extract, ExtractedItem, SessionIntelligence};
pub use import::{persist, PersistedSession};
pub use model::{DiscoveredSession, DiscoveryContext, NormalizedSession, NormalizedTurn};
pub use provider::{file_only_capabilities, SessionProvider, SessionSource};

use rune_providers::ProviderRegistry;

pub fn register_default_adapters(registry: &mut ProviderRegistry) {
    registry.register(Box::new(SessionProvider::new(
        adapters::claude::ClaudeCodeAdapter,
    )));
    registry.register(Box::new(SessionProvider::new(
        adapters::codex::CodexAdapter,
    )));
    registry.register(Box::new(SessionProvider::new(
        adapters::cursor::CursorAdapter,
    )));
    registry.register(Box::new(SessionProvider::new(
        adapters::opencode::OpenCodeAdapter,
    )));
    registry.register(Box::new(SessionProvider::new(
        adapters::gemini::GeminiAdapter,
    )));
    registry.register(Box::new(SessionProvider::new(
        adapters::aider::AiderAdapter,
    )));
}

pub fn all_session_providers() -> Vec<SessionProvider<Box<dyn SessionSource>>> {
    all_sources()
        .into_iter()
        .map(SessionProvider::new)
        .collect()
}

/// S075: import every discoverable local session using AgentSession provenance.
pub fn import_discovered(
    store: &rune_storage::Store,
    ctx: &DiscoveryContext,
) -> Result<Vec<PersistedSession>> {
    let mut imported = Vec::new();
    for source in all_sources() {
        for discovered in source.discover(ctx)? {
            let normalized = source.read_normalized(&discovered)?;
            imported.push(persist(store, &normalized)?);
        }
    }
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::{EdgeKind, NodeKind, Validity};
    use rune_providers::{Capability, Provider};
    use rune_storage::Store;
    use std::fs;
    use tempfile::tempdir;

    fn providers() -> Vec<Box<dyn Provider>> {
        vec![
            Box::new(SessionProvider::new(adapters::claude::ClaudeCodeAdapter)),
            Box::new(SessionProvider::new(adapters::codex::CodexAdapter)),
            Box::new(SessionProvider::new(adapters::cursor::CursorAdapter)),
            Box::new(SessionProvider::new(adapters::opencode::OpenCodeAdapter)),
            Box::new(SessionProvider::new(adapters::gemini::GeminiAdapter)),
            Box::new(SessionProvider::new(adapters::aider::AiderAdapter)),
        ]
    }

    #[test]
    fn adapters_do_not_declare_continuation_if_not_implemented() {
        for provider in providers() {
            let caps = provider.capabilities();
            assert!(
                !caps.contains(&Capability::SessionContinuation),
                "{} must not declare continuation",
                provider.identity().id
            );
            assert!(!caps.contains(&Capability::ContextInjection));
            assert!(!caps.contains(&Capability::CommandInvocation));
            assert!(!caps.contains(&Capability::Handoff));
            assert!(!caps.contains(&Capability::StreamingEvents));
            assert!(caps.contains(&Capability::SessionDiscovery));
            assert!(caps.contains(&Capability::SessionImport));
        }
    }

    #[test]
    fn missing_session_dir_yields_empty_list_not_error() {
        let dir = tempdir().unwrap();
        let ctx = DiscoveryContext::isolated(dir.path(), None);
        for source in all_sources() {
            let found = source.discover(&ctx).expect("missing dirs are empty");
            assert!(
                found.is_empty(),
                "{} should return empty when roots are missing",
                source.identity().id
            );
        }
    }

    #[test]
    fn extraction_from_fixture_creates_attempt_and_failure_linked_to_turns() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        fs::write(
            &path,
            r#"{"type":"user","uuid":"turn-1","message":{"role":"user","content":"Fix token rotation in TokenStore.rs"}}
{"type":"assistant","uuid":"turn-2","message":{"role":"assistant","content":"I'll try adding a mutex around rotate(). Attempting to lock Redis."}}
{"type":"assistant","uuid":"turn-3","message":{"role":"assistant","content":"The attempt failed: error: non_atomic_rotation still reproduces. The lock didn't work."}}
"#,
        )
        .unwrap();
        let discovered = DiscoveredSession {
            provider: "claude-code".into(),
            external_id: "fixture".into(),
            path,
            title: Some("fixture".into()),
        };
        let normalized = adapters::claude::ClaudeCodeAdapter
            .read_normalized(&discovered)
            .unwrap();
        assert_eq!(normalized.turns.len(), 3);
        let store = Store::open_in_memory().unwrap();
        let persisted = persist(&store, &normalized).unwrap();
        let attempts = store.nodes_of_kind(NodeKind::Attempt).unwrap();
        let failures = store.nodes_of_kind(NodeKind::Failure).unwrap();
        assert!(
            !attempts.is_empty(),
            "expected Attempt nodes from heuristic"
        );
        assert!(
            !failures.is_empty(),
            "expected Failure nodes from heuristic"
        );
        for attempt in &attempts {
            assert_eq!(attempt.validity, Validity::Candidate);
            assert_ne!(attempt.validity, Validity::Verified);
            let edges = store
                .edges_from_kind(attempt.id, EdgeKind::AttemptedIn)
                .unwrap();
            assert!(
                edges
                    .iter()
                    .any(|edge| persisted.turn_ids.contains(&edge.to)),
                "attempt must link to a source turn"
            );
        }
        for failure in &failures {
            assert_eq!(failure.validity, Validity::Candidate);
            let edges = store
                .edges_from_kind(failure.id, EdgeKind::FailedIn)
                .unwrap();
            assert!(
                edges
                    .iter()
                    .any(|edge| persisted.turn_ids.contains(&edge.to)),
                "failure must link to a source turn"
            );
            let provenance = store.provenance_for_node(failure.id).unwrap();
            assert!(provenance.iter().any(|p| p.derived));
        }
        let session_prov = store.provenance_for_node(persisted.session_id).unwrap();
        assert!(session_prov
            .iter()
            .any(|p| !p.derived && p.source.kind_name() == "agent_session"));
    }

    #[test]
    fn session_import_extracts_candidate_agent_memory_not_guidance() {
        use rune_memory::{MemoryStore, RetrievalMode};

        let dir = tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        fs::write(
            &path,
            r#"{"type":"user","uuid":"turn-1","message":{"role":"user","content":"Please prefer PostgreSQL for sessions."}}
{"type":"assistant","uuid":"turn-2","message":{"role":"assistant","content":"I think Redis sessions would be faster."}}
"#,
        )
        .unwrap();
        let discovered = DiscoveredSession {
            provider: "claude-code".into(),
            external_id: "mem-fixture".into(),
            path,
            title: Some("mem-fixture".into()),
        };
        let normalized = adapters::claude::ClaudeCodeAdapter
            .read_normalized(&discovered)
            .unwrap();
        let store = Store::open_in_memory().unwrap();
        let persisted = persist(&store, &normalized).unwrap();
        assert!(
            !persisted.memory_ids.is_empty(),
            "expected memories from session turns"
        );
        let memories = MemoryStore::new(&store);
        let records = memories.list().unwrap();
        assert!(records.iter().any(|record| {
            record.statement.to_ascii_lowercase().contains("redis")
                && record.validity == Validity::Candidate
                && !record.may_guide_agents()
        }));
        assert!(records.iter().any(|record| {
            record.statement.to_ascii_lowercase().contains("prefer")
                && record.validity == Validity::Verified
        }));
        let guidance = memories.retrieve(RetrievalMode::AgentGuidance).unwrap();
        assert!(guidance
            .iter()
            .all(|record| !record.statement.to_ascii_lowercase().contains("redis")));
    }
}
