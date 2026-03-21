use crate::{
    chunker::chunk_text,
    classifier::classify,
    db::{
        retry_queue::RetryQueue,
        store::{FileChunkRecord, FileStore},
    },
    embedder::Embedder,
    extractor,
};
use anyhow::Result;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Semaphore;

#[derive(Clone, serde::Serialize)]
struct IndexProgressPayload {
    total: u64,
    indexed: u64,
    failed: u64,
    is_running: bool,
}

pub struct IndexPipeline {
    pub store: Arc<FileStore>,
    pub retry_queue: Arc<RetryQueue>,
    pub embedder: Arc<Embedder>,
    semaphore: Arc<Semaphore>,
    pub total: Arc<AtomicU64>,
    pub indexed: Arc<AtomicU64>,
    pub failed: Arc<AtomicU64>,
    app_handle: AppHandle,
}

impl IndexPipeline {
    pub fn new(
        store: Arc<FileStore>,
        retry_queue: Arc<RetryQueue>,
        embedder: Arc<Embedder>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            store,
            retry_queue,
            embedder,
            semaphore: Arc::new(Semaphore::new(4)),
            total: Arc::new(AtomicU64::new(0)),
            indexed: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
            app_handle,
        }
    }

    pub async fn index_file(&self, path: &Path) -> Result<()> {
        let _permit = self.semaphore.acquire().await?;
        let file_id = file_id(path);
        let meta = std::fs::metadata(path)?;
        let size = meta.len() as i64;
        let modified_at = meta
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
            })
            .unwrap_or(0);
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let category = classify(path).to_string();

        let text = extractor::extract_text(path).unwrap_or_default();
        let chunks = if text.is_empty() {
            vec![name.clone()] // index by filename only
        } else {
            chunk_text(&text, 400, 50)
        };

        let texts_ref: Vec<&str> = chunks.iter().map(|s| s.as_str()).collect();
        let vectors = self.embedder.embed(&texts_ref).unwrap_or_else(|_| {
            chunks.iter().map(|_| vec![0.0f32; 384]).collect()
        });

        let now = Utc::now().timestamp_millis();
        let records: Vec<FileChunkRecord> = chunks
            .into_iter()
            .enumerate()
            .zip(vectors)
            .map(|((i, text), vector)| FileChunkRecord {
                id: format!("{}_{}", file_id, i),
                file_id: file_id.clone(),
                path: path.to_string_lossy().to_string(),
                name: name.clone(),
                extension: extension.clone(),
                size,
                modified_at,
                category: category.clone(),
                user_category: None,
                chunk_index: i as i32,
                content_text: text.chars().take(2000).collect(),
                vector,
                thumbnail_path: None,
                indexed_at: now,
                deleted_at: None,
            })
            .collect();

        self.total.fetch_add(1, Ordering::Relaxed);
        if let Err(e) = self.store.insert_chunks(records).await {
            self.failed.fetch_add(1, Ordering::Relaxed);
            self.retry_queue
                .push(&path.to_string_lossy(), &e.to_string())?;
        } else {
            let n = self.indexed.fetch_add(1, Ordering::Relaxed) + 1;
            self.app_handle
                .emit(
                    "index_progress",
                    IndexProgressPayload {
                        total: self.total.load(Ordering::Relaxed),
                        indexed: n,
                        failed: self.failed.load(Ordering::Relaxed),
                        is_running: true,
                    },
                )
                .ok();
        }
        Ok(())
    }

    pub async fn delete_file(&self, path: &Path) -> Result<()> {
        self.store
            .soft_delete_by_path(&path.to_string_lossy())
            .await
    }
}

pub fn file_id(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}
