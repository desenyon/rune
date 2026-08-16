use crate::onboard::inspect_environment;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use rune_app::App;
use rune_core::NodeKind;
use rune_graph::Graph;
use rune_memory::MemoryTimeline;
use rune_search::{SearchEngine, SearchRequest};
use rune_specs::Coverage;
use rune_storage::Store;
use rune_tasks::TaskStore;
use rune_terminal::{RendererLevel, TerminalCapabilities};
use rune_ui::palette::PaletteItem;
use rune_ui::views::{
    ActiveView, AgentCard, AgentCockpitSnapshot, ContextInspectorSnapshot, DashboardSnapshot,
    GraphEdgeView, GraphExplorerState, GraphNodeView, InspectorItem, MemoryEvent,
    MemoryTimelineSnapshot, RequirementCoverage, SessionExplorerSnapshot, SessionRow,
    SpecCoverageSnapshot, TaskGraphSnapshot, TaskNodeView, TaskStatus as UiTaskStatus,
};
use rune_ui::{render_app, render_app_text, AppSnapshot, UiContext};
use std::io::{self, stdout, IsTerminal};
use std::path::Path;
use std::time::Duration;

pub fn run_tui(workspace: &Path) -> std::result::Result<(), String> {
    let caps = TerminalCapabilities::detect();
    let ui = UiContext::new(caps.clone());
    let app = App::open_or_create(workspace).ok();
    let store = app.as_ref().map(|app| &app.store);
    let mut snapshot = load_snapshot(workspace, store, &caps);

    if !stdout().is_terminal() || (caps.renderer_level == RendererLevel::Basic && !caps.is_tty) {
        print!("{}", render_app_text(&ui, &snapshot));
        return Ok(());
    }

    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| e.to_string())?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    let result = loop_ui(&mut terminal, &ui, &mut snapshot, store, workspace, &caps);
    disable_raw_mode().ok();
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen).ok();
    result
}

