use crate::keys::action_name;
use nucleo::{Config, Matcher, Utf32Str};
use rune_core::ActionKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaletteItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub kind_label: String,
    pub action: ActionKind,
}

impl PaletteItem {
    pub fn action(kind: ActionKind) -> Self {
        let title = action_name(&kind);
        Self {
            id: format!("action:{title}"),
            subtitle: "action".into(),
            kind_label: "action".into(),
            title,
            action: kind,
        }
    }

    pub fn search(
        id: impl Into<String>,
        title: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        let title = title.into();
        Self {
            id: id.into(),
            subtitle: kind.into(),
            kind_label: "object".into(),
            action: ActionKind::Open,
            title,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PalettePhase {
    Idle,
    Querying {
        query: String,
    },
    Results {
        query: String,
        items: Vec<PaletteItem>,
        selected: usize,
    },
    Preview {
        query: String,
        items: Vec<PaletteItem>,
        selected: usize,
    },
}

impl Default for PalettePhase {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PaletteState {
    pub phase: PalettePhase,
    pub all_actions: Vec<PaletteItem>,
}

impl PaletteState {
    pub fn new() -> Self {
        let all_actions = default_actions();
        Self {
            phase: PalettePhase::Idle,
            all_actions,
        }
    }

    pub fn open(&mut self) {
        self.phase = PalettePhase::Querying {
            query: String::new(),
        };
    }

    pub fn close(&mut self) {
        self.phase = PalettePhase::Idle;
    }

    pub fn is_open(&self) -> bool {
        !matches!(self.phase, PalettePhase::Idle)
    }

    pub fn toggle(&mut self) {
        if self.is_open() {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn set_query(&mut self, query: String, search_hits: Vec<PaletteItem>) {
        let mut items = self.filter_actions(&query);
        items.extend(search_hits);
        items = fuzzy_order(&query, items);
        if query.is_empty() && items.is_empty() {
            self.phase = PalettePhase::Querying { query };
            return;
        }
        self.phase = PalettePhase::Results {
            selected: 0,
            query,
            items,
        };
    }

    pub fn select_next(&mut self) {
        if let PalettePhase::Results {
            items, selected, ..
        }
        | PalettePhase::Preview {
            items, selected, ..
        } = &mut self.phase
        {
            if !items.is_empty() {
                *selected = (*selected + 1) % items.len();
            }
        }
    }

    pub fn select_prev(&mut self) {
        if let PalettePhase::Results {
            items, selected, ..
        }
        | PalettePhase::Preview {
            items, selected, ..
        } = &mut self.phase
        {
            if !items.is_empty() {
                *selected = if *selected == 0 {
                    items.len() - 1
                } else {
                    *selected - 1
                };
            }
        }
    }

    pub fn enter_preview(&mut self) {
        if let PalettePhase::Results {
            query,
            items,
            selected,
        } = &self.phase
        {
            self.phase = PalettePhase::Preview {
                query: query.clone(),
                items: items.clone(),
                selected: *selected,
            };
        }
    }

    pub fn selected_item(&self) -> Option<&PaletteItem> {
        match &self.phase {
            PalettePhase::Results {
                items, selected, ..
            }
            | PalettePhase::Preview {
                items, selected, ..
            } => items.get(*selected),
            _ => None,
        }
    }

    pub fn selected_action(&self) -> Option<ActionKind> {
        self.selected_item().map(|item| item.action.clone())
    }

    fn filter_actions(&self, query: &str) -> Vec<PaletteItem> {
        if query.is_empty() {
            return self.all_actions.clone();
        }
        fuzzy_order(query, self.all_actions.clone())
    }
}

fn default_actions() -> Vec<PaletteItem> {
    [
        ActionKind::Open,
        ActionKind::Inspect,
        ActionKind::SearchReferences,
        ActionKind::ShowHistory,
        ActionKind::CompileContext,
        ActionKind::AssignAgent,
        ActionKind::Handoff,
        ActionKind::RunTests,
        ActionKind::OpenWorktree,
        ActionKind::Compare,
        ActionKind::Export,
        ActionKind::Archive,
        ActionKind::Pin,
        ActionKind::Unpin,
        ActionKind::Exclude,
        ActionKind::CommandPalette,
        ActionKind::Quit,
    ]
    .into_iter()
    .map(PaletteItem::action)
    .collect()
}

fn fuzzy_order(query: &str, items: Vec<PaletteItem>) -> Vec<PaletteItem> {
    if query.is_empty() {
        return items;
    }
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut scored = Vec::new();
    for item in items {
        let hay = format!("{} {}", item.title, item.subtitle);
        let mut hbuf = Vec::new();
        let mut nbuf = Vec::new();
        if let Some(score) = matcher.fuzzy_match(
            Utf32Str::new(&hay, &mut hbuf),
            Utf32Str::new(query, &mut nbuf),
        ) {
            scored.push((score, item));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, item)| item).collect()
}
