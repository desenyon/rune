use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("invalid identifier: {0}")]
    InvalidId(String),
    #[error("invalid fingerprint: {0}")]
    Fingerprint(String),
    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
