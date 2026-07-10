use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::SynapseError;

/// Reads and deserializes JSON from `path`, or returns `T::default()` if the
/// file doesn't exist yet (e.g. first run, before anything has been saved).
pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<T, SynapseError> {
    if !path.exists() {
        return Ok(T::default());
    }
    let data = fs::read_to_string(path).map_err(|source| SynapseError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_str(&data)?)
}

/// Serializes `value` to `path`, writing to a temp file and renaming over the
/// target so a crash mid-write never corrupts previously-saved data.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), SynapseError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| SynapseError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let data = serde_json::to_string_pretty(value)?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &data).map_err(|source| SynapseError::Io {
        path: tmp_path.clone(),
        source,
    })?;
    fs::rename(&tmp_path, path).map_err(|source| SynapseError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}
