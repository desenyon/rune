use crate::error::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub fn read_lossy(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn parse_jsonl(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(value) => out.push(value),
            Err(err) => {
                tracing::debug!(index, %err, "skipping malformed jsonl line");
            }
        }
    }
    out
}

pub fn json_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(json_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                return text.to_string();
            }
            if let Some(content) = map.get("content") {
                let nested = json_text(content);
                if !nested.is_empty() {
                    return nested;
                }
            }
            if let Some(body) = map.get("body").and_then(Value::as_str) {
                return body.to_string();
            }
            String::new()
        }
        _ => String::new(),
    }
}

pub fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value.get(*key) {
            if let Some(text) = found.as_str() {
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
    }
    None
}

pub fn walk_files(root: &Path, extensions: &[&str]) -> Result<Vec<std::path::PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(12);
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                tracing::debug!(%err, path = %root.display(), "skipping unreadable path");
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some("node_modules" | ".git" | "target" | ".venv")
            )
        }) {
            continue;
        }
        let allowed = match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => extensions.iter().any(|want| *want == ext),
            None => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name.starts_with(".aider") || name.ends_with("history")
            }
        };
        if allowed {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}
