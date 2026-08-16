use ratatui::style::{Color, Modifier, Style};
use rune_terminal::TerminalCapabilities;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        let v = value.trim().trim_start_matches('#');
        if v.len() != 6 {
            return None;
        }
        let n = u32::from_str_radix(v, 16).ok()?;
        Some(Self {
            r: ((n >> 16) & 0xff) as u8,
            g: ((n >> 8) & 0xff) as u8,
            b: (n & 0xff) as u8,
        })
    }

    pub fn to_color(self, true_color: bool) -> Color {
        if true_color {
            Color::Rgb(self.r, self.g, self.b)
        } else {
            Color::Indexed(ansi256(self))
        }
    }
}

fn ansi256(rgb: Rgb) -> u8 {
    let q = |c: u8| (u16::from(c) * 5 / 255) as u8;
    16 + 36 * q(rgb.r) + 6 * q(rgb.g) + q(rgb.b)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticTokens {
    pub surface: Rgb,
    pub elevated: Rgb,
    pub border: Rgb,
    pub primary_text: Rgb,
    pub secondary_text: Rgb,
    pub muted_text: Rgb,
    pub accent: Rgb,
    pub success: Rgb,
    pub warning: Rgb,
    pub error: Rgb,
    pub selection: Rgb,
    pub focus: Rgb,
}

impl SemanticTokens {
    /// Carved ember: warm ink, bone type, molten gold accent.
    pub fn dark() -> Self {
        Self {
            surface: Rgb::from_hex("0b0a09").unwrap(),
            elevated: Rgb::from_hex("161310").unwrap(),
            border: Rgb::from_hex("2a241c").unwrap(),
            primary_text: Rgb::from_hex("f4ede3").unwrap(),
            secondary_text: Rgb::from_hex("c4b8a8").unwrap(),
            muted_text: Rgb::from_hex("7a7066").unwrap(),
            accent: Rgb::from_hex("e4b060").unwrap(),
            success: Rgb::from_hex("7c9a6a").unwrap(),
            warning: Rgb::from_hex("d4a017").unwrap(),
            error: Rgb::from_hex("c45c4a").unwrap(),
            selection: Rgb::from_hex("3d2e16").unwrap(),
            focus: Rgb::from_hex("f0c674").unwrap(),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            surface: Rgb::new(0, 0, 0),
            elevated: Rgb::new(20, 20, 20),
            border: Rgb::new(255, 255, 255),
            primary_text: Rgb::new(255, 255, 255),
            secondary_text: Rgb::new(255, 255, 0),
            muted_text: Rgb::new(200, 200, 200),
            accent: Rgb::new(0, 255, 255),
            success: Rgb::new(0, 255, 0),
            warning: Rgb::new(255, 255, 0),
            error: Rgb::new(255, 80, 80),
            selection: Rgb::new(255, 255, 255),
            focus: Rgb::new(255, 0, 255),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Spacing {
    pub xs: u16,
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xs: 0,
            sm: 1,
            md: 2,
            lg: 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typography {
    Title,
    Section,
    Body,
    Muted,
    Code,
    Status,
    KeyHint,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub tokens: SemanticTokens,
    pub spacing: Spacing,
    pub true_color: bool,
}

impl Theme {
    pub fn rune_dark() -> Self {
        Self {
            name: "rune-ember".into(),
            tokens: SemanticTokens::dark(),
            spacing: Spacing::default(),
            true_color: true,
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            name: "high-contrast".into(),
            tokens: SemanticTokens::high_contrast(),
            spacing: Spacing::default(),
            true_color: true,
        }
    }

    pub fn fallback(mut self, true_color: bool) -> Self {
        self.true_color = true_color;
        self
    }

    pub fn from_toml(text: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(text).map_err(|err| err.to_string())?;
        Self::from_table(&value)
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|err| err.to_string())?;
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("custom")
            .to_string();
        let tokens = value
            .get("tokens")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        Ok(Self {
            name,
            tokens: tokens_from_map(json_map(&tokens)?),
            spacing: Spacing::default(),
            true_color: true,
        })
    }

    fn from_table(value: &toml::Value) -> Result<Self, String> {
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("custom")
            .to_string();
        let tokens = value
            .get("tokens")
            .cloned()
            .unwrap_or(toml::Value::Table(Default::default()));
        let map = toml_map(&tokens)?;
        Ok(Self {
            name,
            tokens: tokens_from_map(map),
            spacing: Spacing::default(),
            true_color: true,
        })
    }

    pub fn style(&self, role: Typography) -> Style {
        let t = &self.tokens;
        let tc = self.true_color;
        match role {
            Typography::Title => Style::default()
                .fg(t.accent.to_color(tc))
                .add_modifier(Modifier::BOLD),
            Typography::Section => Style::default()
                .fg(t.focus.to_color(tc))
                .add_modifier(Modifier::BOLD),
            Typography::Body => Style::default().fg(t.primary_text.to_color(tc)),
            Typography::Muted => Style::default().fg(t.muted_text.to_color(tc)),
            Typography::Code => Style::default().fg(t.secondary_text.to_color(tc)),
            Typography::Status => Style::default().fg(t.accent.to_color(tc)),
            Typography::KeyHint => Style::default().fg(t.muted_text.to_color(tc)),
        }
    }

    pub fn bg(&self) -> Color {
        self.tokens.surface.to_color(self.true_color)
    }

    pub fn elevated(&self) -> Color {
        self.tokens.elevated.to_color(self.true_color)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.tokens.primary_text.to_color(self.true_color))
            .bg(self.tokens.selection.to_color(self.true_color))
    }

    pub fn kind_style(&self, kind: StatusKind) -> Style {
        let color = match kind {
            StatusKind::Success => self.tokens.success,
            StatusKind::Warning => self.tokens.warning,
            StatusKind::Error => self.tokens.error,
            StatusKind::Stale => self.tokens.warning,
            StatusKind::Neutral => self.tokens.secondary_text,
        };
        Style::default().fg(color.to_color(self.true_color))
    }
}

fn json_map(value: &serde_json::Value) -> Result<BTreeMap<String, String>, String> {
    let obj = value.as_object().ok_or("tokens must be an object")?;
    Ok(obj
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect())
}

fn toml_map(value: &toml::Value) -> Result<BTreeMap<String, String>, String> {
    let table = value.as_table().ok_or("tokens must be a table")?;
    Ok(table
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect())
}

fn tokens_from_map(map: BTreeMap<String, String>) -> SemanticTokens {
    let mut tokens = SemanticTokens::dark();
    let set = |key: &str, target: &mut Rgb| {
        if let Some(hex) = map.get(key) {
            if let Some(rgb) = Rgb::from_hex(hex) {
                *target = rgb;
            }
        }
    };
    set("surface", &mut tokens.surface);
    set("elevated", &mut tokens.elevated);
    set("border", &mut tokens.border);
    set("primary_text", &mut tokens.primary_text);
    set("secondary_text", &mut tokens.secondary_text);
    set("muted_text", &mut tokens.muted_text);
    set("accent", &mut tokens.accent);
    set("success", &mut tokens.success);
    set("warning", &mut tokens.warning);
    set("error", &mut tokens.error);
    set("selection", &mut tokens.selection);
    set("focus", &mut tokens.focus);
    tokens
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Success,
    Warning,
    Error,
    Stale,
    Neutral,
}

/// Color-independent status labels (S073).
pub fn status_label(kind: StatusKind) -> &'static str {
    match kind {
        StatusKind::Success => "OK",
        StatusKind::Warning => "WARN",
        StatusKind::Error => "ERR",
        StatusKind::Stale => "STALE",
        StatusKind::Neutral => "INFO",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Accessibility {
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub keyboard_only: bool,
}

impl Accessibility {
    pub fn from_caps(caps: &TerminalCapabilities) -> Self {
        let reduced = std::env::var("RUNE_REDUCED_MOTION")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let high = std::env::var("RUNE_HIGH_CONTRAST")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        Self {
            reduced_motion: reduced,
            high_contrast: high,
            keyboard_only: !caps.mouse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ember_theme_parses_hex_tokens() {
        let theme = Theme::rune_dark();
        assert_eq!(theme.name, "rune-ember");
        assert_eq!(theme.tokens.accent, Rgb::from_hex("e4b060").unwrap());
    }
}
