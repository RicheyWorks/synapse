use std::fs;
use std::path::PathBuf;

use crate::domain::MemoryItem;
use crate::error::SynapseError;

/// Persists the full set of memory items.
///
/// Implementations must guarantee that a crash mid-save never corrupts
/// previously-saved data (e.g. by writing to a temp file and renaming over
/// the target, which is atomic on the same filesystem).
pub trait MemoryStore {
    fn load(&self) -> Result<Vec<MemoryItem>, SynapseError>;
    fn save(&self, items: &[MemoryItem]) -> Result<(), SynapseError>;
}

/// JSON-on-disk store. Simple and human-inspectable; swap for a SQLite-backed
/// store later without touching callers, since they only depend on `MemoryStore`.
pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl MemoryStore for JsonFileStore {
    fn load(&self) -> Result<Vec<MemoryItem>, SynapseError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(&self.path).map_err(|source| SynapseError::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(serde_json::from_str(&data)?)
    }

    fn save(&self, items: &[MemoryItem]) -> Result<(), SynapseError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| SynapseError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let data = serde_json::to_string_pretty(items)?;
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, &data).map_err(|source| SynapseError::Io {
            path: tmp_path.clone(),
            source,
        })?;
        fs::rename(&tmp_path, &self.path).map_err(|source| SynapseError::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MemoryItem;

    #[test]
    fn round_trips_items_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonFileStore::new(dir.path().join("memories.json"));

        assert!(store.load().unwrap().is_empty());

        let item = MemoryItem::new("Rust", "What is a lifetime?", "A scope for borrows");
        store.save(&[item.clone()]).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, item.id);
        assert_eq!(loaded[0].prompt, item.prompt);
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested_path = dir.path().join("nested").join("deep").join("memories.json");
        let store = JsonFileStore::new(nested_path);

        store.save(&[]).unwrap();
        assert!(store.load().unwrap().is_empty());
    }
}
