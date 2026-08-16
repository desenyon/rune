use thiserror::Error;

pub type Result<T> = std::result::Result<T, TaskError>;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error(transparent)]
    Storage(#[from] rune_storage::StorageError),
    #[error("{0}")]
    Message(String),
    #[error("invalid task: {0}")]
    Invalid(String),
    #[error("dependency cycle: {0}")]
    Cycle(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl TaskError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub fn cycle(path: &[rune_core::NodeId]) -> Self {
        let rendered = path
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        Self::Cycle(rendered)
    }
}