fn loop_ui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ui: &UiContext,
    snapshot: &mut AppSnapshot,
    store: Option<&Store>,
    workspace: &Path,
    caps: &TerminalCapabilities,
) -> std::result::Result<(), String> {
    loop {
        terminal
            .draw(|frame| render_app(frame, frame.area(), ui, snapshot))
            .map_err(|e| e.to_string())?;
        if !event::poll(Duration::from_millis(80)).map_err(|e| e.to_string())? {
            continue;
        }
        match event::read().map_err(|e| e.to_string())? {
            Event::Resize(_, _) => {}
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if handle_key(key, snapshot, store, workspace, caps) {
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_key(
    key: KeyEvent,
    snapshot: &mut AppSnapshot,
    store: Option<&Store>,
    workspace: &Path,
    caps: &TerminalCapabilities,
) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if ctrl && matches!(key.code, KeyCode::Char('p') | KeyCode::Char('k') | KeyCode::Char('P')) {
        if snapshot.palette.is_open() {
            snapshot.palette.close();
        } else {
            snapshot.palette.open();
            snapshot
                .palette
                .set_query(String::new(), palette_hits(store, ""));
        }
        return false;
    }
    if snapshot.palette.is_open() {
        return handle_palette_key(key, snapshot, store);
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => true,
        KeyCode::Tab => {
            snapshot.view = snapshot.view.next();
            false
        }
        KeyCode::BackTab => {
            snapshot.view = snapshot.view.prev();
            false
        }
        KeyCode::Char(c) => {
            if let Some(view) = ActiveView::from_digit(c) {
                snapshot.view = view;
            } else if c == 'r' {
                *snapshot = load_snapshot(workspace, store, caps);
            }
            false
        }
        _ => false,
    }
}

fn handle_palette_key(
    key: KeyEvent,
    snapshot: &mut AppSnapshot,
    store: Option<&Store>,
) -> bool {
    match key.code {
        KeyCode::Esc => {
            snapshot.palette.close();
            false
        }
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            snapshot.palette.select_next();
            false
        }
        KeyCode::Up | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            snapshot.palette.select_prev();
            false
        }
        KeyCode::Down => {
            snapshot.palette.select_next();
            false
        }
        KeyCode::Up => {
            snapshot.palette.select_prev();
            false
        }
        KeyCode::Enter => {
            if let Some(item) = snapshot.palette.selected_item().cloned() {
                snapshot.view = view_for_kind(&item.subtitle);
                snapshot.status = format!("opened  {}", item.title);
                snapshot.palette.close();
            } else {
                snapshot.palette.enter_preview();
            }
            false
        }
        KeyCode::Char(c) => {
            if let Some(query) = palette_query(snapshot) {
                let mut q = query;
                q.push(c);
                let hits = palette_hits(store, &q);
                snapshot.palette.set_query(q, hits);
            }
            false
        }
        KeyCode::Backspace => {
            if let Some(query) = palette_query(snapshot) {
                let mut q = query;
                q.pop();
                let hits = palette_hits(store, &q);
                snapshot.palette.set_query(q, hits);
            }
            false
        }
        _ => false,
    }
}

fn view_for_kind(kind: &str) -> ActiveView {
    match kind {
        "memory" => ActiveView::Memory,
        "session" | "turn" => ActiveView::Sessions,
        "task" => ActiveView::Tasks,
        "specification" | "requirement" => ActiveView::Specs,
        "context_capsule" => ActiveView::Context,
        "agent" => ActiveView::Agents,
        "action" => ActiveView::Home,
        _ => ActiveView::Graph,
    }
}

fn palette_query(snapshot: &AppSnapshot) -> Option<String> {
    match &snapshot.palette.phase {
        rune_ui::palette::PalettePhase::Querying { query }
        | rune_ui::palette::PalettePhase::Results { query, .. }
        | rune_ui::palette::PalettePhase::Preview { query, .. } => Some(query.clone()),
        rune_ui::palette::PalettePhase::Idle => None,
    }
}

fn load_snapshot(
    workspace: &Path,
    store: Option<&Store>,
    caps: &TerminalCapabilities,
) -> AppSnapshot {
    let env = inspect_environment(workspace);
    let mut snapshot = AppSnapshot {
        title: workspace.display().to_string(),
        status: format!(
            "{}  ·  {} agents  ·  {} tools",
            if env.languages.is_empty() {
                "unindexed".into()
            } else {
                env.languages.join(" · ")
            },
            env.coding_agents.len(),
            env.tools.len()
        ),
        renderer_level: format!("{:?}", caps.renderer_level),
        dashboard: DashboardSnapshot {
            languages: env.languages.clone(),
            agents: env.coding_agents.clone(),
            ..DashboardSnapshot::default()
        },
        agents: AgentCockpitSnapshot {
            agents: env
                .coding_agents
                .iter()
                .map(|name| AgentCard {
                    provider: name.clone(),
                    model: None,
                    task: "detected locally".into(),
                    worktree: None,
                    branch: None,
                    current_action: "idle".into(),
                    context_usage: "—".into(),
                    tests: "—".into(),
                    status: "detected".into(),
                    elapsed: "—".into(),
                    recent_events: Vec::new(),
                })
                .collect(),
        },
        ..AppSnapshot::default()
    };
    let Some(store) = store else {
        return snapshot;
    };
    snapshot.dashboard.files = count_kind(store, NodeKind::File);
    snapshot.dashboard.symbols = count_kind(store, NodeKind::Function)
        + count_kind(store, NodeKind::Method)
        + count_kind(store, NodeKind::Class);
    snapshot.dashboard.memories = count_kind(store, NodeKind::Memory);
    snapshot.dashboard.sessions = count_kind(store, NodeKind::Session);
    snapshot.dashboard.tasks = count_kind(store, NodeKind::Task);
    snapshot.dashboard.specs = count_kind(store, NodeKind::Specification);
    snapshot.dashboard.commits = count_kind(store, NodeKind::Commit);
    snapshot.graph = load_graph(store);
    snapshot.memory = load_memory(store);
    snapshot.sessions = load_sessions(store);
    snapshot.tasks = load_tasks(store);
    snapshot.specs = load_specs(store);
    snapshot.inspector = load_inspector(store);
    snapshot
}

fn count_kind(store: &Store, kind: NodeKind) -> u64 {
    store
        .nodes_of_kind(kind)
        .map(|nodes| nodes.len() as u64)
        .unwrap_or(0)
}

fn load_graph(store: &Store) -> GraphExplorerState {
    let mut graph = GraphExplorerState::new();
    for kind in [
        NodeKind::Repository,
        NodeKind::File,
        NodeKind::Function,
        NodeKind::Memory,
        NodeKind::Session,
        NodeKind::Task,
        NodeKind::Commit,
    ] {
        if let Ok(nodes) = store.nodes_of_kind(kind.clone()) {
            for node in nodes.into_iter().take(12) {
                let id = node.id.to_string();
                if graph.focus.is_none() {
                    graph.focus = Some(id.clone());
                }
                if let Ok(neighbors) = Graph::new(store).neighbors(node.id) {
                    for neighbor in neighbors.into_iter().take(6) {
                        graph.edges.push(GraphEdgeView {
                            from: id.clone(),
                            to: neighbor.node.id.to_string(),
                            kind: neighbor.edge.kind.as_str().to_string(),
                        });
                    }
                }
                graph.nodes.push(GraphNodeView {
                    id,
                    label: node.name.unwrap_or_else(|| node.id.to_string()),
                    kind: node.kind.as_str().to_string(),
                });
            }
        }
    }
    graph
}

fn load_memory(store: &Store) -> MemoryTimelineSnapshot {
    let events = MemoryTimeline::new(store)
        .events()
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(40)
        .map(|event| MemoryEvent {
            id: event.memory_id.to_string(),
            statement: event.statement,
            state: format!("{:?}", event.kind).to_ascii_lowercase(),
            at: event.at.to_string(),
        })
        .collect();
    MemoryTimelineSnapshot { events }
}

fn load_sessions(store: &Store) -> SessionExplorerSnapshot {
    let sessions = store
        .nodes_of_kind(NodeKind::Session)
        .unwrap_or_default()
        .into_iter()
        .map(|node| SessionRow {
            provider: node
                .payload
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            project: node
                .payload
                .get("cwd")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            outcome: node
                .payload
                .get("goal")
                .and_then(|v| v.as_str())
                .unwrap_or("imported")
                .to_string(),
            when: node.updated_at.to_string(),
            id: node.id.to_string(),
        })
        .collect();
    SessionExplorerSnapshot {
        sessions,
        provider_filter: None,
    }
}

fn load_tasks(store: &Store) -> TaskGraphSnapshot {
    let tasks = TaskStore::new(store)
        .list()
        .unwrap_or_default()
        .into_iter()
        .map(|task| TaskNodeView {
            id: task.id.to_string(),
            title: task.title,
            status: match task.status {
                rune_tasks::TaskStatus::Ready => UiTaskStatus::Ready,
                rune_tasks::TaskStatus::Active => UiTaskStatus::Active,
                rune_tasks::TaskStatus::Blocked => UiTaskStatus::Blocked,
                rune_tasks::TaskStatus::Failed => UiTaskStatus::Failed,
                rune_tasks::TaskStatus::Review => UiTaskStatus::Review,
                rune_tasks::TaskStatus::Complete => UiTaskStatus::Complete,
            },
            blocked_by: task.blockers.iter().map(|id| id.to_string()).collect(),
        })
        .collect();
    TaskGraphSnapshot { tasks }
}

fn load_specs(store: &Store) -> SpecCoverageSnapshot {
    let mut requirements = Vec::new();
    if let Ok(nodes) = store.nodes_of_kind(NodeKind::Specification) {
        let coverage = Coverage::new(store);
        for node in nodes {
            if let Ok(report) = coverage.for_specification(node.id) {
                for item in report.requirements {
                    requirements.push(RequirementCoverage {
                        id: item.requirement.id.to_string(),
                        title: item.requirement.text,
                        tasks: Vec::new(),
                        symbols: item
                            .implementing_nodes
                            .iter()
                            .map(|id| id.to_string())
                            .collect(),
                        tests: Vec::new(),
                        commits: Vec::new(),
                        status: if item.covered {
                            "covered".into()
                        } else {
                            "uncovered".into()
                        },
                    });
                }
            }
        }
    }
    SpecCoverageSnapshot { requirements }
}

fn load_inspector(store: &Store) -> Option<ContextInspectorSnapshot> {
    let capsules = store.nodes_of_kind(NodeKind::ContextCapsule).ok()?;
    let node = capsules.last()?;
    let included = node
        .payload
        .get("included")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let items = included
        .iter()
        .filter_map(|item| {
            Some(InspectorItem {
                name: item.get("name")?.as_str()?.to_string(),
                reason: item
                    .get("reason")
                    .and_then(|v| v.get("explanation"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                tokens: item.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                stale: item
                    .get("warnings")
                    .and_then(|v| v.as_array())
                    .map(|w| !w.is_empty())
                    .unwrap_or(false),
            })
        })
        .collect();
    Some(ContextInspectorSnapshot {
        token_estimate: node
            .payload
            .get("token_estimate")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        budget: node
            .payload
            .get("budget")
            .and_then(|v| v.get("total"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        allocations: Vec::new(),
        included: items,
        excluded: Vec::new(),
        stale: Vec::new(),
        contradictions: Vec::new(),
        duplicates_removed: node
            .payload
            .get("duplicates_removed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        compression_notes: Vec::new(),
    })
}

fn palette_hits(store: Option<&Store>, query: &str) -> Vec<PaletteItem> {
    let Some(store) = store else {
        return Vec::new();
    };
    if query.trim().is_empty() {
        return store
            .nodes_of_kind(NodeKind::File)
            .unwrap_or_default()
            .into_iter()
            .take(24)
            .map(item_from_node)
            .collect();
    }
    let mut request = SearchRequest::new(query);
    request.limit = 32;
    match SearchEngine::new(store).search(request) {
        Ok(response) => response
            .hits
            .into_iter()
            .map(|hit| item_from_node(hit.node))
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn item_from_node(node: rune_core::Node) -> PaletteItem {
    PaletteItem::search(
        node.id.to_string(),
        node.name.unwrap_or_else(|| node.id.to_string()),
        node.kind.as_str(),
    )
}
