use std::fs;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::MemoryItem;
use crate::error::SynapseError;
use crate::store::{export_to_path, import_from_path};

/// Backups beyond this count are deleted, oldest first, so the backup
/// directory doesn't grow unbounded on a long-lived install.
const MAX_BACKUPS: usize = 10;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: DateTime<Utc>,
}

fn backup_filename(now: DateTime<Utc>) -> String {
    format!("memories-{}.json", now.format("%Y%m%dT%H%M%S%.3fZ"))
}

/// Writes a timestamped snapshot into `backup_dir`, then rotates out the
/// oldest backups beyond `MAX_BACKUPS`. Filenames sort chronologically
/// (ISO-ish timestamp), so lexicographic ordering is also creation ordering.
pub fn create_backup(items: &[MemoryItem], backup_dir: &Path) -> Result<String, SynapseError> {
    fs::create_dir_all(backup_dir).map_err(|source| SynapseError::Io {
        path: backup_dir.to_path_buf(),
        source,
    })?;

    let filename = backup_filename(Utc::now());
    export_to_path(items, &backup_dir.join(&filename))?;
    rotate_backups(backup_dir)?;
    Ok(filename)
}

fn json_backup_entries(backup_dir: &Path) -> Result<Vec<std::fs::DirEntry>, SynapseError> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<_> = fs::read_dir(backup_dir)
        .map_err(|source| SynapseError::Io {
            path: backup_dir.to_path_buf(),
            source,
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

fn rotate_backups(backup_dir: &Path) -> Result<(), SynapseError> {
    let entries = json_backup_entries(backup_dir)?;
    if entries.len() > MAX_BACKUPS {
        for entry in &entries[..entries.len() - MAX_BACKUPS] {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

/// Newest-first list of available backups.
pub fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>, SynapseError> {
    let mut entries = json_backup_entries(backup_dir)?;
    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));

    Ok(entries
        .into_iter()
        .map(|entry| {
            let created_at = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| DateTime::<Utc>::from(SystemTime::now()));
            BackupInfo {
                filename: entry.file_name().to_string_lossy().to_string(),
                created_at,
            }
        })
        .collect())
}

/// Loads a specific backup file's contents for the caller to apply (e.g. by
/// replacing the current in-memory set and persisting).
pub fn restore_backup(backup_dir: &Path, filename: &str) -> Result<Vec<MemoryItem>, SynapseError> {
    import_from_path(&backup_dir.join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardContent;

    #[test]
    fn create_and_restore_backup_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backups");

        let item = MemoryItem::new("Rust", "What is a backup?", CardContent::basic("A safety net"));
        let filename = create_backup(&[item.clone()], &backup_dir).unwrap();

        let restored = restore_backup(&backup_dir, &filename).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, item.id);
    }

    #[test]
    fn list_backups_is_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backups");

        create_backup(&[], &backup_dir).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let second = create_backup(&[], &backup_dir).unwrap();

        let backups = list_backups(&backup_dir).unwrap();
        assert_eq!(backups.len(), 2);
        assert_eq!(backups[0].filename, second);
    }

    #[test]
    fn rotation_keeps_only_the_newest_backups() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backups");

        for _ in 0..(MAX_BACKUPS + 3) {
            create_backup(&[], &backup_dir).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let backups = list_backups(&backup_dir).unwrap();
        assert_eq!(backups.len(), MAX_BACKUPS);
    }

    #[test]
    fn empty_backup_dir_yields_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let backups = list_backups(&dir.path().join("backups")).unwrap();
        assert!(backups.is_empty());
    }
}
