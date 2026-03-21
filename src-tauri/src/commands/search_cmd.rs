use crate::{
    search::{SearchFilters, SearchResult},
    AppState,
};
use tauri::State;

#[tauri::command]
pub async fn search_files(
    query: String,
    filters: SearchFilters,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    state
        .searcher
        .search(&query, filters)
        .await
        .map_err(|e| e.to_string())
}
