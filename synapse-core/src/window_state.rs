use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SynapseError;
use crate::persistence::{read_json, write_json_atomic};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct WindowState {
    /// `None` on first run: let the OS pick a placement rather than forcing (0, 0).
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1000,
            height: 720,
        }
    }
}

#[derive(Clone)]
pub struct WindowStateStore {
    path: PathBuf,
}

impl WindowStateStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<WindowState, SynapseError> {
        read_json(&self.path)
    }

    pub fn save(&self, state: &WindowState) -> Result<(), SynapseError> {
        write_json_atomic(&self.path, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_sane_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = WindowStateStore::new(dir.path().join("window_state.json"));

        let state = store.load().unwrap();
        assert_eq!(state, WindowState::default());
        assert!(state.width > 0 && state.height > 0);
    }

    #[test]
    fn round_trips_custom_state_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = WindowStateStore::new(dir.path().join("window_state.json"));

        let custom = WindowState {
            x: Some(120),
            y: Some(80),
            width: 1280,
            height: 800,
        };
        store.save(&custom).unwrap();

        assert_eq!(store.load().unwrap(), custom);
    }
}
