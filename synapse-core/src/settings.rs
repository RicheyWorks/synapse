use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SynapseError;
use crate::persistence::{read_json, write_json_atomic};

fn default_daily_review_limit() -> u32 {
    20
}

fn default_theme() -> String {
    "neural".to_string()
}

fn default_scheduler() -> String {
    "sm2".to_string()
}

fn default_fsrs_desired_retention() -> f32 {
    0.9
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Settings {
    /// Max number of items a review session pulls in at once, by default.
    #[serde(default = "default_daily_review_limit")]
    pub daily_review_limit: u32,
    /// UI theme id, e.g. "neural" (default) or "blackbeard".
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Which `Scheduler` reviews are run through: "sm2" (default) or "fsrs".
    /// Existing items keep whatever fields their prior scheduler set; switching
    /// takes effect on each item's next review, there's no bulk migration.
    #[serde(default = "default_scheduler")]
    pub scheduler: String,
    /// FSRS's target probability of recall at the scheduled review (0.7-0.99).
    /// Unused when `scheduler` is "sm2".
    #[serde(default = "default_fsrs_desired_retention")]
    pub fsrs_desired_retention: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            daily_review_limit: default_daily_review_limit(),
            theme: default_theme(),
            scheduler: default_scheduler(),
            fsrs_desired_retention: default_fsrs_desired_retention(),
        }
    }
}

#[derive(Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Settings, SynapseError> {
        read_json(&self.path)
    }

    pub fn save(&self, settings: &Settings) -> Result<(), SynapseError> {
        write_json_atomic(&self.path, settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path().join("settings.json"));

        let settings = store.load().unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn round_trips_custom_settings_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path().join("settings.json"));

        let custom = Settings {
            daily_review_limit: 50,
            theme: "blackbeard".to_string(),
            scheduler: "fsrs".to_string(),
            fsrs_desired_retention: 0.85,
        };
        store.save(&custom).unwrap();

        assert_eq!(store.load().unwrap(), custom);
    }
}
