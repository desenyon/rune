//! TUI design system (S039), command palette (S038), themes (S071),
//! keybindings (S072), accessibility (S073), and view snapshots (S043–S049).
//!
//! Widgets render snapshots. Domain crates are not depended on in reverse.

pub mod keys;
pub mod palette;
pub mod theme;
pub mod views;

pub use keys::{KeyChord, KeyConflict, KeyMap};
pub use palette::{PaletteItem, PalettePhase, PaletteState};
pub use theme::{
    status_label, Accessibility, SemanticTokens, Spacing, StatusKind, Theme, Typography,
};
pub use views::{
    ActiveView, AgentCockpitSnapshot, ContextInspectorSnapshot, DashboardSnapshot,
    GraphExplorerState, MemoryTimelineSnapshot, SessionExplorerSnapshot, SpecCoverageSnapshot,
    TaskGraphSnapshot,
};

use ratatui::layout::Rect;
use ratatui::Frame;
use rune_motion::{BufferDiffHint, FrameBudget, MotionConfig};
use rune_terminal::{RendererLevel, TerminalCapabilities};

#[derive(Clone, Debug)]
pub struct UiContext {
    pub theme: Theme,
    pub motion: MotionConfig,
    pub accessibility: Accessibility,
    pub capabilities: TerminalCapabilities,
    pub keymap: KeyMap,
}

impl UiContext {
    pub fn new(caps: TerminalCapabilities) -> Self {
        let accessibility = Accessibility::from_caps(&caps);
        let theme = if accessibility.high_contrast {
            Theme::high_contrast()
        } else {
            Theme::rune_dark().fallback(caps.true_color)
        };
        Self {
            motion: MotionConfig {
                reduced_motion: accessibility.reduced_motion,
            },
            keymap: KeyMap::default_bindings(),
            theme,
            accessibility,
            capabilities: caps,
        }
    }

    pub fn frame_budget(&self, animating: bool) -> FrameBudget {
        FrameBudget::for_activity(
            animating,
            self.capabilities.renderer_level == RendererLevel::Graphics,
        )
    }
}

/// Top-level application snapshot rendered by the TUI.
#[derive(Clone, Debug, Default)]
pub struct AppSnapshot {
    pub title: String,
    pub status: String,
    pub view: crate::views::ActiveView,
    pub palette: PaletteState,
    pub dashboard: crate::views::DashboardSnapshot,
    pub graph: crate::views::GraphExplorerState,
    pub memory: crate::views::MemoryTimelineSnapshot,
    pub sessions: crate::views::SessionExplorerSnapshot,
    pub tasks: crate::views::TaskGraphSnapshot,
    pub specs: crate::views::SpecCoverageSnapshot,
    pub agents: crate::views::AgentCockpitSnapshot,
    pub inspector: Option<ContextInspectorSnapshot>,
    pub renderer_level: String,
}

pub fn render_app(frame: &mut Frame, area: Rect, ui: &UiContext, snapshot: &AppSnapshot) {
    views::render_shell(frame, area, ui, snapshot);
}

pub fn render_app_text(ui: &UiContext, snapshot: &AppSnapshot) -> String {
    views::render_shell_text(ui, snapshot)
}

pub fn buffer_hint(prev: u64, current: u64, synchronized: bool) -> BufferDiffHint {
    BufferDiffHint::after_render(prev, current, synchronized)
}
