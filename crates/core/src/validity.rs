use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Lifecycle of a stored object. Derived facts use this plus provenance.derived.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validity {
    Active,
    Candidate,
    Verified,
    Stable,
    Stale,
    Contradicted,
    Superseded,
    Archived,
    Invalid,
}

impl Validity {
    pub fn is_current_guidance(self) -> bool {
        matches!(self, Validity::Verified | Validity::Stable | Validity::Active)
    }

    pub fn may_guide_agents(self) -> bool {
        matches!(self, Validity::Verified | Validity::Stable)
    }
}

impl Display for Validity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| format!("{self:?}").to_lowercase());
        f.write_str(&name)
    }
}

impl Default for Validity {
    fn default() -> Self {
        Validity::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_must_not_guide_agents() {
        assert!(!Validity::Stale.may_guide_agents());
        assert!(!Validity::Candidate.may_guide_agents());
        assert!(Validity::Verified.may_guide_agents());
    }
}
