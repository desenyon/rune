use crate::error::Result;
use crate::io::{first_string, json_text, parse_jsonl, read_lossy, walk_files};
use crate::model::{DiscoveredSession, NormalizedSession, NormalizedTurn};
use crate::provider::SessionSource;
use serde_json::Value;

pub mod aider;
pub mod claude;
pub mod codex;
pub mod cursor;
pub mod gemini;
pub mod opencode;

pub fn all_sources() -> Vec<Box<dyn SessionSource>> {
    vec![
        Box::new(claude::ClaudeCodeAdapter),
        Box::new(codex::CodexAdapter),
        Box::new(cursor::CursorAdapter),
        Box::new(opencode::OpenCodeAdapter),
        Box::new(gemini::GeminiAdapter),
        Box::new(aider::AiderAdapter),
    ]
}

pub(crate) fn discover_files(
    provider: &str,
    roots: &[std::path::PathBuf],
    extensions: &[&str],
) -> Result<Vec<DiscoveredSession>> {
    let mut found = Vec::new();
    for root in roots {
        for path in walk_files(root, extensions)? {
            let external_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("session")
                .to_string();
            found.push(DiscoveredSession {
                provider: provider.to_string(),
                external_id,
                title: Some(path.display().to_string()),
                path,
            });
        }
    }
    Ok(found)
}

pub(crate) fn jsonl_session(
    provider: &str,
    discovered: &DiscoveredSession,
    turn_from_value: fn(&Value, usize) -> Option<NormalizedTurn>,
) -> Result<NormalizedSession> {
    let raw = read_lossy(&discovered.path)?;
    let values = parse_jsonl(&raw);
    let mut session = NormalizedSession {
        provider: provider.to_string(),
        external_id: discovered.external_id.clone(),
        source_path: discovered.path.clone(),
        title: discovered.title.clone(),
        cwd: None,
        raw,
        turns: Vec::new(),
    };
    for (index, value) in values.iter().enumerate() {
        if session.cwd.is_none() {
            session.cwd = first_string(value, &["cwd", "workingDirectory", "working_directory"]);
            if session.cwd.is_none() {
                if let Some(payload) = value.get("payload") {
                    session.cwd = first_string(payload, &["cwd", "working_directory"]);
                }
            }
        }
        if let Some(id) = first_string(value, &["sessionId", "session_id", "id"]) {
            session.external_id = id;
        }
        if let Some(payload) = value.get("payload") {
            if let Some(id) = first_string(payload, &["id", "session_id"]) {
                session.external_id = id;
            }
        }
        if let Some(turn) = turn_from_value(value, index) {
            session.turns.push(turn);
        }
    }
    if session.title.is_none() {
        session.title = session
            .turns
            .iter()
            .find(|turn| turn.role == "user" && !turn.text.trim().is_empty())
            .map(|turn| truncate(&turn.text, 80));
    }
    Ok(session)
}

pub(crate) fn turn(
    external_id: impl Into<String>,
    role: impl Into<String>,
    text: impl Into<String>,
    raw: Value,
    timestamp: Option<String>,
) -> NormalizedTurn {
    NormalizedTurn {
        external_id: external_id.into(),
        role: role.into(),
        text: text.into(),
        raw,
        timestamp,
    }
}

pub(crate) fn message_turn(value: &Value, index: usize) -> Option<NormalizedTurn> {
    let message = value.get("message").unwrap_or(value);
    let role = first_string(value, &["role", "type"])
        .or_else(|| first_string(message, &["role", "type"]))?;
    let role = normalize_role(&role);
    if matches!(
        role.as_str(),
        "queue-operation" | "attachment" | "last-prompt" | "session_meta" | "event_msg"
    ) && value.get("message").is_none()
        && json_text(value).is_empty()
    {
        return None;
    }
    let text = if let Some(msg) = value.get("message") {
        json_text(msg)
    } else {
        json_text(value)
    };
    if text.trim().is_empty() && !matches!(role.as_str(), "user" | "assistant" | "tool") {
        return None;
    }
    let id =
        first_string(value, &["uuid", "id", "turn_id"]).unwrap_or_else(|| format!("turn-{index}"));
    let timestamp = first_string(value, &["timestamp", "createdAt", "created_at"]);
    Some(turn(id, role, text, value.clone(), timestamp))
}

pub(crate) fn normalize_role(role: &str) -> String {
    match role {
        "human" | "Human" => "user".into(),
        "ai" | "model" | "bot" => "assistant".into(),
        other => other.to_lowercase(),
    }
}

pub(crate) fn truncate(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        let clipped: String = flat.chars().take(max.saturating_sub(1)).collect();
        format!("{clipped}…")
    }
}
