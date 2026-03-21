use crate::{preview, AppState};
use tauri::State;

#[tauri::command]
pub async fn get_preview(
    path: String,
    state: State<'_, AppState>,
) -> Result<preview::PreviewData, String> {
    let cache_dir = state.cache_dir.join("office_preview");
    preview::preview(std::path::Path::new(&path), &cache_dir).map_err(|e| e.to_string())
}
