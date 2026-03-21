use crate::{db::store::FileChunkRecord, AppState};
use tauri::State;

#[tauri::command]
pub async fn list_files(
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<FileChunkRecord>, String> {
    state
        .store
        .list_by_category(category.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_category(
    file_id: String,
    category: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .store
        .update_user_category(&file_id, &category)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}
