//! Store-backed retrievers that honor memory validity and task/spec stores.

use rune_context_compiler::{
    candidate_from_node, Candidate, MemoryRetriever, SpecRetriever, TaskRetriever,
};
use rune_context_compiler::{CompilerError, Result};
use rune_core::Validity;
use rune_memory::{MemoryStore, RetrievalMode};
use rune_specs::SpecStore;
use rune_storage::Store;
use rune_tasks::TaskStore;

fn keyword_hits(hay: &str, keywords: &[String]) -> usize {
    let hay = hay.to_ascii_lowercase();
    keywords
        .iter()
        .filter(|k| hay.contains(&k.to_ascii_lowercase()))
        .count()
}

fn relevance(hits: usize, keywords: &[String]) -> f32 {
    if keywords.is_empty() {
        0.2
    } else {
        hits as f32 / keywords.len() as f32
    }
}

pub struct StoreTaskRetriever;

impl TaskRetriever for StoreTaskRetriever {
    fn retrieve(
        &self,
        intent: &rune_context_compiler::Intent,
        store: &Store,
    ) -> Result<Vec<Candidate>> {
        let mut out = Vec::new();
        for task in TaskStore::new(store)
            .list()
            .map_err(|err| CompilerError::Message(err.to_string()))?
        {
            let node = store.get_node(task.id)?;
            if node.validity == Validity::Archived {
                continue;
            }
            let hay = format!("{} {}", task.title, task.description);
            let hits = keyword_hits(&hay, &intent.keywords);
            if hits == 0 && !intent.keywords.is_empty() {
                continue;
            }
            out.push(candidate_from_node(
                node,
                "task_store",
                vec!["task_store".into()],
                relevance(hits, &intent.keywords),
            ));
        }
        Ok(out)
    }
}

pub struct StoreSpecRetriever;

impl SpecRetriever for StoreSpecRetriever {
    fn retrieve(
        &self,
        intent: &rune_context_compiler::Intent,
        store: &Store,
    ) -> Result<Vec<Candidate>> {
        let specs = SpecStore::new(store);
        let mut out = Vec::new();
        for spec in specs
            .list()
            .map_err(|err| CompilerError::Message(err.to_string()))?
        {
            let node = store.get_node(spec.id)?;
            if node.validity == Validity::Archived {
                continue;
            }
            let hay = format!("{} {} {}", spec.name, spec.problem, spec.desired_behavior);
            let hits = keyword_hits(&hay, &intent.keywords);
            if hits == 0 && !intent.keywords.is_empty() {
                continue;
            }
            out.push(candidate_from_node(
                node,
                "spec_store",
                vec!["spec_store".into()],
                relevance(hits, &intent.keywords),
            ));
            for requirement in spec.requirements {
                let req_node = store.get_node(requirement.id)?;
                let req_hay = format!("{} {}", requirement.key, requirement.text);
                let req_hits = keyword_hits(&req_hay, &intent.keywords);
                if req_hits == 0 && !intent.keywords.is_empty() {
                    continue;
                }
                out.push(candidate_from_node(
                    req_node,
                    "spec_store",
                    vec!["spec_store".into(), "requirement".into()],
                    relevance(req_hits, &intent.keywords),
                ));
            }
        }
        Ok(out)
    }
}

pub struct GuidanceMemoryRetriever;

impl MemoryRetriever for GuidanceMemoryRetriever {
    fn retrieve(
        &self,
        intent: &rune_context_compiler::Intent,
        store: &Store,
    ) -> Result<Vec<Candidate>> {
        let memories = MemoryStore::new(store)
            .retrieve(RetrievalMode::AgentGuidance)
            .map_err(|err| CompilerError::Message(err.to_string()))?;
        let mut out = Vec::new();
        for record in memories {
            let node = store.get_node(record.id)?;
            let hits = keyword_hits(&record.statement, &intent.keywords);
            if hits == 0 && !intent.keywords.is_empty() {
                continue;
            }
            out.push(candidate_from_node(
                node,
                "memory_guidance",
                vec!["memory_store".into(), "agent_guidance".into()],
                relevance(hits, &intent.keywords),
            ));
        }
        Ok(out)
    }
}
