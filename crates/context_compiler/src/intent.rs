use crate::budget::TaskType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    Exact,
    Fuzzy,
    FullText,
    Structural,
    Semantic,
    Graph,
    Temporal,
    Hybrid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Intent {
    pub goal: String,
    pub task_type: TaskType,
    pub keywords: Vec<String>,
    pub forced_mode: Option<RetrievalMode>,
}

pub fn analyze_intent(goal: &str, forced: Option<RetrievalMode>) -> Intent {
    let lower = goal.to_ascii_lowercase();
    let task_type = if contains_any(
        &lower,
        &["debug", "fail", "error", "race", "bug", "panic", "flaky"],
    ) {
        TaskType::Debugging
    } else if contains_any(
        &lower,
        &["architecture", "design", "adr", "refactor structure"],
    ) {
        TaskType::Architecture
    } else if contains_any(&lower, &["review", "pull request", "diff intelligence"]) {
        TaskType::Review
    } else if contains_any(&lower, &["document", "readme", "docs"]) {
        TaskType::Documentation
    } else if contains_any(&lower, &["implement", "add", "fix", "change"]) {
        TaskType::Implementation
    } else {
        TaskType::General
    };

    let keywords = goal
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_string())
        .collect();

    Intent {
        goal: goal.to_string(),
        task_type,
        keywords,
        forced_mode: forced,
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}
