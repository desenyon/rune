use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use uuid::Uuid;

macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            pub fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}{}", $prefix, self.0.hyphenated())
            }
        }

        impl std::str::FromStr for $name {
            type Err = IdParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let rest = s
                    .strip_prefix($prefix)
                    .ok_or_else(|| IdParseError::MissingPrefix {
                        expected: $prefix,
                        value: s.to_string(),
                    })?;
                let uuid = Uuid::parse_str(rest).map_err(|source| IdParseError::InvalidUuid {
                    value: s.to_string(),
                    source,
                })?;
                Ok(Self(uuid))
            }
        }
    };
}

typed_id!(NodeId, "nod_");
typed_id!(EdgeId, "edg_");
typed_id!(ProvenanceId, "prv_");

#[derive(Debug, Error)]
pub enum IdParseError {
    #[error("expected prefix `{expected}` in `{value}`")]
    MissingPrefix { expected: &'static str, value: String },
    #[error("invalid uuid in `{value}`: {source}")]
    InvalidUuid {
        value: String,
        source: uuid::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_prefixed_ids() {
        let node = NodeId::generate();
        let text = node.to_string();
        assert!(text.starts_with("nod_"));
        assert_eq!(text.parse::<NodeId>().unwrap(), node);
    }

    #[test]
    fn rejects_wrong_prefix() {
        let node = NodeId::generate();
        let as_edge = format!("edg_{}", node.as_uuid().hyphenated());
        assert!(as_edge.parse::<NodeId>().is_err());
    }
}
