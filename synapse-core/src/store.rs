use std::path::{Path, PathBuf};

use crate::domain::MemoryItem;
use crate::error::SynapseError;
use crate::persistence::{read_json, write_json_atomic};

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
        read_json(&self.path)
    }

    fn save(&self, items: &[MemoryItem]) -> Result<(), SynapseError> {
        write_json_atomic(&self.path, &items)
    }
}

/// Writes items to an arbitrary path chosen by the user (e.g. a backup file),
/// as opposed to the app's own persisted store location. Same atomic-write
/// JSON format as `JsonFileStore`, so exported files are also valid imports.
pub fn export_to_path(items: &[MemoryItem], path: &Path) -> Result<(), SynapseError> {
    JsonFileStore::new(path.to_path_buf()).save(items)
}

/// Reads items from a user-chosen JSON file, e.g. a previously exported backup.
pub fn import_from_path(path: &Path) -> Result<Vec<MemoryItem>, SynapseError> {
    JsonFileStore::new(path.to_path_buf()).load()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CardContent, MemoryItem};

    #[test]
    fn export_then_import_round_trips_through_a_backup_file() {
        let dir = tempfile::tempdir().unwrap();
        let backup_path = dir.path().join("backup.json");

        let item = MemoryItem::new(
            "Biology",
            "What is a mitochondrion?",
            CardContent::basic("The powerhouse of the cell"),
        );
        export_to_path(&[item.clone()], &backup_path).unwrap();

        let restored = import_from_path(&backup_path).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, item.id);
    }

    #[test]
    fn round_trips_items_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonFileStore::new(dir.path().join("memories.json"));

        assert!(store.load().unwrap().is_empty());

        let item = MemoryItem::new("Rust", "What is a lifetime?", CardContent::basic("A scope for borrows"));
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
