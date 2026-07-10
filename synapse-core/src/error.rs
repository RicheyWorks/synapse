use std::path::PathBuf;
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
}
