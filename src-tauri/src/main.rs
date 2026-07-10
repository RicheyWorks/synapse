#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use std::sync::Mutex;

use synapse_core::domain::MemoryItem;
use synapse_core::error::SynapseError;
use synapse_core::scheduler::{Scheduler, Sm2Scheduler};
use synapse_core::settings::{Settings, SettingsStore};
use synapse_core::stats::{compute_stats, list_tracks, Stats, TrackSummary};
use synapse_core::store::{export_to_path, import_from_path, JsonFileStore, MemoryStore};
use tauri::Manager;

struct AppState {
    memories: Mutex<Vec<MemoryItem>>,
    settings: Mutex<Settings>,
    store: JsonFileStore,
    settings_store: SettingsStore,
    scheduler: Sm2Scheduler,
}

impl AppState {
    fn persist(&self) -> Result<(), SynapseError> {
        let memories = self.memories.lock().unwrap();
        self.store.save(&memories)
    }
}

#[tauri::command]
fn add_memory(
    state: tauri::State<'_, AppState>,
    track: String,
    prompt: String,
    content: String,
) -> Result<MemoryItem, SynapseError> {
    let new_item = MemoryItem::new(&track, &prompt, &content);
    {
        let mut memories = state.memories.lock().unwrap();
        memories.push(new_item.clone());
    }
    state.persist()?;
    Ok(new_item)
}

#[tauri::command]
fn review_memory(
    state: tauri::State<'_, AppState>,
    id: String,
    score: u8,
) -> Result<MemoryItem, SynapseError> {
    let updated = {
        let mut memories = state.memories.lock().unwrap();
        let item = memories
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| SynapseError::NotFound(id.clone()))?;
        state.scheduler.schedule(item, score);
        item.clone()
    };
    state.persist()?;
    Ok(updated)
}

#[tauri::command]
fn get_due_memories(state: tauri::State<'_, AppState>) -> Vec<MemoryItem> {
    let memories = state.memories.lock().unwrap();
    memories.iter().filter(|m| m.is_due()).cloned().collect()
}

#[tauri::command]
fn get_all_memories(state: tauri::State<'_, AppState>) -> Vec<MemoryItem> {
    state.memories.lock().unwrap().clone()
}

#[tauri::command]
fn get_stats(state: tauri::State<'_, AppState>) -> Stats {
    let memories = state.memories.lock().unwrap();
    compute_stats(&memories)
}

#[tauri::command]
fn list_all_tracks(state: tauri::State<'_, AppState>) -> Vec<TrackSummary> {
    let memories = state.memories.lock().unwrap();
    list_tracks(&memories)
}

/// Returns the due items for a review session, most-overdue first, capped at
/// `limit` (or the user's configured daily review limit if not given).
#[tauri::command]
fn start_review_session(state: tauri::State<'_, AppState>, limit: Option<usize>) -> Vec<MemoryItem> {
    let memories = state.memories.lock().unwrap();
    let cap = limit.unwrap_or_else(|| state.settings.lock().unwrap().daily_review_limit as usize);

    let mut due: Vec<MemoryItem> = memories.iter().filter(|m| m.is_due()).cloned().collect();
    due.sort_by_key(|m| m.next_review);
    due.truncate(cap);
    due
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn update_settings(state: tauri::State<'_, AppState>, settings: Settings) -> Result<Settings, SynapseError> {
    state.settings_store.save(&settings)?;
    *state.settings.lock().unwrap() = settings.clone();
    Ok(settings)
}

#[tauri::command]
fn export_memories(state: tauri::State<'_, AppState>, path: String) -> Result<(), SynapseError> {
    let memories = state.memories.lock().unwrap();
    export_to_path(&memories, Path::new(&path))
}

/// Merges items from a backup file into the current set: items whose id already
/// exists are overwritten, new ids are appended. Returns the number imported.
#[tauri::command]
fn import_memories(state: tauri::State<'_, AppState>, path: String) -> Result<usize, SynapseError> {
    let imported = import_from_path(Path::new(&path))?;
    let imported_count = imported.len();
    {
        let mut memories = state.memories.lock().unwrap();
        for item in imported {
            match memories.iter_mut().find(|m| m.id == item.id) {
                Some(existing) => *existing = item,
                None => memories.push(item),
            }
        }
    }
    state.persist()?;
    Ok(imported_count)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path_resolver()
                .app_data_dir()
                .expect("could not resolve app data directory");

            let store = JsonFileStore::new(data_dir.join("memories.json"));
            let memories = store.load().expect("failed to load memory store");

            let settings_store = SettingsStore::new(data_dir.join("settings.json"));
            let settings = settings_store.load().expect("failed to load settings");

            app.manage(AppState {
                memories: Mutex::new(memories),
                settings: Mutex::new(settings),
                store,
                settings_store,
                scheduler: Sm2Scheduler,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_memory,
            review_memory,
            get_due_memories,
            get_all_memories,
            get_stats,
            list_all_tracks,
            start_review_session,
            get_settings,
            update_settings,
            export_memories,
            import_memories
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
