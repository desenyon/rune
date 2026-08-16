use crate::model::NormalizedTurn;
use regex::Regex;
use rune_core::{NodeKind, Validity};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Heuristic extraction from transcript text. Never a verified fact.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractedItem {
    pub kind: NodeKind,
    pub extracted_as: String,
    pub statement: String,
    pub source_turn_index: usize,
    pub source_turn_external_id: String,
    pub validity: Validity,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionIntelligence {
    pub goal: Option<String>,
    pub items: Vec<ExtractedItem>,
    pub files_touched: Vec<String>,
}

pub fn extract(turns: &[NormalizedTurn]) -> SessionIntelligence {
    let mut intel = SessionIntelligence::default();
    intel.goal = turns
        .iter()
        .find(|turn| turn.role == "user" && !turn.text.trim().is_empty())
        .map(|turn| first_goal(&turn.text));
    for (index, turn) in turns.iter().enumerate() {
        extract_turn(turn, index, &mut intel);
    }
    intel.files_touched.sort();
    intel.files_touched.dedup();
    intel
}

fn extract_turn(turn: &NormalizedTurn, index: usize, intel: &mut SessionIntelligence) {
    let text = strip_tags(&turn.text);
    for path in file_re().find_iter(&text) {
        intel.files_touched.push(path.as_str().to_string());
    }
    push_matches(
        intel,
        turn,
        index,
        attempt_re(),
        NodeKind::Attempt,
        "attempt",
    );
    push_matches(
        intel,
        turn,
        index,
        failure_re(),
        NodeKind::Failure,
        "failure",
    );
    push_matches(
        intel,
        turn,
        index,
        decision_re(),
        NodeKind::Decision,
        "decision",
    );
    push_matches(
        intel,
        turn,
        index,
        discovery_re(),
        NodeKind::Discovery,
        "discovery",
    );
    for cap in command_re().captures_iter(&text) {
        if let Some(cmd) = cap.get(1) {
            intel.items.push(item(
                turn,
                index,
                NodeKind::Command,
                "command",
                cmd.as_str(),
            ));
        }
    }
    for line in text.lines() {
        let line = line.trim();
        if line.ends_with('?') && line.len() > 8 && turn.role != "user" {
            intel.items.push(item(
                turn,
                index,
                NodeKind::Unknown("open_question".into()),
                "open_question",
                line,
            ));
        }
    }
}

fn push_matches(
    intel: &mut SessionIntelligence,
    turn: &NormalizedTurn,
    index: usize,
    regex: &Regex,
    kind: NodeKind,
    extracted_as: &str,
) {
    let text = strip_tags(&turn.text);
    for cap in regex.find_iter(&text) {
        let statement = surrounding_sentence(&text, cap.start(), cap.end());
        intel
            .items
            .push(item(turn, index, kind.clone(), extracted_as, &statement));
    }
}

fn item(
    turn: &NormalizedTurn,
    index: usize,
    kind: NodeKind,
    extracted_as: &str,
    statement: &str,
) -> ExtractedItem {
    ExtractedItem {
        kind,
        extracted_as: extracted_as.to_string(),
        statement: statement.trim().to_string(),
        source_turn_index: index,
        source_turn_external_id: turn.external_id.clone(),
        validity: Validity::Candidate,
    }
}

fn first_goal(text: &str) -> String {
    let stripped = strip_tags(text);
    if let Some(caps) = goal_re().captures(&stripped) {
        if let Some(goal) = caps.get(1) {
            return goal.as_str().trim().to_string();
        }
    }
    stripped
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(stripped.trim())
        .to_string()
}

fn strip_tags(text: &str) -> String {
    let without_ts = timestamp_re().replace_all(text, "");
    let without_query = user_query_re().replace_all(&without_ts, "$1");
    without_query.to_string()
}

fn surrounding_sentence(text: &str, start: usize, end: usize) -> String {
    let prefix = text[..start].rfind(['.', '\n']).map(|i| i + 1).unwrap_or(0);
    let suffix = text[end..]
        .find(['.', '\n'])
        .map(|i| end + i + 1)
        .unwrap_or(text.len());
    text[prefix..suffix].trim().to_string()
}

fn attempt_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(i(?:'ll| will)? try|let me try|trying to|attempt(?:ing)? to)\b")
            .expect("static regex")
    })
}

fn failure_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(failed|didn't work|did not work|doesn't work|does not work|error:)\b")
            .expect("static regex")
    })
}

fn decision_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(i(?:'ve| have)? decided|decision:|going with|we should use|will use)\b")
            .expect("static regex")
    })
}

fn discovery_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(found that|discovered|it turns out|root cause)\b")
            .expect("static regex")
    })
}

fn command_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^(?:\$|❯|➜)\s+(.+)$").expect("static regex"))
}

fn file_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b([\w./\-]+?\.(?:rs|ts|tsx|js|jsx|py|go|toml|md|json|yml|yaml))\b")
            .expect("static regex")
    })
}

fn goal_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?is)<user_query>\s*(.*?)\s*</user_query>|(?i)\b(?:goal|task|objective)\s*:\s*(.+)",
        )
        .expect("static regex")
    })
}

fn timestamp_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)<timestamp>.*?</timestamp>").expect("static regex"))
}

fn user_query_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?is)<user_query>\s*(.*?)\s*</user_query>").expect("static regex")
    })
}
