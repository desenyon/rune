use crate::palette::{PalettePhase, PaletteState};
use crate::theme::{status_label, StatusKind, Typography};
use crate::{AppSnapshot, UiContext};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveView {
    #[default]
    Home,
    Graph,
    Memory,
    Sessions,
    Tasks,
    Specs,
    Context,
    Agents,
}

impl ActiveView {
    pub fn all() -> [ActiveView; 8] {
        [
            Self::Home,
            Self::Graph,
            Self::Memory,
            Self::Sessions,
            Self::Tasks,
            Self::Specs,
            Self::Context,
            Self::Agents,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Graph => "Graph",
            Self::Memory => "Memory",
            Self::Sessions => "Sessions",
            Self::Tasks => "Tasks",
            Self::Specs => "Specs",
            Self::Context => "Context",
            Self::Agents => "Agents",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            Self::Home => "1",
            Self::Graph => "2",
            Self::Memory => "3",
            Self::Sessions => "4",
            Self::Tasks => "5",
            Self::Specs => "6",
            Self::Context => "7",
            Self::Agents => "8",
        }
    }

    pub fn from_digit(c: char) -> Option<Self> {
        match c {
            '1' => Some(Self::Home),
            '2' => Some(Self::Graph),
            '3' => Some(Self::Memory),
            '4' => Some(Self::Sessions),
            '5' => Some(Self::Tasks),
            '6' => Some(Self::Specs),
            '7' => Some(Self::Context),
            '8' => Some(Self::Agents),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|v| *v == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|v| *v == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub files: u64,
    pub symbols: u64,
    pub memories: u64,
    pub sessions: u64,
    pub tasks: u64,
    pub specs: u64,
    pub commits: u64,
    pub languages: Vec<String>,
    pub agents: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNodeView {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdgeView {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphExplorerState {
    pub pan: (f32, f32),
    pub zoom: f32,
    pub focus: Option<String>,
    pub expanded: BTreeSet<String>,
    pub node_filter: Vec<String>,
    pub edge_filter: Vec<String>,
    pub path: Vec<String>,
    pub nodes: Vec<GraphNodeView>,
    pub edges: Vec<GraphEdgeView>,
}

impl GraphExplorerState {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            ..Self::default()
        }
    }

    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        self.pan.0 += dx;
        self.pan.1 += dy;
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.25, 8.0);
    }

    pub fn focus_node(&mut self, id: impl Into<String>) {
        self.focus = Some(id.into());
    }

    pub fn expand_neighbors(&mut self, id: impl Into<String>) {
        self.expanded.insert(id.into());
    }

    pub fn collapse(&mut self, id: &str) {
        self.expanded.remove(id);
    }

    pub fn filter_node_type(&mut self, kind: impl Into<String>) {
        let kind = kind.into();
        if !self.node_filter.iter().any(|k| k == &kind) {
            self.node_filter.push(kind);
        }
    }

    pub fn filter_edge_type(&mut self, kind: impl Into<String>) {
        let kind = kind.into();
        if !self.edge_filter.iter().any(|k| k == &kind) {
            self.edge_filter.push(kind);
        }
    }

    pub fn set_path(&mut self, path: Vec<String>) {
        self.path = path;
    }

    pub fn visible_nodes(&self) -> Vec<&GraphNodeView> {
        self.nodes
            .iter()
            .filter(|n| {
                self.node_filter.is_empty() || self.node_filter.iter().any(|k| k == &n.kind)
            })
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorItem {
    pub name: String,
    pub reason: String,
    pub tokens: usize,
    pub stale: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContextInspectorSnapshot {
    pub token_estimate: usize,
    pub budget: usize,
    pub allocations: Vec<(String, usize)>,
    pub included: Vec<InspectorItem>,
    pub excluded: Vec<InspectorItem>,
    pub stale: Vec<String>,
    pub contradictions: Vec<String>,
    pub duplicates_removed: usize,
    pub compression_notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub id: String,
    pub statement: String,
    pub state: String,
    pub at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryTimelineSnapshot {
    pub events: Vec<MemoryEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub provider: String,
    pub project: String,
    pub outcome: String,
    pub when: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionExplorerSnapshot {
    pub sessions: Vec<SessionRow>,
    pub provider_filter: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentCard {
    pub provider: String,
    pub model: Option<String>,
    pub task: String,
    pub worktree: Option<String>,
    pub branch: Option<String>,
    pub current_action: String,
    pub context_usage: String,
    pub tests: String,
    pub status: String,
    pub elapsed: String,
    pub recent_events: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AgentCockpitSnapshot {
    pub agents: Vec<AgentCard>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Ready,
    Active,
    Blocked,
    Failed,
    Review,
    Complete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskNodeView {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub blocked_by: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaskGraphSnapshot {
    pub tasks: Vec<TaskNodeView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequirementCoverage {
    pub id: String,
    pub title: String,
    pub tasks: Vec<String>,
    pub symbols: Vec<String>,
    pub tests: Vec<String>,
    pub commits: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpecCoverageSnapshot {
    pub requirements: Vec<RequirementCoverage>,
}

pub fn render_shell(frame: &mut Frame, area: Rect, ui: &UiContext, snapshot: &AppSnapshot) {
    frame.render_widget(
        Block::default().style(Style::default().bg(ui.theme.bg()).fg(ui.theme.tokens.primary_text.to_color(ui.theme.true_color))),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], ui, snapshot);
    render_tabs(frame, chunks[1], ui, snapshot.view);
    render_tab_rule(frame, chunks[2], ui, snapshot.view);
    render_body(frame, chunks[3], ui, snapshot);
    render_footer(frame, chunks[4], ui, snapshot);

    if snapshot.palette.is_open() {
        render_palette_overlay(frame, area, ui, &snapshot.palette);
    }
}

fn render_header(frame: &mut Frame, area: Rect, ui: &UiContext, snapshot: &AppSnapshot) {
    let mark = Span::styled("ᚱ  RUNE", ui.theme.style(Typography::Title));
    let sep = Span::styled("   ", ui.theme.style(Typography::Muted));
    let title = Span::styled(&snapshot.title, ui.theme.style(Typography::Muted));
    let right = format!("  {}", snapshot.renderer_level.to_ascii_lowercase());
    let header = Paragraph::new(Line::from(vec![mark, sep, title]))
        .style(Style::default().bg(ui.theme.bg()));
    frame.render_widget(header, area);
    let meta = Paragraph::new(right)
        .alignment(Alignment::Right)
        .style(ui.theme.style(Typography::Muted));
    frame.render_widget(meta, area);
}

fn tab_label(view: ActiveView) -> String {
    format!(" {} {} ", view.shortcut(), view.label())
}

fn render_tabs(frame: &mut Frame, area: Rect, ui: &UiContext, active: ActiveView) {
    let mut spans = Vec::new();
    for view in ActiveView::all() {
        let selected = view == active;
        let style = if selected {
            ui.theme.style(Typography::Title)
        } else {
            ui.theme.style(Typography::Muted)
        };
        spans.push(Span::styled(tab_label(view), style));
        spans.push(Span::raw(" "));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_tab_rule(frame: &mut Frame, area: Rect, ui: &UiContext, active: ActiveView) {
    let accent = Style::default().fg(ui.theme.tokens.accent.to_color(ui.theme.true_color));
    let mute = Style::default().fg(ui.theme.tokens.border.to_color(ui.theme.true_color));
    let mut spans = Vec::new();
    for view in ActiveView::all() {
        let label = tab_label(view);
        let width = label.chars().count();
        let selected = view == active;
        let rule: String = if selected {
            "━".repeat(width)
        } else {
            "─".repeat(width)
        };
        spans.push(Span::styled(rule, if selected { accent } else { mute }));
        spans.push(Span::styled(" ", mute));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_footer(frame: &mut Frame, area: Rect, ui: &UiContext, snapshot: &AppSnapshot) {
    let hints = if snapshot.palette.is_open() {
        "esc close  ·  ↑↓ move  ·  enter open  ·  type to search"
    } else {
        "1–8 views  ·  tab next  ·  ctrl+p command  ·  r reload  ·  q quit"
    };
    let left = Span::styled(&snapshot.status, ui.theme.style(Typography::Muted));
    let line = Line::from(vec![
        left,
        Span::styled("    ", ui.theme.style(Typography::Muted)),
        Span::styled(hints, ui.theme.style(Typography::KeyHint)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_body(frame: &mut Frame, area: Rect, ui: &UiContext, snapshot: &AppSnapshot) {
    let inner = inset(area, ui.theme.spacing.sm);
    match snapshot.view {
        ActiveView::Home => render_home(frame, inner, ui, snapshot),
        ActiveView::Graph => render_graph(frame, inner, ui, &snapshot.graph),
        ActiveView::Memory => render_memory_timeline(frame, inner, ui, &snapshot.memory),
        ActiveView::Sessions => render_sessions(frame, inner, ui, &snapshot.sessions),
        ActiveView::Tasks => render_task_graph(frame, inner, ui, &snapshot.tasks),
        ActiveView::Specs => render_spec_coverage(frame, inner, ui, &snapshot.specs),
        ActiveView::Context => match &snapshot.inspector {
            Some(inspector) => render_inspector(frame, inner, ui, inspector),
            None => render_empty(
                frame,
                inner,
                ui,
                "No capsule compiled",
                "Run rune context compile \"your goal\" then reopen this view.",
            ),
        },
        ActiveView::Agents => render_cockpit(frame, inner, ui, &snapshot.agents),
    }
}

fn inset(area: Rect, pad: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(pad),
        y: area.y.saturating_add(pad / 2),
        width: area.width.saturating_sub(pad.saturating_mul(2)),
        height: area.height.saturating_sub(pad),
    }
}

fn render_home(frame: &mut Frame, area: Rect, ui: &UiContext, snapshot: &AppSnapshot) {
    let dash = &snapshot.dashboard;
    let cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Workspace intelligence", ui.theme.style(Typography::Section)),
            Span::styled(
                "  ·  everything is an object",
                ui.theme.style(Typography::Muted),
            ),
        ])),
        cols[0],
    );

    let row = |area: Rect| {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ])
            .split(area)
    };
    let top = row(cols[1]);
    render_metric(frame, top[0], ui, "files", dash.files);
    render_metric(frame, top[1], ui, "symbols", dash.symbols);
    render_metric(frame, top[2], ui, "memories", dash.memories);
    render_metric(frame, top[3], ui, "sessions", dash.sessions);
    let bottom = row(cols[2]);
    render_metric(frame, bottom[0], ui, "tasks", dash.tasks);
    render_metric(frame, bottom[1], ui, "specs", dash.specs);
    render_metric(frame, bottom[2], ui, "commits", dash.commits);
    render_metric(frame, bottom[3], ui, "agents", dash.agents.len() as u64);

    let langs = if dash.languages.is_empty() {
        "index this workspace to detect languages".into()
    } else {
        dash.languages.join("  ·  ")
    };
    let agents = if dash.agents.is_empty() {
        "no local coding agents detected".into()
    } else {
        dash.agents.join("  ·  ")
    };
    let body = vec![
        Line::from(Span::styled(langs, ui.theme.style(Typography::Body))),
        Line::from(Span::styled(agents, ui.theme.style(Typography::Muted))),
        Line::from(""),
        Line::from(Span::styled(
            "ctrl+p searches every indexed object.  rune index fills the graph.",
            ui.theme.style(Typography::Muted),
        )),
    ];
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: true }), cols[3]);
}

fn render_metric(frame: &mut Frame, area: Rect, ui: &UiContext, label: &str, value: u64) {
    let lines = vec![
        Line::from(Span::styled(
            value.to_string(),
            ui.theme.style(Typography::Title),
        )),
        Line::from(Span::styled(label, ui.theme.style(Typography::Muted))),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_empty(frame: &mut Frame, area: Rect, ui: &UiContext, title: &str, hint: &str) {
    let lines = vec![
        Line::from(Span::styled("ᚱ", ui.theme.style(Typography::Title))),
        Line::from(""),
        Line::from(Span::styled(title, ui.theme.style(Typography::Section))),
        Line::from(""),
        Line::from(Span::styled(hint, ui.theme.style(Typography::Muted))),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_palette_overlay(frame: &mut Frame, area: Rect, ui: &UiContext, palette: &PaletteState) {
    let width = area.width.saturating_mul(7) / 10;
    let height = area.height.saturating_mul(6) / 10;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 3;
    let overlay = Rect {
        x,
        y,
        width: width.max(24),
        height: height.max(10),
    };
    frame.render_widget(Clear, overlay);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(ui.theme.tokens.accent.to_color(ui.theme.true_color)))
        .style(Style::default().bg(ui.theme.elevated()))
        .padding(Padding::horizontal(1))
        .title(Span::styled(" command ", ui.theme.style(Typography::Title)));
    let inner = block.inner(overlay);
    frame.render_widget(block, overlay);
    render_palette(frame, inner, ui, palette);
}

pub fn render_palette(frame: &mut Frame, area: Rect, ui: &UiContext, palette: &PaletteState) {
    let (query, items, selected) = match &palette.phase {
        PalettePhase::Idle => ("", Vec::new(), 0usize),
        PalettePhase::Querying { query } => (query.as_str(), Vec::new(), 0),
        PalettePhase::Results {
            query,
            items,
            selected,
        }
        | PalettePhase::Preview {
            query,
            items,
            selected,
        } => (query.as_str(), items.clone(), *selected),
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    let caret = if query.is_empty() {
        Line::from(vec![
            Span::styled("› ", ui.theme.style(Typography::Title)),
            Span::styled("search objects, commands, memories…", ui.theme.style(Typography::Muted)),
        ])
    } else {
        Line::from(vec![
            Span::styled("› ", ui.theme.style(Typography::Title)),
            Span::styled(query, ui.theme.style(Typography::Body)),
            Span::styled("▌", ui.theme.style(Typography::Status)),
        ])
    };
    frame.render_widget(Paragraph::new(caret), chunks[0]);
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("no matches").style(ui.theme.style(Typography::Muted)),
            chunks[1],
        );
        return;
    }
    let rows: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let prefix = if idx == selected { "▸ " } else { "  " };
            let style = if idx == selected {
                ui.theme.selected()
            } else {
                ui.theme.style(Typography::Body)
            };
            let line = Line::from(vec![
                Span::styled(format!("{prefix}{}", item.title), style),
                Span::styled(
                    format!("  {}", item.subtitle),
                    if idx == selected {
                        style
                    } else {
                        ui.theme.style(Typography::Muted)
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();
    frame.render_widget(List::new(rows), chunks[1]);
}

pub fn render_inspector(
    frame: &mut Frame,
    area: Rect,
    ui: &UiContext,
    inspector: &ContextInspectorSnapshot,
) {
    let used = if inspector.budget == 0 {
        0
    } else {
        (inspector.token_estimate * 100) / inspector.budget
    };
    let mut lines = vec![
        Line::from(Span::styled("Context capsule", ui.theme.style(Typography::Section))),
        Line::from(Span::styled(
            format!(
                "tokens {} / {}  ·  {}%  ·  dupes removed {}",
                inspector.token_estimate, inspector.budget, used, inspector.duplicates_removed
            ),
            ui.theme.style(Typography::Muted),
        )),
        Line::from(""),
    ];
    for (cat, n) in &inspector.allocations {
        lines.push(Line::from(Span::styled(
            format!("  {cat:<16} {n}"),
            ui.theme.style(Typography::Code),
        )));
    }
    if inspector.included.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No objects selected yet.",
            ui.theme.style(Typography::Muted),
        )));
    }
    for item in inspector.included.iter().take(12) {
        let kind = if item.stale {
            StatusKind::Stale
        } else {
            StatusKind::Success
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", status_label(kind)), ui.theme.kind_style(kind)),
            Span::styled(&item.name, ui.theme.style(Typography::Body)),
            Span::styled(
                format!("  {} tok  {}", item.tokens, item.reason),
                ui.theme.style(Typography::Muted),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

pub fn render_graph(frame: &mut Frame, area: Rect, ui: &UiContext, graph: &GraphExplorerState) {
    if graph.nodes.is_empty() {
        render_empty(
            frame,
            area,
            ui,
            "Graph is empty",
            "Run rune index so files, symbols, and commits become nodes.",
        );
        return;
    }
    let focus_id = graph.focus.clone();
    let focus = graph
        .nodes
        .iter()
        .find(|n| Some(n.id.as_str()) == focus_id.as_deref());
    let mut lines = vec![
        Line::from(Span::styled("Canonical graph", ui.theme.style(Typography::Section))),
        Line::from(Span::styled(
            format!(
                "{} nodes  ·  {} edges  ·  zoom {:.1}",
                graph.visible_nodes().len(),
                graph.edges.len(),
                graph.zoom
            ),
            ui.theme.style(Typography::Muted),
        )),
        Line::from(""),
    ];
    if !graph.path.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("path  {}", graph.path.join("  →  ")),
            ui.theme.style(Typography::Code),
        )));
        lines.push(Line::from(""));
    }
    if let Some(focus) = focus {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}  ", kind_glyph(&focus.kind)),
                ui.theme.style(Typography::Title),
            ),
            Span::styled(&focus.label, ui.theme.style(Typography::Title)),
            Span::styled(
                format!("  {}", focus.kind),
                ui.theme.style(Typography::Muted),
            ),
        ]));
        let children: Vec<&GraphEdgeView> = graph
            .edges
            .iter()
            .filter(|e| e.from == focus.id)
            .collect();
        for (idx, edge) in children.iter().enumerate() {
            let last = idx + 1 == children.len();
            let branch = if last { "└─" } else { "├─" };
            let target = graph
                .nodes
                .iter()
                .find(|n| n.id == edge.to)
                .map(|n| n.label.as_str())
                .unwrap_or(edge.to.as_str());
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {branch} {}  ", kind_glyph(&edge.kind)),
                    ui.theme.style(Typography::Muted),
                ),
                Span::styled(target, ui.theme.style(Typography::Body)),
                Span::styled(
                    format!("  {}", edge.kind),
                    ui.theme.style(Typography::Muted),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }
    for node in graph.visible_nodes().into_iter().take(16) {
        if Some(node.id.as_str()) == focus_id.as_deref() {
            continue;
        }
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}  ", kind_glyph(&node.kind)),
                ui.theme.style(Typography::Muted),
            ),
            Span::styled(format!("{:<12}", node.kind), ui.theme.style(Typography::Muted)),
            Span::styled(&node.label, ui.theme.style(Typography::Body)),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn kind_glyph(kind: &str) -> &'static str {
    match kind {
        "function" | "method" => "λ",
        "file" => "◇",
        "commit" => "·",
        "memory" => "▣",
        "session" => "◎",
        "task" => "▸",
        "specification" | "requirement" => "§",
        "agent" => "◈",
        _ => "○",
    }
}

pub fn render_memory_timeline(
    frame: &mut Frame,
    area: Rect,
    ui: &UiContext,
    snap: &MemoryTimelineSnapshot,
) {
    if snap.events.is_empty() {
        render_empty(
            frame,
            area,
            ui,
            "No memories yet",
            "Import sessions or record a human decision. Stale memories stay visible here, never as silent guidance.",
        );
        return;
    }
    let mut lines = vec![
        Line::from(Span::styled("Memory timeline", ui.theme.style(Typography::Section))),
        Line::from(""),
    ];
    for event in snap.events.iter().take(24) {
        let kind = match event.state.as_str() {
            "stale" => StatusKind::Stale,
            "contradicted" => StatusKind::Error,
            "verified" | "stable" => StatusKind::Success,
            _ => StatusKind::Neutral,
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<7}", status_label(kind)),
                ui.theme.kind_style(kind),
            ),
            Span::styled(
                format!("  {:<12}  ", event.state),
                ui.theme.style(Typography::Muted),
            ),
            Span::styled(&event.statement, ui.theme.style(Typography::Body)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("         {}", event.at),
            ui.theme.style(Typography::Muted),
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

pub fn render_sessions(
    frame: &mut Frame,
    area: Rect,
    ui: &UiContext,
    snap: &SessionExplorerSnapshot,
) {
    let rows: Vec<&SessionRow> = snap
        .sessions
        .iter()
        .filter(|s| {
            snap.provider_filter
                .as_ref()
                .map(|p| &s.provider == p)
                .unwrap_or(true)
        })
        .collect();
    if rows.is_empty() {
        render_empty(
            frame,
            area,
            ui,
            "No sessions ingested",
            "rune sessions import discovers Claude Code, Codex, Cursor, OpenCode, Gemini, and Aider histories.",
        );
        return;
    }
    let mut lines = vec![
        Line::from(Span::styled("Sessions", ui.theme.style(Typography::Section))),
        Line::from(""),
    ];
    for session in rows.into_iter().take(20) {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<12}", session.provider),
                ui.theme.style(Typography::Title),
            ),
            Span::styled(
                format!("  {}  {}", session.project, session.outcome),
                ui.theme.style(Typography::Body),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!("             {}", session.when),
            ui.theme.style(Typography::Muted),
        )));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn render_cockpit(frame: &mut Frame, area: Rect, ui: &UiContext, snap: &AgentCockpitSnapshot) {
    if snap.agents.is_empty() {
        render_empty(
            frame,
            area,
            ui,
            "No active agents",
            "Detected local agent directories appear on Home. Runtime cards show here when an execution is live.",
        );
        return;
    }
    let mut lines = vec![
        Line::from(Span::styled("Agent cockpit", ui.theme.style(Typography::Section))),
        Line::from(""),
    ];
    for agent in &snap.agents {
        let kind = match agent.status.as_str() {
            "failed" => StatusKind::Error,
            "running" | "active" => StatusKind::Success,
            _ => StatusKind::Neutral,
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}  ", status_label(kind)),
                ui.theme.kind_style(kind),
            ),
            Span::styled(&agent.provider, ui.theme.style(Typography::Title)),
            Span::styled(
                format!("  {}  {}", agent.task, agent.elapsed),
                ui.theme.style(Typography::Body),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!("       {}  {}", agent.current_action, agent.context_usage),
            ui.theme.style(Typography::Muted),
        )));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn render_task_graph(frame: &mut Frame, area: Rect, ui: &UiContext, snap: &TaskGraphSnapshot) {
    if snap.tasks.is_empty() {
        render_empty(
            frame,
            area,
            ui,
            "No tasks",
            "Tasks are first-class graph objects with dependencies, blockers, and parallelization analysis.",
        );
        return;
    }
    let mut lines = vec![
        Line::from(Span::styled("Task graph", ui.theme.style(Typography::Section))),
        Line::from(""),
    ];
    for task in &snap.tasks {
        let kind = match task.status {
            TaskStatus::Complete => StatusKind::Success,
            TaskStatus::Failed => StatusKind::Error,
            TaskStatus::Blocked => StatusKind::Warning,
            TaskStatus::Ready | TaskStatus::Active | TaskStatus::Review => StatusKind::Neutral,
        };
        let blocked = if task.blocked_by.is_empty() {
            String::new()
        } else {
            format!("  blocked by {}", task.blocked_by.join(", "))
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<7}", status_label(kind)),
                ui.theme.kind_style(kind),
            ),
            Span::styled(format!("  {}{}", task.title, blocked), ui.theme.style(Typography::Body)),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn render_spec_coverage(
    frame: &mut Frame,
    area: Rect,
    ui: &UiContext,
    snap: &SpecCoverageSnapshot,
) {
    if snap.requirements.is_empty() {
        render_empty(
            frame,
            area,
            ui,
            "No requirements",
            "Specifications keep intent addressable. Uncovered requirements are listed with a warning, never hidden.",
        );
        return;
    }
    let mut lines = vec![
        Line::from(Span::styled("Specification coverage", ui.theme.style(Typography::Section))),
        Line::from(""),
    ];
    for req in &snap.requirements {
        let uncovered = req.symbols.is_empty() && req.tests.is_empty();
        let kind = if uncovered {
            StatusKind::Warning
        } else {
            StatusKind::Success
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<7}", status_label(kind)),
                ui.theme.kind_style(kind),
            ),
            Span::styled(&req.title, ui.theme.style(Typography::Body)),
        ]));
        lines.push(Line::from(Span::styled(
            format!(
                "         tasks {}  symbols {}  tests {}  commits {}",
                req.tasks.len(),
                req.symbols.len(),
                req.tests.len(),
                req.commits.len()
            ),
            ui.theme.style(Typography::Muted),
        )));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn render_shell_text(ui: &UiContext, snapshot: &AppSnapshot) -> String {
    let _ = ui;
    let mut out = String::new();
    out.push_str(&format!("ᚱ RUNE  {}\n", snapshot.title));
    let tabs: Vec<String> = ActiveView::all()
        .into_iter()
        .map(|view| {
            if view == snapshot.view {
                format!("[{}]", view.label())
            } else {
                view.label().to_string()
            }
        })
        .collect();
    out.push_str(&format!("{}\n", tabs.join("  ")));
    match snapshot.view {
        ActiveView::Home => {
            out.push_str(&format!(
                "files {}  symbols {}  memories {}  sessions {}\n",
                snapshot.dashboard.files,
                snapshot.dashboard.symbols,
                snapshot.dashboard.memories,
                snapshot.dashboard.sessions
            ));
        }
        ActiveView::Graph => {
            for node in snapshot.graph.visible_nodes().into_iter().take(12) {
                out.push_str(&format!("  {}  {}\n", node.kind, node.label));
            }
        }
        ActiveView::Memory => {
            for event in snapshot.memory.events.iter().take(12) {
                out.push_str(&format!("  {}  {}\n", event.state, event.statement));
            }
        }
        _ => {}
    }
    if snapshot.palette.is_open() {
        match &snapshot.palette.phase {
            PalettePhase::Idle => {}
            PalettePhase::Querying { query } => out.push_str(&format!("> {query}\n")),
            PalettePhase::Results {
                query,
                items,
                selected,
            }
            | PalettePhase::Preview {
                query,
                items,
                selected,
            } => {
                out.push_str(&format!("> {query}\n"));
                for (idx, item) in items.iter().enumerate() {
                    let mark = if idx == *selected { ">" } else { " " };
                    out.push_str(&format!("{mark} {}  {}\n", item.title, item.subtitle));
                }
            }
        }
    }
    out.push_str(&format!("{}\n", snapshot.status));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_state_supports_pan_zoom_focus_filter() {
        let mut g = GraphExplorerState::new();
        g.nodes.push(GraphNodeView {
            id: "a".into(),
            label: "Auth".into(),
            kind: "function".into(),
        });
        g.pan_by(2.0, 1.0);
        g.zoom_by(2.0);
        g.focus_node("a");
        g.expand_neighbors("a");
        g.filter_node_type("function");
        g.set_path(vec!["repo".into(), "a".into()]);
        assert_eq!(g.pan, (2.0, 1.0));
        assert!(g.zoom > 1.0);
        assert_eq!(g.focus.as_deref(), Some("a"));
        assert_eq!(g.visible_nodes().len(), 1);
        assert_eq!(g.path.len(), 2);
    }

    #[test]
    fn view_digits_and_cycle() {
        assert_eq!(ActiveView::from_digit('3'), Some(ActiveView::Memory));
        assert_eq!(ActiveView::Home.next(), ActiveView::Graph);
        assert_eq!(ActiveView::Home.prev(), ActiveView::Agents);
    }

    #[test]
    fn shell_text_includes_home_metrics() {
        let ui = crate::UiContext::new(rune_terminal::TerminalCapabilities::detect());
        let snapshot = crate::AppSnapshot {
            title: "demo".into(),
            dashboard: DashboardSnapshot {
                files: 12,
                symbols: 40,
                ..DashboardSnapshot::default()
            },
            ..crate::AppSnapshot::default()
        };
        let text = render_shell_text(&ui, &snapshot);
        assert!(text.contains("ᚱ RUNE"));
        assert!(text.contains("[Home]"));
        assert!(text.contains("files 12"));
        assert!(text.contains("symbols 40"));
    }
}
