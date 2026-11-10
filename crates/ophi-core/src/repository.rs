use crate::{Case, Entry, HistoryPage, NewCase, NewEntry};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("reference target must exist in the same Case: {0}")]
    InvalidReference(String),
    #[error("unsupported database schema version: {0}")]
    UnsupportedSchema(i64),
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) | Self::Json(_) => "invalid_input",
            Self::NotFound { .. } => "not_found",
            Self::InvalidReference(_) => "invalid_reference",
            Self::UnsupportedSchema(_) => "unsupported_schema",
            Self::Storage(_) => "storage_error",
        }
    }
}

/// Writes create new records only. Implementations must commit each write atomically.
pub trait Repository {
    fn create_case(&mut self, input: NewCase) -> Result<Case, Error>;
    fn get_case(&self, id: &str) -> Result<Option<Case>, Error>;
    fn append(&mut self, input: NewEntry) -> Result<Entry, Error>;
    fn get(&self, id: &str) -> Result<Option<Entry>, Error>;
    /// Ascending append order, with a limit in 1..=1000.
    fn history(
        &self,
        case_id: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<HistoryPage, Error>;
}
