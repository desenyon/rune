//! Multimodal document indexing that preserves source structure (S005).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedSection {
    pub heading: String,
    pub content: String,
    pub start_line: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub kind: String,
    pub title: String,
    pub sections: Vec<ParsedSection>,
}

pub fn parse_document(relative: &str, source: &str, language: &str) -> ParsedDocument {
    match language {
        "markdown" => parse_markdown(relative, source),
        "json" => parse_keyed(relative, source, "json"),
        "toml" => parse_toml_tables(relative, source),
        "yaml" => parse_keyed(relative, source, "yaml"),
        _ => ParsedDocument {
            kind: language.to_string(),
            title: relative.to_string(),
            sections: vec![ParsedSection {
                heading: relative.to_string(),
                content: source.chars().take(8_192).collect(),
                start_line: 1,
            }],
        },
    }
}

fn parse_markdown(relative: &str, source: &str) -> ParsedDocument {
    let mut sections = Vec::new();
    let mut current_heading = relative.to_string();
    let mut current_body = String::new();
    let mut start_line = 1u32;
    let mut line_no = 0u32;
    for line in source.lines() {
        line_no += 1;
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let heading = rest.trim_start_matches('#').trim();
            if !heading.is_empty() {
                flush_section(&mut sections, &current_heading, &current_body, start_line);
                current_heading = heading.to_string();
                current_body.clear();
                start_line = line_no;
                continue;
            }
        }
        current_body.push_str(line);
        current_body.push('\n');
    }
    flush_section(&mut sections, &current_heading, &current_body, start_line);
    if sections.is_empty() {
        sections.push(ParsedSection {
            heading: relative.to_string(),
            content: source.chars().take(8_192).collect(),
            start_line: 1,
        });
    }
    ParsedDocument {
        kind: "markdown".into(),
        title: relative.to_string(),
        sections,
    }
}

fn parse_toml_tables(relative: &str, source: &str) -> ParsedDocument {
    let mut sections = Vec::new();
    let mut current = "root".to_string();
    let mut body = String::new();
    let mut start_line = 1u32;
    let mut line_no = 0u32;
    for line in source.lines() {
        line_no += 1;
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            flush_section(&mut sections, &current, &body, start_line);
            current = trimmed.trim_matches(['[', ']']).to_string();
            body.clear();
            start_line = line_no;
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    flush_section(&mut sections, &current, &body, start_line);
    ParsedDocument {
        kind: "toml".into(),
        title: relative.to_string(),
        sections,
    }
}

fn parse_keyed(relative: &str, source: &str, kind: &str) -> ParsedDocument {
    let mut sections = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(source) {
        if let Some(obj) = value.as_object() {
            for (key, val) in obj {
                sections.push(ParsedSection {
                    heading: key.clone(),
                    content: val.to_string(),
                    start_line: 1,
                });
            }
        }
    }
    if sections.is_empty() {
        for (idx, line) in source.lines().enumerate() {
            if let Some((key, rest)) = line.split_once(':') {
                let key = key.trim();
                if !key.is_empty() && !key.starts_with('#') && !key.starts_with('-') {
                    sections.push(ParsedSection {
                        heading: key.to_string(),
                        content: rest.trim().to_string(),
                        start_line: (idx as u32) + 1,
                    });
                }
            }
        }
    }
    if sections.is_empty() {
        sections.push(ParsedSection {
            heading: relative.to_string(),
            content: source.chars().take(4_096).collect(),
            start_line: 1,
        });
    }
    ParsedDocument {
        kind: kind.into(),
        title: relative.to_string(),
        sections,
    }
}

fn flush_section(sections: &mut Vec<ParsedSection>, heading: &str, body: &str, start_line: u32) {
    let content = body.trim();
    if content.is_empty() && heading.is_empty() {
        return;
    }
    if content.is_empty() {
        return;
    }
    sections.push(ParsedSection {
        heading: heading.to_string(),
        content: content.chars().take(8_192).collect(),
        start_line,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_preserves_headings() {
        let parsed = parse_document(
            "ADR.md",
            "# Decision\n\nUse SQLite.\n\n## Consequences\n\nLocal first.\n",
            "markdown",
        );
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].heading, "Decision");
        assert!(parsed.sections[1].content.contains("Local first"));
    }
}
