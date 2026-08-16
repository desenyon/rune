use rune_core::ActionKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KeyChord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub code: String,
}

impl KeyChord {
    pub fn parse(spec: &str) -> Result<Self, KeyError> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut code = None;
        for part in spec.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" | "option" => alt = true,
                "shift" => shift = true,
                other if other.is_empty() => {}
                other => {
                    if code.is_some() {
                        return Err(KeyError::Invalid(spec.to_string()));
                    }
                    code = Some(other.to_string());
                }
            }
        }
        Ok(Self {
            ctrl,
            alt,
            shift,
            code: code.ok_or_else(|| KeyError::Invalid(spec.to_string()))?,
        })
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.alt {
            parts.push("alt");
        }
        if self.shift {
            parts.push("shift");
        }
        parts.push(self.code.as_str());
        parts.join("+")
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KeyError {
    #[error("invalid key spec: {0}")]
    Invalid(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyConflict {
    pub chord: KeyChord,
    pub first: String,
    pub second: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeyMap {
    bindings: BTreeMap<KeyChord, ActionKind>,
}

impl KeyMap {
    pub fn default_bindings() -> Self {
        let mut map = Self::default();
        let _ = map.bind_str("ctrl+p", ActionKind::CommandPalette);
        let _ = map.bind_str("ctrl+k", ActionKind::FocusSearch);
        let _ = map.bind_str("tab", ActionKind::Unknown("next_view".into()));
        let _ = map.bind_str("q", ActionKind::Quit);
        let _ = map.bind_str("enter", ActionKind::Open);
        let _ = map.bind_str("i", ActionKind::Inspect);
        let _ = map.bind_str("h", ActionKind::ShowHistory);
        let _ = map.bind_str("c", ActionKind::CompileContext);
        let _ = map.bind_str("x", ActionKind::Export);
        map
    }

    pub fn bind(&mut self, chord: KeyChord, action: ActionKind) -> Result<(), KeyConflict> {
        if let Some(existing) = self.bindings.get(&chord) {
            if existing != &action {
                return Err(KeyConflict {
                    chord,
                    first: action_name(existing),
                    second: action_name(&action),
                });
            }
        }
        self.bindings.insert(chord, action);
        Ok(())
    }

    pub fn bind_str(&mut self, spec: &str, action: ActionKind) -> Result<(), KeyConflict> {
        let chord = KeyChord::parse(spec).expect("valid default spec");
        self.bind(chord, action)
    }

    pub fn get(&self, chord: &KeyChord) -> Option<&ActionKind> {
        self.bindings.get(chord)
    }

    pub fn detect_conflicts(&self, extra: &[(KeyChord, ActionKind)]) -> Vec<KeyConflict> {
        let mut conflicts = Vec::new();
        let mut seen = self.bindings.clone();
        for (chord, action) in extra {
            if let Some(existing) = seen.get(chord) {
                if existing != action {
                    conflicts.push(KeyConflict {
                        chord: chord.clone(),
                        first: action_name(existing),
                        second: action_name(action),
                    });
                }
            } else {
                seen.insert(chord.clone(), action.clone());
            }
        }
        conflicts
    }
}

pub fn action_name(kind: &ActionKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{kind:?}").to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keybinding_conflict_detected() {
        let map = KeyMap::default_bindings();
        let chord = KeyChord::parse("ctrl+p").unwrap();
        let conflicts = map.detect_conflicts(&[(chord, ActionKind::Quit)]);
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts[0].second, "quit");
    }

    #[test]
    fn bind_rejects_second_action_on_same_chord() {
        let mut map = KeyMap::default();
        map.bind_str("ctrl+k", ActionKind::FocusSearch).unwrap();
        let err = map.bind_str("ctrl+k", ActionKind::Quit).unwrap_err();
        assert_eq!(err.first, "focus_search");
        assert_eq!(err.second, "quit");
    }
}
