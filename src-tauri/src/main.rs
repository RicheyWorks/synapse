#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use std::sync::Mutex;

use synapse_core::domain::MemoryItem;
use synapse_core::scheduler::{Scheduler, Sm2Scheduler};
use synapse_core::stats::{compute_stats, Stats};
use synapse_core::store::{export_to_path, import_from_path, JsonFileStore, MemoryStore};
use tauri::Manager;

struct AppState {
    memories: Mutex<Vec<MemoryItem>>,
    store: JsonFileStore,
    scheduler: Sm2Scheduler,
}

impl AppState {
    fn persist(&self) -> Result<(), String> {
        let memories = self.memories.lock().unwrap();
        self.store.save(&memories).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn add_memory(
    state: tauri::State<'_, AppState>,
    track: String,
    prompt: String,
    content: String,
) -> Result<MemoryItem, String> {
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
) -> Result<MemoryItem, String> {
    let updated = {
        let mut memories = state.memories.lock().unwrap();
        let item = memories
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| format!("memory item not found: {id}"))?;
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
fn export_memories(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let memories = state.memories.lock().unwrap();
    export_to_path(&memories, Path::new(&path)).map_err(|e| e.to_string())
}

/// Merges items from a backup file into the current set: items whose id already
/// exists are overwritten, new ids are appended. Returns the number imported.
#[tauri::command]
fn import_memories(state: tauri::State<'_, AppState>, path: String) -> Result<usize, String> {
    let imported = import_from_path(Path::new(&path)).map_err(|e| e.to_string())?;
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

            app.manage(AppState {
                memories: Mutex::new(memories),
                store,
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
            export_memories,
            import_memories
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
