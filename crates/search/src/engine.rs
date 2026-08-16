use nucleo::pattern::{CaseMatching, Normalization, Pattern};
use nucleo::{Config, Matcher};
use rune_core::{EdgeKind, Node, NodeId, NodeKind, Timestamp};
use rune_graph::{ExpandFilter, Graph};
use rune_security::UntrustedContent;
use rune_storage::Store;
use std::collections::HashMap;
use std::str::FromStr;

use crate::catalog::{haystack, load_nodes};
use crate::error::{Result, SearchError};
use crate::mode::{SearchHit, SearchMode, SearchRequest, SearchResponse};
use crate::router::{parse_path_query, parse_structural_prefix, require_non_empty, SearchRouter};
use crate::semantic::{cosine, hash_embed};

pub struct SearchEngine<'a> {
    store: &'a Store,
}

impl<'a> SearchEngine<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn search(&self, mut request: SearchRequest) -> Result<SearchResponse> {
        if request.limit == 0 {
            return Err(SearchError::invalid("limit must be greater than zero"));
        }
        SearchRouter::apply_intent(&mut request);
        let wrapped = UntrustedContent::wrap("search.query", &request.query);
        let _ = wrapped.as_instruction();
        request.query = wrapped.body;
        let intent = SearchRouter::analyze(&request.query);
        let mode = request.mode.ok_or_else(|| {
            SearchError::invalid("search mode missing after routing")
        })?;
        tracing::debug!(mode = mode.as_str(), query = %request.query, "executing search");
        let mut response = match mode {
            SearchMode::Exact => self.exact(&request)?,
            SearchMode::Fuzzy => self.fuzzy(&request)?,
            SearchMode::FullText => self.full_text(&request)?,
            SearchMode::Structural => self.structural(&request, &intent)?,
            SearchMode::Semantic => self.semantic(&request)?,
            SearchMode::Graph => self.graph(&request, &intent)?,
            SearchMode::Temporal => self.temporal(&request)?,
            SearchMode::Hybrid => self.hybrid(&request, &intent)?,
        };
        response.mode = mode;
        if response.hits.len() > request.limit {
            response.hits.truncate(request.limit);
        }
        Ok(response)
    }

    fn exact(&self, request: &SearchRequest) -> Result<SearchResponse> {
        require_non_empty(&request.query)?;
        let query = request.query.trim();
        let mut hits = Vec::new();
        if let Ok(id) = NodeId::from_str(query) {
            let node = self.store.get_node(id)?;
            if kind_allowed(&node, &request.kinds) {
                hits.push(hit(node, 1.0, SearchMode::Exact, "matched node id"));
            }
            return Ok(response(SearchMode::Exact, hits, Vec::new()));
        }
        let nodes = load_nodes(self.store, &request.kinds)?;
        for node in nodes {
            if node.name.as_deref() == Some(query) {
                hits.push(hit(node, 1.0, SearchMode::Exact, "matched node name"));
            }
        }
        Ok(response(SearchMode::Exact, hits, Vec::new()))
    }

    fn fuzzy(&self, request: &SearchRequest) -> Result<SearchResponse> {
        require_non_empty(&request.query)?;
        let nodes = load_nodes(self.store, &request.kinds)?;
        let pattern = Pattern::parse(&request.query, CaseMatching::Smart, Normalization::Smart);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let items: Vec<FuzzyItem> = nodes
            .into_iter()
            .map(|node| FuzzyItem {
                text: haystack(&node),
                node,
            })
            .collect();
        let mut matched = pattern.match_list(items, &mut matcher);
        matched.sort_by(|a, b| b.1.cmp(&a.1));
        let hits = matched
            .into_iter()
            .take(request.limit)
            .map(|(item, score)| {
                hit(
                    item.node,
                    f64::from(score),
                    SearchMode::Fuzzy,
                    "nucleo fuzzy match against name, id, and kind",
                )
            })
            .collect();
        Ok(response(SearchMode::Fuzzy, hits, Vec::new()))
    }

    fn full_text(&self, request: &SearchRequest) -> Result<SearchResponse> {
        require_non_empty(&request.query)?;
        let rows = self.store.search_text(&request.query, request.limit)?;
        let mut hits = Vec::new();
        for (id, _kind, rank) in rows {
            let node_id = NodeId::from_str(&id).map_err(|err| SearchError::msg(err.to_string()))?;
            let node = self.store.get_node(node_id)?;
            if !kind_allowed(&node, &request.kinds) {
                continue;
            }
            // bm25() in FTS5 is lower-is-better; invert so ranking is consistent.
            hits.push(hit(
                node,
                -rank,
                SearchMode::FullText,
                "sqlite fts5 match",
            ));
        }
        Ok(response(SearchMode::FullText, hits, Vec::new()))
    }

    fn structural(&self, request: &SearchRequest, intent: &crate::QueryIntent) -> Result<SearchResponse> {
        let center = if let Some(id) = request.around {
            self.store.get_node(id)?
        } else if let Some(name) = intent.structural_name.as_deref() {
            let kind = intent
                .structural_kind
                .as_deref()
                .map(NodeKind::parse)
                .unwrap_or(NodeKind::Function);
            resolve_named(self.store, name, Some(kind))?
        } else {
            require_non_empty(&request.query)?;
            resolve_named(self.store, request.query.trim(), None)?
        };
        let edge_kinds = if request.edge_kinds.is_empty() {
            vec![
                EdgeKind::Calls,
                EdgeKind::Defines,
                EdgeKind::References,
                EdgeKind::Implements,
                EdgeKind::Imports,
                EdgeKind::Exports,
                EdgeKind::Tests,
                EdgeKind::Contains,
            ]
        } else {
            request.edge_kinds.clone()
        };
        let graph = Graph::new(self.store);
        let nodes = graph.expand(
            center.id,
            ExpandFilter {
                node_kinds: request.kinds.clone(),
                edge_kinds: edge_kinds.clone(),
                depth: request.max_depth.max(1),
            },
        )?;
        let mut hits = Vec::new();
        for (index, node) in nodes.into_iter().enumerate() {
            let score = if index == 0 { 1.0 } else { 0.85 / index as f64 };
            let reason = if index == 0 {
                "structural center".to_string()
            } else {
                format!("neighbor via {:?}", edge_kinds)
            };
            hits.push(hit(node, score, SearchMode::Structural, reason));
        }
        Ok(response(SearchMode::Structural, hits, Vec::new()))
    }

    fn semantic(&self, request: &SearchRequest) -> Result<SearchResponse> {
        require_non_empty(&request.query)?;
        let query_vec = hash_embed(&request.query);
        let nodes = load_nodes(self.store, &request.kinds)?;
        let mut hits = Vec::new();
        for node in nodes {
            let body = node.search_body();
            let score = cosine(&query_vec, &hash_embed(&body));
            if score < 0.12 {
                continue;
            }
            hits.push(hit(
                node,
                score,
                SearchMode::Semantic,
                "hashed n-gram cosine against node search body (local embedder; not a remote model)",
            ));
        }
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(response(SearchMode::Semantic, hits, Vec::new()))
    }

    fn graph(&self, request: &SearchRequest, intent: &crate::QueryIntent) -> Result<SearchResponse> {
        let (from, to) = match (request.from, request.to) {
            (Some(from), Some(to)) => (self.store.get_node(from)?, self.store.get_node(to)?),
            _ => {
                let (from_name, to_name) = match (
                    intent.path_from.as_deref(),
                    intent.path_to.as_deref(),
                ) {
                    (Some(from), Some(to)) => (from.to_string(), to.to_string()),
                    _ => parse_path_query(&request.query).ok_or_else(|| {
                        SearchError::invalid(
                            "graph search requires from/to node ids or a 'path from X to Y' query",
                        )
                    })?,
                };
                (
                    resolve_named(self.store, &from_name, None)?,
                    resolve_named(self.store, &to_name, None)?,
                )
            }
        };
        let graph = Graph::new(self.store);
        let Some(path) = graph.trace_path(from.id, to.id, request.max_depth.max(1))? else {
            return Ok(response(
                SearchMode::Graph,
                Vec::new(),
                vec![format!(
                    "no path found from {} to {} within depth {}",
                    from.id, to.id, request.max_depth
                )],
            ));
        };
        let mut hits = Vec::new();
        for (index, node) in path.nodes.into_iter().enumerate() {
            hits.push(hit(
                node,
                1.0 - (index as f64 * 0.01),
                SearchMode::Graph,
                format!("node on path from {} to {}", from.id, to.id),
            ));
        }
        Ok(response(SearchMode::Graph, hits, Vec::new()))
    }

    fn temporal(&self, request: &SearchRequest) -> Result<SearchResponse> {
        if request.after.is_none() && request.before.is_none() {
            return Err(SearchError::invalid(
                "temporal search requires after and/or before timestamps",
            ));
        }
        let nodes = load_nodes(self.store, &request.kinds)?;
        let mut hits = Vec::new();
        for node in nodes {
            if !in_window(node.updated_at, request.after, request.before)
                && !in_window(node.created_at, request.after, request.before)
            {
                continue;
            }
            if !request.query.trim().is_empty()
                && !matches_text(&node, request.query.trim())
                && parse_structural_prefix(&request.query).is_none()
            {
                let name_ok = node
                    .name
                    .as_deref()
                    .is_some_and(|name| name.contains(request.query.trim()));
                if !name_ok {
                    continue;
                }
            }
            let recency = node.updated_at.as_millis() as f64;
            hits.push(hit(
                node,
                recency,
                SearchMode::Temporal,
                "timestamp window filter",
            ));
        }
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(response(SearchMode::Temporal, hits, Vec::new()))
    }

    fn hybrid(&self, request: &SearchRequest, intent: &crate::QueryIntent) -> Result<SearchResponse> {
        require_non_empty(&request.query)?;
        let mut combined = Vec::new();
        let mut notes = Vec::new();
        match self.fuzzy(request) {
            Ok(mut resp) => combined.append(&mut resp.hits),
            Err(err) => notes.push(format!("fuzzy: {err}")),
        }
        match self.full_text(request) {
            Ok(mut resp) => combined.append(&mut resp.hits),
            Err(err) => notes.push(format!("full_text: {err}")),
        }
        match self.semantic(request) {
            Ok(mut resp) => combined.append(&mut resp.hits),
            Err(err) => notes.push(format!("semantic: {err}")),
        }
        if parse_structural_prefix(&request.query).is_some() {
            match self.structural(request, intent) {
                Ok(mut resp) => combined.append(&mut resp.hits),
                Err(err) => notes.push(format!("structural: {err}")),
            }
        }
        if combined.is_empty() && notes.is_empty() {
            return Ok(response(SearchMode::Hybrid, Vec::new(), notes));
        }
        if combined.is_empty() {
            return Err(SearchError::msg(format!(
                "hybrid search produced no hits and sub-searches failed: {}",
                notes.join("; ")
            )));
        }
        Ok(response(SearchMode::Hybrid, merge_hits(combined), notes))
    }
}

