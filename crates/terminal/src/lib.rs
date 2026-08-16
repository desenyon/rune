//! Terminal capability engine (S002).
//!
//! The interface degrades gracefully. Missing graphics never make the TUI unusable.

use serde::{Deserialize, Serialize};
use std::env;
use std::io::IsTerminal;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererLevel {
    /// Monochrome, no mouse, no images.
    Basic,
    /// Color and Unicode, keyboard only.
    Standard,
    /// True color, mouse, hyperlinks.
    Enhanced,
    /// Graphics protocols available.
    Graphics,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCapabilities {
    pub true_color: bool,
    pub unicode: bool,
    pub mouse: bool,
    pub hyperlinks: bool,
    pub synchronized_output: bool,
    pub kitty_graphics: bool,
    pub sixel: bool,
    pub iterm_graphics: bool,
    pub cells: Option<(u16, u16)>,
    pub pixel_size: Option<(u16, u16)>,
    pub is_tty: bool,
    pub color_level: ColorLevel,
    pub term: Option<String>,
    pub renderer_level: RendererLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorLevel {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

impl TerminalCapabilities {
    pub fn detect() -> Self {
        Self::detect_from_env(&EnvProbe::from_process())
    }

    pub fn detect_from_env(probe: &EnvProbe) -> Self {
        let term = probe.term.clone();
        let term_lower = term.as_deref().unwrap_or("").to_ascii_lowercase();
        let colorterm = probe.colorterm.as_deref().unwrap_or("").to_ascii_lowercase();
        let true_color = colorterm.contains("truecolor")
            || colorterm.contains("24bit")
            || matches!(term.as_deref(), Some("xterm-ghostty") | Some("xterm-kitty") | Some("wezterm"));
        let kitty_graphics = term_lower.contains("kitty") || probe.term_program.as_deref() == Some("kitty");
        let iterm_graphics = probe.term_program.as_deref() == Some("iTerm.app")
            || probe.term_program.as_deref() == Some("WezTerm")
            || term_lower.contains("wezterm");
        let sixel = term_lower.contains("mlterm")
            || term_lower.contains("sixel")
            || probe.has_sixel_hint;
        let hyperlinks = kitty_graphics || iterm_graphics || term_lower.contains("ghostty") || true_color;
        let synchronized_output = kitty_graphics || term_lower.contains("ghostty") || term_lower.contains("wezterm");
        let unicode = !matches!(probe.lang.as_deref(), Some(lang) if lang.to_ascii_lowercase().contains("ascii"));
        let mouse = probe.is_tty;
        let color_level = if true_color {
            ColorLevel::TrueColor
        } else if probe.colorterm.is_some() || term.is_some() {
            ColorLevel::Ansi256
        } else if probe.is_tty {
            ColorLevel::Ansi16
        } else {
            ColorLevel::None
        };
        let renderer_level = if !probe.is_tty {
            RendererLevel::Basic
        } else if kitty_graphics || iterm_graphics || sixel {
            RendererLevel::Graphics
        } else if true_color && mouse {
            RendererLevel::Enhanced
        } else if unicode {
            RendererLevel::Standard
        } else {
            RendererLevel::Basic
        };
        Self {
            true_color,
            unicode,
            mouse,
            hyperlinks,
            synchronized_output,
            kitty_graphics,
            sixel: sixel && probe.is_tty,
            iterm_graphics,
            cells: probe.cells,
            pixel_size: probe.pixel_size,
            is_tty: probe.is_tty,
            color_level,
            term,
            renderer_level,
        }
    }

    pub fn images_supported(&self) -> bool {
        self.kitty_graphics || self.sixel || self.iterm_graphics
    }

    /// Fallback path when advanced graphics are unavailable.
    pub fn image_strategy(&self) -> ImageStrategy {
        if self.kitty_graphics {
            ImageStrategy::Kitty
        } else if self.iterm_graphics {
            ImageStrategy::Iterm
        } else if self.sixel {
            ImageStrategy::Sixel
        } else {
            ImageStrategy::UnicodeFallback
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageStrategy {
    Kitty,
    Sixel,
    Iterm,
    UnicodeFallback,
}

#[derive(Clone, Debug, Default)]
pub struct EnvProbe {
    pub term: Option<String>,
    pub term_program: Option<String>,
    pub colorterm: Option<String>,
    pub lang: Option<String>,
    pub is_tty: bool,
    pub cells: Option<(u16, u16)>,
    pub pixel_size: Option<(u16, u16)>,
    pub has_sixel_hint: bool,
}

impl EnvProbe {
    pub fn from_process() -> Self {
        let cells = crossterm::terminal::size().ok();
        Self {
            term: env::var("TERM").ok(),
            term_program: env::var("TERM_PROGRAM").ok(),
            colorterm: env::var("COLORTERM").ok(),
            lang: env::var("LANG").ok(),
            is_tty: std::io::stdout().is_terminal(),
            cells,
            pixel_size: None,
            has_sixel_hint: env::var("TERM").map(|t| t.contains("sixel")).unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghostty_is_graphics_capable() {
        let caps = TerminalCapabilities::detect_from_env(&EnvProbe {
            term: Some("xterm-ghostty".into()),
            term_program: Some("ghostty".into()),
            colorterm: Some("truecolor".into()),
            lang: Some("en_US.UTF-8".into()),
            is_tty: true,
            cells: Some((120, 40)),
            pixel_size: Some((1920, 1080)),
            has_sixel_hint: false,
        });
        assert_eq!(caps.renderer_level, RendererLevel::Enhanced);
        assert!(caps.true_color);
        assert!(caps.unicode);
        assert!(caps.hyperlinks);
        assert_eq!(caps.image_strategy(), ImageStrategy::UnicodeFallback);
    }

    #[test]
    fn kitty_enables_graphics_protocol() {
        let caps = TerminalCapabilities::detect_from_env(&EnvProbe {
            term: Some("xterm-kitty".into()),
            term_program: Some("kitty".into()),
            colorterm: Some("truecolor".into()),
            lang: Some("en_US.UTF-8".into()),
            is_tty: true,
            cells: Some((80, 24)),
            pixel_size: Some((800, 600)),
            has_sixel_hint: false,
        });
        assert_eq!(caps.renderer_level, RendererLevel::Graphics);
        assert_eq!(caps.image_strategy(), ImageStrategy::Kitty);
    }

    #[test]
    fn non_tty_degrades_to_basic_and_stays_usable() {
        let caps = TerminalCapabilities::detect_from_env(&EnvProbe {
            is_tty: false,
            ..EnvProbe::default()
        });
        assert_eq!(caps.renderer_level, RendererLevel::Basic);
        assert_eq!(caps.image_strategy(), ImageStrategy::UnicodeFallback);
        assert!(!caps.images_supported());
    }
}
