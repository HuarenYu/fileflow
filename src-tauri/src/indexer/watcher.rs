use crate::indexer::pipeline::IndexPipeline;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new(dirs: Vec<PathBuf>, pipeline: Arc<IndexPipeline>) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel::<notify::Result<Event>>(100);

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.blocking_send(res);
        })?;

        for dir in &dirs {
            watcher.watch(dir, RecursiveMode::Recursive)?;
        }

        tokio::spawn(async move {
            while let Some(Ok(event)) = rx.recv().await {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in &event.paths {
                            if path.is_file() {
                                let _ = pipeline.index_file(path).await;
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in &event.paths {
                            let _ = pipeline.delete_file(path).await;
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(Self { _watcher: watcher })
    }
}
