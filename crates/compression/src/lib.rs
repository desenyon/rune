//! Adaptive compression for tool output and context objects (S028).
//!
//! Representations are chosen from evidence, never applied blindly. When a
//! non-raw representation is produced, the original bytes are stored in the
//! blob store and `raw_hash` is recorded so decompression is reversible.

use rune_core::{ContentHash, Timestamp};
use rune_storage::BlobStore;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Bytes-per-token heuristic used across the compiler and compression.
pub const CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error("artifact is not reversible: {0}")]
    Irreversible(String),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, CompressionError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    Raw,
    Structured,
    Summary,
    Errors,
    Diff,
    ChangesSincePrevious,
}

impl Representation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Structured => "structured",
            Self::Summary => "summary",
            Self::Errors => "errors",
            Self::Diff => "diff",
            Self::ChangesSincePrevious => "changes_since_previous",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressionInput {
    pub label: String,
    pub bytes: Vec<u8>,
    pub media_type: Option<String>,
    pub previous: Option<Vec<u8>>,
    pub exit_code: Option<i32>,
    pub is_tool_output: bool,
    /// When true, never leave Raw even for small payloads (tests / explicit).
    pub force_representation: Option<Representation>,
}

impl CompressionInput {
    pub fn from_text(label: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            bytes: text.into().into_bytes(),
            media_type: Some("text/plain".into()),
            previous: None,
            exit_code: None,
            is_tool_output: false,
            force_representation: None,
        }
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressionDecision {
    pub representation: Representation,
    pub reason: String,
    pub compressed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedArtifact {
    pub representation: Representation,
    pub body: String,
    pub raw_hash: Option<ContentHash>,
    pub reversible: bool,
    pub estimated_tokens: usize,
    pub original_bytes: usize,
    pub decision: CompressionDecision,
    pub created_at: Timestamp,
}

pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    let from_chars = chars.saturating_add(CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN;
    // Prefer the character heuristic; word count is a floor for dense tokens.
    from_chars.max(words)
}

/// Choose a representation. Small or already-structured payloads stay raw.
pub fn decide(input: &CompressionInput) -> CompressionDecision {
    if let Some(forced) = input.force_representation {
        return CompressionDecision {
            representation: forced,
            reason: format!("explicit representation {}", forced.as_str()),
            compressed: forced != Representation::Raw,
        };
    }

    let text = input.text();
    let len = input.bytes.len();

    if input.exit_code.unwrap_or(0) != 0 || looks_like_errors(&text) {
        return CompressionDecision {
            representation: Representation::Errors,
            reason: "nonzero exit or error-dense output; keep failing lines".into(),
            compressed: true,
        };
    }

    if let Some(prev) = input.previous.as_ref() {
        if prev != &input.bytes && len > 256 {
            return CompressionDecision {
                representation: Representation::ChangesSincePrevious,
                reason: "previous snapshot available; send only the delta".into(),
                compressed: true,
            };
        }
    }

    if looks_like_unified_diff(&text) {
        return CompressionDecision {
            representation: Representation::Diff,
            reason: "payload is already a unified diff".into(),
            compressed: false,
        };
    }

    if looks_like_json(&text) && len > 2048 {
        return CompressionDecision {
            representation: Representation::Structured,
            reason: "large JSON; keep typed fields, drop nulls and empty arrays".into(),
            compressed: true,
        };
    }

    // Do not blindly compress. Short payloads and non-tool text stay raw.
    if len < 1024 || !input.is_tool_output {
        return CompressionDecision {
            representation: Representation::Raw,
            reason: "payload is small or not tool output; raw preserves reasoning fidelity".into(),
            compressed: false,
        };
    }

    if len > 8192 {
        return CompressionDecision {
            representation: Representation::Summary,
            reason: "large tool output; summary plus stored raw_hash".into(),
            compressed: true,
        };
    }

    CompressionDecision {
        representation: Representation::Raw,
        reason: "no compression signal fired".into(),
        compressed: false,
    }
}

pub fn compress(input: &CompressionInput, blobs: Option<&BlobStore>) -> Result<CompressedArtifact> {
    let decision = decide(input);
    let raw_hash = if decision.compressed {
        match blobs {
            Some(store) => Some(store.put(&input.bytes, input.media_type.as_deref())?.hash),
            None => None,
        }
    } else {
        None
    };

    let body = match decision.representation {
        Representation::Raw | Representation::Diff => input.text(),
        Representation::Structured => structured_body(&input.text()),
        Representation::Summary => summarize(&input.text(), &input.label),
        Representation::Errors => extract_errors(&input.text()),
        Representation::ChangesSincePrevious => match input.previous.as_ref() {
            Some(prev) => unified_diff(&input.label, prev, &input.bytes),
            None => input.text(),
        },
    };

    let reversible = raw_hash.is_some() || !decision.compressed;
    Ok(CompressedArtifact {
        estimated_tokens: estimate_tokens(&body),
        original_bytes: input.bytes.len(),
        representation: decision.representation,
        body,
        raw_hash,
        reversible,
        decision,
        created_at: Timestamp::now(),
    })
}

pub fn restore(artifact: &CompressedArtifact, blobs: &BlobStore) -> Result<Vec<u8>> {
    if artifact.representation == Representation::Raw
        || artifact.representation == Representation::Diff
    {
        return Ok(artifact.body.as_bytes().to_vec());
    }
    let Some(hash) = artifact.raw_hash else {
        return Err(CompressionError::Irreversible(
            "raw bytes were not stored; cannot reverse compression".into(),
        ));
    };
    Ok(blobs.get(hash)?)
}

fn looks_like_errors(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let markers = [
        "error:",
        "error[",
        "panic:",
        "failed",
        "exception",
        "traceback",
    ];
    let hits = markers.iter().filter(|m| lower.contains(**m)).count();
    hits >= 2 || (hits >= 1 && text.lines().count() > 8)
}

fn looks_like_unified_diff(text: &str) -> bool {
    text.contains("\n@@ ") || text.starts_with("diff --git ") || text.starts_with("--- ")
}

fn looks_like_json(text: &str) -> bool {
    let trimmed = text.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

fn structured_body(text: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => {
            let pruned = prune_json(value);
            serde_json::to_string_pretty(&pruned).unwrap_or_else(|_| text.to_string())
        }
        Err(_) => summarize(text, "structured"),
    }
}

fn prune_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::Value::Null,
        serde_json::Value::Array(items) => {
            let kept: Vec<_> = items
                .into_iter()
                .map(prune_json)
                .filter(|item| !item.is_null() && item != &serde_json::json!([]))
                .collect();
            serde_json::Value::Array(kept)
        }
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                let pruned = prune_json(val);
                if pruned.is_null() {
                    continue;
                }
                if pruned.as_array().is_some_and(|a| a.is_empty()) {
                    continue;
                }
                out.insert(key, pruned);
            }
            serde_json::Value::Object(out)
        }
        other => other,
    }
}

