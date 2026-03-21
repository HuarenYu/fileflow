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
use tokio::sync::Mutex;

pub struct AppState {
    pub store: Arc<FileStore>,
    pub searcher: Arc<Searcher>,
    pub pipeline: Arc<IndexPipeline>,
    pub cache_dir: PathBuf,
    pub retry_queue: Arc<RetryQueue>,
    pub libreoffice_available: bool,
    /// Active file watcher (kept alive)
    _watcher: Mutex<Option<FileWatcher>>,
}

impl AppState {
    pub async fn add_directory(&self, path: &str) -> anyhow::Result<()> {
        // Walk existing files and index them
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
        Ok(())
    }

    pub async fn remove_directory(&self, _path: &str) -> anyhow::Result<()> {
        // Soft-delete all entries under this path
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
                    retry_queue,
                    libreoffice_available: lo,
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
