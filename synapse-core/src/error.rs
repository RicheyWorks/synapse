use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SynapseError {
    #[error("memory item not found: {0}")]
    NotFound(String),

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to (de)serialize memory data: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid operation: {0}")]
    InvalidOperation(String),
}

impl SynapseError {
    /// Stable, frontend-matchable discriminant (kept separate from the
    /// human-readable `Display` message, which may change wording over time).
    fn kind(&self) -> &'static str {
        match self {
            SynapseError::NotFound(_) => "NotFound",
            SynapseError::Io { .. } => "Io",
            SynapseError::Serialization(_) => "Serialization",
            SynapseError::InvalidOperation(_) => "InvalidOperation",
        }
    }
}

/// Tauri (v1) requires command errors to implement `Serialize`, which
/// `std::io::Error` inside `SynapseError::Io` does not. Serialize as a
/// `{ kind, message }` object instead of collapsing to a bare string, so the
/// frontend can branch on `kind` rather than pattern-match display text.
impl Serialize for SynapseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SynapseError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
