use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct IndexStatus {
    pub total: u64,
    pub indexed: u64,
    pub failed: u64,
    pub is_running: bool,
}

#[tauri::command]
pub async fn add_directory(path: String, state: State<'_, AppState>) -> Result<(), String> {
    state.add_directory(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_directory(path: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .remove_directory(&path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_index_status(state: State<'_, AppState>) -> Result<IndexStatus, String> {
    Ok(state.get_index_status().await)
}
