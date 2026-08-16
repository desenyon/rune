use rune_core::{NodeKind, Timestamp};

use crate::error::{Result, SearchError};
use crate::mode::{QueryIntent, SearchMode};

/// Chooses a retrieval mode from query intent. Callers may still force a mode
/// on [`crate::SearchRequest::mode`].
pub struct SearchRouter;

impl SearchRouter {
    pub fn analyze(query: &str) -> QueryIntent {
        let trimmed = query.trim();
        if let Some((from, to)) = parse_path_query(trimmed) {
            return QueryIntent {
                mode: SearchMode::Graph,
                query: trimmed.to_string(),
                quoted: None,
                path_from: Some(from),
                path_to: Some(to),
                structural_kind: None,
                structural_name: None,
                after: None,
                before: None,
                reason: "query describes a path between two named nodes".into(),
            };
        }
        if let Some(quoted) = extract_quoted(trimmed) {
            let looks_like_id = quoted.starts_with("nod_") || quoted.starts_with("edg_");
            return QueryIntent {
                mode: if looks_like_id {
                    SearchMode::Exact
                } else {
                    SearchMode::FullText
                },
                query: quoted.clone(),
                quoted: Some(quoted),
                path_from: None,
                path_to: None,
                structural_kind: None,
                structural_name: None,
                after: None,
                before: None,
                reason: "quoted string selects exact identity or FTS phrase search".into(),
            };
        }
        if trimmed.starts_with("nod_") {
            return QueryIntent {
                mode: SearchMode::Exact,
                query: trimmed.to_string(),
                quoted: None,
                path_from: None,
                path_to: None,
                structural_kind: None,
                structural_name: None,
                after: None,
                before: None,
                reason: "query is a node identifier".into(),
            };
        }
        if let Some((kind, name)) = parse_structural_prefix(trimmed) {
            return QueryIntent {
                mode: SearchMode::Structural,
                query: name.clone(),
                quoted: None,
                path_from: None,
                path_to: None,
                structural_kind: Some(kind.as_str().to_string()),
                structural_name: Some(name),
                after: None,
                before: None,
                reason: "language-shaped prefix selects structural neighborhood search".into(),
            };
        }
        if let Some((after, before)) = parse_temporal(trimmed) {
            return QueryIntent {
                mode: SearchMode::Temporal,
                query: trimmed.to_string(),
                quoted: None,
                path_from: None,
                path_to: None,
                structural_kind: None,
                structural_name: None,
                after,
                before,
                reason: "query constrains results by time".into(),
            };
        }
        if looks_like_semantic(trimmed) {
            return QueryIntent {
                mode: SearchMode::Semantic,
                query: trimmed.to_string(),
                quoted: None,
                path_from: None,
                path_to: None,
                structural_kind: None,
                structural_name: None,
                after: None,
                before: None,
                reason: "query asks for meaning, similarity, or why-style retrieval".into(),
            };
        }
        if looks_like_fts(trimmed) {
            return QueryIntent {
                mode: SearchMode::FullText,
                query: trimmed.to_string(),
                quoted: None,
                path_from: None,
                path_to: None,
                structural_kind: None,
                structural_name: None,
                after: None,
                before: None,
                reason: "query uses full-text operators".into(),
            };
        }
        QueryIntent {
            mode: SearchMode::Hybrid,
            query: trimmed.to_string(),
            quoted: None,
            path_from: None,
            path_to: None,
            structural_kind: None,
            structural_name: None,
            after: None,
            before: None,
            reason: "default hybrid of fuzzy name matching and FTS".into(),
        }
    }

    pub fn apply_intent(request: &mut crate::SearchRequest) {
        let intent = Self::analyze(&request.query);
        if request.mode.is_none() {
            request.mode = Some(intent.mode);
        }
        if request.after.is_none() {
            request.after = intent.after.as_deref().and_then(parse_timestamp_token);
        }
        if request.before.is_none() {
            request.before = intent.before.as_deref().and_then(parse_timestamp_token);
        }
    }
}