struct FuzzyItem {
    text: String,
    node: Node,
}

impl AsRef<str> for FuzzyItem {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

fn hit(node: Node, score: f64, mode: SearchMode, reason: impl Into<String>) -> SearchHit {
    SearchHit {
        node,
        score,
        mode,
        modes: vec![mode],
        reason: reason.into(),
    }
}

fn response(mode: SearchMode, hits: Vec<SearchHit>, notes: Vec<String>) -> SearchResponse {
    SearchResponse { mode, hits, notes }
}

fn kind_allowed(node: &Node, kinds: &[NodeKind]) -> bool {
    kinds.is_empty() || kinds.iter().any(|kind| kind == &node.kind)
}

fn in_window(ts: Timestamp, after: Option<Timestamp>, before: Option<Timestamp>) -> bool {
    if let Some(after) = after {
        if ts < after {
            return false;
        }
    }
    if let Some(before) = before {
        if ts > before {
            return false;
        }
    }
    true
}

fn matches_text(node: &Node, query: &str) -> bool {
    node.search_body().to_ascii_lowercase().contains(&query.to_ascii_lowercase())
}

fn merge_hits(hits: Vec<SearchHit>) -> Vec<SearchHit> {
    let mut best: HashMap<NodeId, SearchHit> = HashMap::new();
    for hit in hits {
        match best.get_mut(&hit.node.id) {
            Some(existing) => {
                existing.score = existing.score.max(hit.score) + 0.12;
                if !existing.modes.contains(&hit.mode) {
                    existing.modes.push(hit.mode);
                }
                existing.reason = format!("{}; {}", existing.reason, hit.reason);
            }
            None => {
                best.insert(hit.node.id, hit);
            }
        }
    }
    let mut merged: Vec<SearchHit> = best.into_values().collect();
    merged.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    merged
}

pub fn resolve_named(store: &Store, name: &str, preferred_kind: Option<NodeKind>) -> Result<Node> {
    let name = name.trim();
    if name.is_empty() {
        return Err(SearchError::invalid("node name must not be empty"));
    }
    if let Ok(id) = NodeId::from_str(name) {
        return store.get_node(id).map_err(SearchError::from);
    }
    if let Some(kind) = preferred_kind.clone() {
        if let Some(node) = store.find_node_by_name(kind, name)? {
            return Ok(node);
        }
    }
    let mut matches = Vec::new();
    for kind in crate::catalog::searchable_kinds() {
        if let Some(node) = store.find_node_by_name(kind, name)? {
            matches.push(node);
        }
    }
    match matches.len() {
        0 => Err(SearchError::NotFound(name.to_string())),
        1 => Ok(matches.pop().expect("len == 1")),
        count => Err(SearchError::Ambiguous {
            name: name.to_string(),
            count,
        }),
    }
}
