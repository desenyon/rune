use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// UTC timestamp stored as milliseconds since epoch for SQLite friendliness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn now() -> Self {
        Self(Utc::now().timestamp_millis())
    }

    pub fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    pub fn as_millis(self) -> i64 {
        self.0
    }

    pub fn to_utc(self) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(self.0)
            .single()
            .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap())
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utc().to_rfc3339())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value.timestamp_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_after_epoch() {
        assert!(Timestamp::now().as_millis() > 0);
    }
}
