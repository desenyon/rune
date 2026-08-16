use rune_core::{EdgeKind, Node, NodeId, NodeKind, Timestamp};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    Exact,
    Fuzzy,
    FullText,
    Structural,
    Semantic,
    Graph,
    Temporal,
    Hybrid,
}

impl SearchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Fuzzy => "fuzzy",
            Self::FullText => "full_text",
            Self::Structural => "structural",
            Self::Semantic => "semantic",
            Self::Graph => "graph",
            Self::Temporal => "temporal",
            Self::Hybrid => "hybrid",
        }
    }
}

impl FromStr for SearchMode {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "exact" => Ok(Self::Exact),
            "fuzzy" => Ok(Self::Fuzzy),
            "full_text" | "fts" | "text" => Ok(Self::FullText),
            "structural" | "symbol" => Ok(Self::Structural),
            "semantic" | "meaning" => Ok(Self::Semantic),
            "graph" => Ok(Self::Graph),
            "temporal" | "git" => Ok(Self::Temporal),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(format!("unknown search mode `{other}`")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    /// When set, the engine must use this mode and must not substitute another.
    pub mode: Option<SearchMode>,
    pub limit: usize,
    pub kinds: Vec<NodeKind>,
    pub edge_kinds: Vec<EdgeKind>,
    pub around: Option<NodeId>,
    pub from: Option<NodeId>,
    pub to: Option<NodeId>,
    pub after: Option<Timestamp>,
    pub before: Option<Timestamp>,
    pub max_depth: usize,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            mode: None,
            limit: 50,
            kinds: Vec::new(),
            edge_kinds: Vec::new(),
            around: None,
            from: None,
            to: None,
            after: None,
            before: None,
            max_depth: 8,
        }
    }
}

impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }

    pub fn with_mode(mut self, mode: SearchMode) -> Self {
        self.mode = Some(mode);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub node: Node,
    pub score: f64,
    pub mode: SearchMode,
    pub modes: Vec<SearchMode>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub mode: SearchMode,
    pub hits: Vec<SearchHit>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryIntent {
    pub mode: SearchMode,
    pub query: String,
    pub quoted: Option<String>,
    pub path_from: Option<String>,
    pub path_to: Option<String>,
    pub structural_kind: Option<String>,
    pub structural_name: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub reason: String,
}
