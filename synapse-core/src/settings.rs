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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Settings {
    /// Max number of items a review session pulls in at once, by default.
    #[serde(default = "default_daily_review_limit")]
    pub daily_review_limit: u32,
    /// UI theme id, e.g. "neural" (default) or "blackbeard".
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            daily_review_limit: default_daily_review_limit(),
            theme: default_theme(),
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
        };
        store.save(&custom).unwrap();

        assert_eq!(store.load().unwrap(), custom);
    }
}