pub fn parse_path_query(query: &str) -> Option<(String, String)> {
    let lower = query.to_ascii_lowercase();
    if let Some(start) = find_ci(&lower, "path from ") {
        let after = &query[start + "path from ".len()..];
        let after_lower = after.to_ascii_lowercase();
        if let Some(mid) = after_lower.find(" to ") {
            let from = after[..mid].trim();
            let to = after[mid + 4..].trim();
            if !from.is_empty() && !to.is_empty() {
                return Some((from.to_string(), to.to_string()));
            }
        }
    }
    if let Some(start) = find_ci(&lower, "path ") {
        let after = query[start + 5..].trim();
        if let Some((from, to)) = after.split_once("->") {
            let from = from.trim();
            let to = to.trim();
            if !from.is_empty() && !to.is_empty() {
                return Some((from.to_string(), to.to_string()));
            }
        }
    }
    if let Some(start) = find_ci(&lower, "from ") {
        if lower.contains(" to ") && lower.contains("path") {
            let after = &query[start + 5..];
            let after_lower = after.to_ascii_lowercase();
            if let Some(mid) = after_lower.find(" to ") {
                let from = after[..mid].trim();
                let to = after[mid + 4..].trim();
                if !from.is_empty() && !to.is_empty() {
                    return Some((from.to_string(), to.to_string()));
                }
            }
        }
    }
    None
}

pub fn extract_quoted(query: &str) -> Option<String> {
    let start = query.find('"')?;
    let rest = &query[start + 1..];
    let end = rest.find('"')?;
    let inner = rest[..end].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

pub fn parse_structural_prefix(query: &str) -> Option<(NodeKind, String)> {
    let lower = query.to_ascii_lowercase();
    let prefixes = [
        ("fn ", NodeKind::Function),
        ("function ", NodeKind::Function),
        ("method ", NodeKind::Method),
        ("class ", NodeKind::Class),
        ("trait ", NodeKind::Trait),
        ("interface ", NodeKind::Interface),
        ("type ", NodeKind::Type),
        ("symbol ", NodeKind::Symbol),
        ("mod ", NodeKind::Module),
        ("module ", NodeKind::Module),
        ("test ", NodeKind::Test),
    ];
    for (prefix, kind) in prefixes {
        if lower.starts_with(prefix) {
            let name = query[prefix.len()..].trim();
            if !name.is_empty() {
                return Some((kind, name.to_string()));
            }
        }
    }
    None
}

fn parse_temporal(query: &str) -> Option<(Option<String>, Option<String>)> {
    let lower = query.to_ascii_lowercase();
    let mut after = None;
    let mut before = None;
    if let Some(token) = take_after_keyword(&lower, query, "after ") {
        after = Some(token);
    } else if let Some(token) = take_after_keyword(&lower, query, "since ") {
        after = Some(token);
    }
    if let Some(token) = take_after_keyword(&lower, query, "before ") {
        before = Some(token);
    } else if let Some(token) = take_after_keyword(&lower, query, "until ") {
        before = Some(token);
    }
    if after.is_some() || before.is_some() {
        Some((after, before))
    } else {
        None
    }
}

fn take_after_keyword(lower: &str, original: &str, keyword: &str) -> Option<String> {
    let start = lower.find(keyword)?;
    let rest = original[start + keyword.len()..].trim();
    let token = rest.split_whitespace().next()?.to_string();
    Some(token)
}

fn looks_like_semantic(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    lower.starts_with("why ")
        || lower.starts_with("similar to ")
        || lower.starts_with("meaning of ")
        || lower.contains("semantically")
        || lower.starts_with("what does ")
}

fn looks_like_fts(query: &str) -> bool {
    let upper = query.to_ascii_uppercase();
    upper.contains(" AND ") || upper.contains(" OR ") || upper.contains(" NEAR ")
}

fn find_ci(lower_haystack: &str, needle: &str) -> Option<usize> {
    lower_haystack.find(needle)
}

pub fn parse_timestamp_token(token: &str) -> Option<Timestamp> {
    let token = token.trim().trim_matches(|c| c == '"' || c == '\'');
    if let Ok(millis) = token.parse::<i64>() {
        return Some(Timestamp::from_millis(millis));
    }
    let parts: Vec<&str> = token.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    ymd_to_timestamp(year, month, day)
}

fn ymd_to_timestamp(year: i32, month: u32, day: u32) -> Option<Timestamp> {
    if !(1..=12).contains(&month) || day == 0 {
        return None;
    }
    let mdays = [31u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let max_day = if month == 2 && is_leap(year) {
        29
    } else {
        mdays[(month - 1) as usize]
    };
    if day > max_day {
        return None;
    }
    let mut days: i64 = 0;
    if year >= 1970 {
        for y in 1970..year {
            days += if is_leap(y) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            days -= if is_leap(y) { 366 } else { 365 };
        }
    }
    for m in 1..month {
        days += mdays[(m - 1) as usize] as i64;
        if m == 2 && is_leap(year) {
            days += 1;
        }
    }
    days += i64::from(day) - 1;
    Some(Timestamp::from_millis(days * 86_400_000))
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn require_non_empty(query: &str) -> Result<()> {
    if query.trim().is_empty() {
        Err(SearchError::invalid("search query must not be empty"))
    } else {
        Ok(())
    }
}
