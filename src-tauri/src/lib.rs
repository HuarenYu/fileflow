pub mod chunker;
pub mod classifier;
pub mod commands;
pub mod db;
pub mod embedder;
pub mod extractor;
pub mod indexer;
pub mod preview;
pub mod search;

use commands::index_cmd::IndexStatus;
use db::{retry_queue::RetryQueue, store::FileStore};
use embedder::Embedder;
use indexer::{pipeline::IndexPipeline, watcher::FileWatcher};
use search::Searcher;
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tauri::Manager;
use tokio::sync::Mutex;

pub struct AppState {
    pub store: Arc<FileStore>,
    pub searcher: Arc<Searcher>,
    pub pipeline: Arc<IndexPipeline>,
    pub cache_dir: PathBuf,
    pub app_dir: PathBuf,                           // NEW
    pub retry_queue: Arc<RetryQueue>,
    pub libreoffice_available: bool,
    pub watched_dirs: Mutex<Vec<PathBuf>>,          // NEW
    /// Active file watcher (kept alive)
    _watcher: Mutex<Option<FileWatcher>>,
}

impl AppState {
    pub async fn add_directory(&self, path: &str) -> anyhow::Result<()> {
        // 1. Walk existing files and index them in the background (existing behaviour)
        let dir = PathBuf::from(path);
        let pipeline = self.pipeline.clone();
        let dir_clone = dir.clone();
        tokio::spawn(async move {
            if let Ok(entries) = walkdir_files(&dir_clone) {
                for file_path in entries {
                    let _ = pipeline.index_file(&file_path).await;
                }
            }
        });

        // 2. Append to watched_dirs (skip if already present); capture snapshot
        let dirs_snapshot = {
            let mut dirs = self.watched_dirs.lock().await;
            if !dirs.contains(&dir) {
                dirs.push(dir.clone());
            }
            dirs.clone()
        };

        // 3. Persist using the snapshot (lock already released)
        save_watched_dirs(&self.app_dir, &dirs_snapshot);

        // 4. Rebuild watcher (after releasing the lock)
        self.rebuild_watcher().await;

        Ok(())
    }

    pub async fn remove_directory(&self, path: &str) -> anyhow::Result<()> {
        // 1. Soft-delete all records under this path prefix
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };
        self.store.soft_delete_by_prefix(&prefix).await?;

        // 2. Remove from watched_dirs; capture snapshot
        let dirs_snapshot = {
            let dir = PathBuf::from(path);
            let mut dirs = self.watched_dirs.lock().await;
            dirs.retain(|d| d != &dir);
            dirs.clone()
        };

        // 3. Persist (lock already released)
        save_watched_dirs(&self.app_dir, &dirs_snapshot);

        // 4. Rebuild watcher (lock already released)
        self.rebuild_watcher().await;

        Ok(())
    }

    pub async fn get_index_status(&self) -> IndexStatus {
        IndexStatus {
            total: self.pipeline.total.load(Ordering::Relaxed),
            indexed: self.pipeline.indexed.load(Ordering::Relaxed),
            failed: self.pipeline.failed.load(Ordering::Relaxed),
            is_running: false,
        }
    }

    async fn rebuild_watcher(&self) {
        let dirs = self.watched_dirs.lock().await.clone();
        let existing_dirs: Vec<PathBuf> = dirs.into_iter().filter(|d| d.exists()).collect();
        let watcher = if existing_dirs.is_empty() {
            None
        } else {
            FileWatcher::new(existing_dirs, self.pipeline.clone())
                .map_err(|e| tracing::error!("Failed to start file watcher: {e}"))
                .ok()
        };
        *self._watcher.lock().await = watcher;
    }
}

fn walkdir_files(dir: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in walkdir::WalkDir::new(dir).follow_links(true) {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(files)
}

fn load_watched_dirs(app_dir: &std::path::Path) -> Vec<PathBuf> {
    let path = app_dir.join("watched_dirs.json");
    let Ok(data) = std::fs::read_to_string(&path) else {
        return vec![];
    };
    let Ok(strings) = serde_json::from_str::<Vec<String>>(&data) else {
        tracing::warn!("Failed to parse watched_dirs.json, starting fresh");
        return vec![];
    };
    strings.into_iter().map(PathBuf::from).collect()
}

fn save_watched_dirs(app_dir: &std::path::Path, dirs: &[PathBuf]) {
    let path = app_dir.join("watched_dirs.json");
    let strings: Vec<String> = dirs
        .iter()
        .map(|d| d.to_string_lossy().into_owned())
        .collect();
    let Ok(data) = serde_json::to_string(&strings) else {
        tracing::error!("Failed to serialize watched_dirs");
        return;
    };
    if let Err(e) = std::fs::write(&path, data) {
        tracing::error!("Failed to save watched_dirs.json: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("fileflow");
            std::fs::create_dir_all(&app_dir)?;

            let handle = app.handle().clone();
            let rt = tokio::runtime::Runtime::new()?;
            let state = rt.block_on(async {
                let store = Arc::new(
                    FileStore::new(app_dir.join("lance").to_str().unwrap())
                        .await
                        .expect("Failed to open LanceDB"),
                );
                let retry_queue = Arc::new(
                    RetryQueue::new(app_dir.join("retry.db").to_str().unwrap())
                        .expect("Failed to open retry queue"),
                );
                let embedder = Arc::new(Embedder::new().expect("Failed to init embedder"));
                let pipeline = Arc::new(IndexPipeline::new(
                    store.clone(),
                    retry_queue.clone(),
                    embedder.clone(),
                    handle,
                ));
                let searcher = Arc::new(Searcher::new(store.clone(), embedder));
                let lo = preview::libreoffice_available();

                // Drain retry queue from previous session
                let pending = retry_queue.drain().unwrap_or_default();
                let p = pipeline.clone();
                tokio::spawn(async move {
                    for path in pending {
                        let _ = p.index_file(std::path::Path::new(&path)).await;
                    }
                });

                AppState {
                    store,
                    searcher,
                    pipeline,
                    cache_dir: app_dir.join("cache"),
                    app_dir: app_dir.clone(),
                    retry_queue,
                    libreoffice_available: lo,
                    watched_dirs: Mutex::new(vec![]),
                    _watcher: Mutex::new(None),
                }
            });

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::index_cmd::add_directory,
            commands::index_cmd::remove_directory,
            commands::index_cmd::get_index_status,
            commands::search_cmd::search_files,
            commands::files_cmd::list_files,
            commands::files_cmd::update_category,
            commands::files_cmd::open_file,
            commands::preview_cmd::get_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_watched_dirs_round_trip() {
        let dir = tempdir().unwrap();
        let app_dir = dir.path();

        let paths = vec![
            PathBuf::from("/home/user/Documents"),
            PathBuf::from("/home/user/Bob's Files"),  // single-quote stress test
        ];

        save_watched_dirs(app_dir, &paths);
        let loaded = load_watched_dirs(app_dir);
        assert_eq!(loaded, paths);
    }

    #[test]
    fn test_load_watched_dirs_missing_file() {
        let dir = tempdir().unwrap();
        // No file written — should return empty vec, not panic
        let result = load_watched_dirs(dir.path());
        assert!(result.is_empty());
    }
}
