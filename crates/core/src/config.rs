use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration layers from lowest to highest precedence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLayer {
    Defaults,
    User,
    Workspace,
    Session,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigValue {
    pub layer: ConfigLayer,
    pub key: String,
    pub value: Value,
}

/// Layered configuration. Later layers override earlier layers for the same key.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LayeredConfig {
    layers: IndexMap<String, Vec<ConfigValue>>,
}

impl LayeredConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, layer: ConfigLayer, key: impl Into<String>, value: Value) {
        let key = key.into();
        let values = self.layers.entry(key.clone()).or_default();
        if let Some(existing) = values.iter_mut().find(|item| item.layer == layer) {
            existing.value = value;
            return;
        }
        values.push(ConfigValue {
            layer,
            key,
            value,
        });
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        let values = self.layers.get(key)?;
        let mut best: Option<&ConfigValue> = None;
        for item in values {
            match (best, item.layer) {
                (None, _) => best = Some(item),
                (Some(current), layer) if layer_rank(layer) >= layer_rank(current.layer) => {
                    best = Some(item)
                }
                _ => {}
            }
        }
        best.map(|item| &item.value)
    }

    pub fn get_layer(&self, key: &str, layer: ConfigLayer) -> Option<&Value> {
        self.layers
            .get(key)?
            .iter()
            .find(|item| item.layer == layer)
            .map(|item| &item.value)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.layers.keys().map(String::as_str)
    }

    pub fn merge(mut self, other: LayeredConfig) -> Self {
        for (key, values) in other.layers {
            for value in values {
                self.set(value.layer, key.clone(), value.value);
            }
        }
        self
    }
}

fn layer_rank(layer: ConfigLayer) -> u8 {
    match layer {
        ConfigLayer::Defaults => 0,
        ConfigLayer::User => 1,
        ConfigLayer::Workspace => 2,
        ConfigLayer::Session => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn session_overrides_workspace() {
        let mut cfg = LayeredConfig::new();
        cfg.set(ConfigLayer::Defaults, "theme", json!("base"));
        cfg.set(ConfigLayer::Workspace, "theme", json!("workspace-dark"));
        cfg.set(ConfigLayer::Session, "theme", json!("high-contrast"));
        assert_eq!(cfg.get("theme"), Some(&json!("high-contrast")));
        assert_eq!(
            cfg.get_layer("theme", ConfigLayer::Defaults),
            Some(&json!("base"))
        );
    }
}
