#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use synapse_core::backup::{self, BackupInfo};
use synapse_core::domain::{CardContent, MemoryItem};
use synapse_core::error::SynapseError;
use synapse_core::gamification::{compute_gamification, GamificationSummary};
use synapse_core::graph::{self, KnowledgeGraph};
use synapse_core::scheduler::{Scheduler, Sm2Scheduler};
use synapse_core::settings::{Settings, SettingsStore};
use synapse_core::stats::{
    compute_stats, forgetting_curve, hardest_items, list_tracks, retention_over_time, review_heatmap,
    HeatmapDay, RetentionPoint, Stats, TrackSummary,
};
use synapse_core::store::{export_to_path, import_from_path, JsonFileStore, MemoryStore};
use synapse_core::window_state::{WindowState, WindowStateStore};
use tauri::Manager;

struct AppState {
    memories: Mutex<Vec<MemoryItem>>,
    settings: Mutex<Settings>,
    store: JsonFileStore,
    settings_store: SettingsStore,
    scheduler: Sm2Scheduler,
    backup_dir: PathBuf,
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
    card: CardContent,
) -> Result<MemoryItem, SynapseError> {
    let new_item = MemoryItem::new(&track, &prompt, card);
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
/// Snapshots the pre-import state first, so a bad import is always undoable
/// via `restore_backup`.
#[tauri::command]
fn import_memories(state: tauri::State<'_, AppState>, path: String) -> Result<usize, SynapseError> {
    let imported = import_from_path(Path::new(&path))?;
    let imported_count = imported.len();
    {
        let mut memories = state.memories.lock().unwrap();
        backup::create_backup(&memories, &state.backup_dir)?;
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

/// Manually snapshots the current vault. Returns the backup's filename.
#[tauri::command]
fn create_manual_backup(state: tauri::State<'_, AppState>) -> Result<String, SynapseError> {
    let memories = state.memories.lock().unwrap();
    backup::create_backup(&memories, &state.backup_dir)
}

#[tauri::command]
fn list_backups(state: tauri::State<'_, AppState>) -> Result<Vec<BackupInfo>, SynapseError> {
    backup::list_backups(&state.backup_dir)
}

/// Replaces the entire current vault with the contents of a backup file.
/// Snapshots the pre-restore state first, in case the restore itself was a mistake.
#[tauri::command]
fn restore_backup(state: tauri::State<'_, AppState>, filename: String) -> Result<usize, SynapseError> {
    let restored = backup::restore_backup(&state.backup_dir, &filename)?;
    let restored_count = restored.len();
    {
        let mut memories = state.memories.lock().unwrap();
        backup::create_backup(&memories, &state.backup_dir)?;
        *memories = restored;
    }
    state.persist()?;
    Ok(restored_count)
}

#[tauri::command]
fn get_knowledge_graph(state: tauri::State<'_, AppState>) -> KnowledgeGraph {
    let memories = state.memories.lock().unwrap();
    graph::build_graph(&memories)
}

#[tauri::command]
fn link_memories(state: tauri::State<'_, AppState>, id_a: String, id_b: String) -> Result<(), SynapseError> {
    {
        let mut memories = state.memories.lock().unwrap();
        graph::link(&mut memories, &id_a, &id_b)?;
    }
    state.persist()
}

#[tauri::command]
fn unlink_memories(state: tauri::State<'_, AppState>, id_a: String, id_b: String) -> Result<(), SynapseError> {
    {
        let mut memories = state.memories.lock().unwrap();
        graph::unlink(&mut memories, &id_a, &id_b);
    }
    state.persist()
}

#[tauri::command]
fn get_review_heatmap(state: tauri::State<'_, AppState>, days: u32) -> Vec<HeatmapDay> {
    let memories = state.memories.lock().unwrap();
    review_heatmap(&memories, days)
}

#[tauri::command]
fn get_retention_curve(state: tauri::State<'_, AppState>) -> Vec<RetentionPoint> {
    let memories = state.memories.lock().unwrap();
    retention_over_time(&memories)
}

#[tauri::command]
fn get_forgetting_curve(
    state: tauri::State<'_, AppState>,
    id: String,
    days_ahead: u32,
) -> Result<Vec<(u32, f32)>, SynapseError> {
    let memories = state.memories.lock().unwrap();
    let item = memories
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| SynapseError::NotFound(id.clone()))?;
    Ok(forgetting_curve(item, days_ahead))
}

#[tauri::command]
fn get_hardest_items(state: tauri::State<'_, AppState>, limit: usize) -> Vec<MemoryItem> {
    let memories = state.memories.lock().unwrap();
    hardest_items(&memories, limit)
}

#[tauri::command]
fn get_gamification(state: tauri::State<'_, AppState>) -> GamificationSummary {
    let memories = state.memories.lock().unwrap();
    let stats = compute_stats(&memories);
    compute_gamification(&memories, &stats)
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

            let window_state_store = WindowStateStore::new(data_dir.join("window_state.json"));
            let window_state = window_state_store.load().unwrap_or_default();

            app.manage(AppState {
                memories: Mutex::new(memories),
                settings: Mutex::new(settings),
                store,
                settings_store,
                scheduler: Sm2Scheduler,
                backup_dir: data_dir.join("backups"),
            });

            // Restore last known window geometry, then save it again on close so
            // it persists across launches. Position is skipped on first run
            // (None) so the OS picks a sane default placement instead of (0, 0).
            let window = app.get_window("main").expect("main window must exist");
            let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: window_state.width,
                height: window_state.height,
            }));
            if let (Some(x), Some(y)) = (window_state.x, window_state.y) {
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
            }

            let closing_window = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    if let (Ok(size), Ok(position)) = (closing_window.outer_size(), closing_window.outer_position()) {
                        let state = WindowState {
                            x: Some(position.x),
                            y: Some(position.y),
                            width: size.width,
                            height: size.height,
                        };
                        let _ = window_state_store.save(&state);
                    }
                }
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
            import_memories,
            get_knowledge_graph,
            link_memories,
            unlink_memories,
            get_review_heatmap,
            get_retention_curve,
            get_forgetting_curve,
            get_hardest_items,
            get_gamification,
            create_manual_backup,
            list_backups,
            restore_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