fn summarize(text: &str, label: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let head: Vec<&str> = lines.iter().take(12).copied().collect();
    let tail: Vec<&str> = if total > 20 {
        lines
            .iter()
            .rev()
            .take(8)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    } else {
        Vec::new()
    };
    let mut out = format!("summary of {label}: {total} lines, {} bytes\n", text.len());
    out.push_str("--- head ---\n");
    out.push_str(&head.join("\n"));
    if !tail.is_empty() {
        out.push_str("\n--- tail ---\n");
        out.push_str(&tail.join("\n"));
    }
    out
}

fn extract_errors(text: &str) -> String {
    let mut kept = Vec::new();
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error")
            || lower.contains("fail")
            || lower.contains("panic")
            || lower.contains("exception")
            || lower.contains("traceback")
            || line.starts_with("E ")
        {
            kept.push(line);
        }
    }
    if kept.is_empty() {
        return summarize(text, "errors");
    }
    format!("errors ({} lines):\n{}", kept.len(), kept.join("\n"))
}

fn unified_diff(label: &str, previous: &[u8], current: &[u8]) -> String {
    let prev_lines: Vec<String> = String::from_utf8_lossy(previous)
        .lines()
        .map(str::to_string)
        .collect();
    let curr_lines: Vec<String> = String::from_utf8_lossy(current)
        .lines()
        .map(str::to_string)
        .collect();
    let mut out = format!("--- a/{label}\n+++ b/{label}\n");
    let max = prev_lines.len().max(curr_lines.len());
    let mut hunk = String::new();
    let mut hunk_has_change = false;
    for i in 0..max {
        match (prev_lines.get(i), curr_lines.get(i)) {
            (Some(a), Some(b)) if a == b => hunk.push_str(&format!(" {a}\n")),
            (Some(a), Some(b)) => {
                hunk.push_str(&format!("-{a}\n+{b}\n"));
                hunk_has_change = true;
            }
            (Some(a), None) => {
                hunk.push_str(&format!("-{a}\n"));
                hunk_has_change = true;
            }
            (None, Some(b)) => {
                hunk.push_str(&format!("+{b}\n"));
                hunk_has_change = true;
            }
            (None, None) => {}
        }
    }
    if hunk_has_change {
        out.push_str(&format!(
            "@@ -1,{} +1,{} @@\n",
            prev_lines.len(),
            curr_lines.len()
        ));
        out.push_str(&hunk);
    } else {
        out.push_str("unchanged\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_payloads_stay_raw() {
        let input = CompressionInput::from_text("cmd", "ok");
        let decision = decide(&input);
        assert_eq!(decision.representation, Representation::Raw);
        assert!(!decision.compressed);
    }

    #[test]
    fn error_output_uses_errors_representation() {
        let mut input = CompressionInput::from_text(
            "test",
            "running tests\nerror: panicked at src/lib.rs:10\nthread panicked\nFAIL suite\n",
        );
        input.exit_code = Some(1);
        input.is_tool_output = true;
        let decision = decide(&input);
        assert_eq!(decision.representation, Representation::Errors);
    }

    #[test]
    fn roundtrip_via_blob_store() {
        let blobs = BlobStore::open_temp().unwrap();
        let mut input =
            CompressionInput::from_text("build", "error: failed\nerror: missing crate\nmore log\n");
        input.is_tool_output = true;
        input.exit_code = Some(1);
        let artifact = compress(&input, Some(&blobs)).unwrap();
        assert!(artifact.raw_hash.is_some());
        assert!(artifact.reversible);
        let restored = restore(&artifact, &blobs).unwrap();
        assert_eq!(restored, input.bytes);
    }

    #[test]
    fn token_heuristic_is_chars_over_four() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }
}
