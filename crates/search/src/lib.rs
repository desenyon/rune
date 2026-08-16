//! Universal search across every indexed Rune object.
//!
//! Modes: exact, fuzzy (nucleo), full text (FTS5), structural, graph, temporal,
//! and hybrid. [`SearchRouter`] selects a mode from query intent; callers may
//! force a mode through [`SearchRequest::mode`].

mod catalog;
mod engine;
mod error;
mod mode;
mod router;

pub use engine::{resolve_named, SearchEngine};
pub use error::{Result, SearchError};
pub use mode::{QueryIntent, SearchHit, SearchMode, SearchRequest, SearchResponse};
pub use router::{extract_quoted, parse_path_query, SearchRouter};

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::{Edge, EdgeKind, Node, NodeKind};
    use rune_storage::Store;

    fn seed() -> (Store, Node, Node, Node) {
        let store = Store::open_in_memory().unwrap();
        let repo = Node::new(NodeKind::Repository, Some("repo".into()), serde_json::json!({}));
        let file = Node::new(NodeKind::File, Some("lib.rs".into()), serde_json::json!({}));
        let symbol = Node::new(
            NodeKind::Function,
            Some("authenticate".into()),
            serde_json::json!({"signature": "fn authenticate()"}),
        );
        store.upsert_node(&repo).unwrap();
        store.upsert_node(&file).unwrap();
        store.upsert_node(&symbol).unwrap();
        store
            .upsert_edge(&Edge::new(repo.id, file.id, EdgeKind::Contains))
            .unwrap();
        store
            .upsert_edge(&Edge::new(file.id, symbol.id, EdgeKind::Defines))
            .unwrap();
        (store, repo, file, symbol)
    }

    #[test]
    fn search_router_picks_graph_for_path_queries() {
        let intent = SearchRouter::analyze("path from repo to authenticate");
        assert_eq!(intent.mode, SearchMode::Graph);
        assert_eq!(intent.path_from.as_deref(), Some("repo"));
        assert_eq!(intent.path_to.as_deref(), Some("authenticate"));
        let arrow = SearchRouter::analyze("path repo -> authenticate");
        assert_eq!(arrow.mode, SearchMode::Graph);
    }

    #[test]
    fn router_picks_structural_for_fn_prefix() {
        let intent = SearchRouter::analyze("fn authenticate");
        assert_eq!(intent.mode, SearchMode::Structural);
        assert_eq!(intent.structural_name.as_deref(), Some("authenticate"));
    }

    #[test]
    fn router_picks_fts_for_quoted_strings() {
        let intent = SearchRouter::analyze("find \"token rotation\"");
        assert_eq!(intent.mode, SearchMode::FullText);
        assert_eq!(intent.quoted.as_deref(), Some("token rotation"));
    }

    #[test]
    fn forced_mode_overrides_router() {
        let (store, repo, _, _) = seed();
        let engine = SearchEngine::new(&store);
        let response = engine
            .search(SearchRequest::new("path from repo to authenticate").with_mode(SearchMode::Exact))
            .unwrap();
        assert_eq!(response.mode, SearchMode::Exact);
        assert!(response.hits.iter().all(|hit| hit.node.id != repo.id || hit.mode == SearchMode::Exact));
        assert!(response.hits.is_empty());
    }

    #[test]
    fn graph_search_returns_path_nodes() {
        let (store, repo, file, symbol) = seed();
        let engine = SearchEngine::new(&store);
        let response = engine
            .search(SearchRequest::new("path from repo to authenticate"))
            .unwrap();
        assert_eq!(response.mode, SearchMode::Graph);
        let ids: Vec<_> = response.hits.iter().map(|hit| hit.node.id).collect();
        assert_eq!(ids, vec![repo.id, file.id, symbol.id]);
    }

    #[test]
    fn exact_and_fuzzy_find_function() {
        let (store, _, _, symbol) = seed();
        let engine = SearchEngine::new(&store);
        let exact = engine
            .search(SearchRequest::new("authenticate").with_mode(SearchMode::Exact))
            .unwrap();
        assert_eq!(exact.hits[0].node.id, symbol.id);
        let fuzzy = engine
            .search(SearchRequest::new("authen").with_mode(SearchMode::Fuzzy))
            .unwrap();
        assert!(fuzzy.hits.iter().any(|hit| hit.node.id == symbol.id));
    }

    #[test]
    fn search_mode_parses_aliases() {
        assert_eq!("fts".parse::<SearchMode>().unwrap(), SearchMode::FullText);
        assert_eq!("symbol".parse::<SearchMode>().unwrap(), SearchMode::Structural);
        assert!("nope".parse::<SearchMode>().is_err());
    }
}
