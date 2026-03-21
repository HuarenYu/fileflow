use crate::db::store::{FileChunkRecord, FileStore};
use crate::embedder::Embedder;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default, Deserialize)]
pub struct SearchFilters {
    pub category: Option<String>,
    pub extension: Option<String>,
    pub min_size: Option<i64>,
    pub max_size: Option<i64>,
    pub after_ms: Option<i64>,
    pub before_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub file_id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub modified_at: i64,
    pub category: String,
    pub score: f32,
    pub thumbnail_path: Option<String>,
}

pub struct Searcher {
    store: Arc<FileStore>,
    embedder: Arc<Embedder>,
}

impl Searcher {
    pub fn new(store: Arc<FileStore>, embedder: Arc<Embedder>) -> Self {
        Self { store, embedder }
    }

    pub async fn search(&self, query: &str, filters: SearchFilters) -> Result<Vec<SearchResult>> {
        let query_vec = self.embedder.embed(&[query])?;
        let raw = self
            .store
            .vector_search(&query_vec[0], 50, &filters)
            .await?;

        // multi-chunk aggregation: keep best score per file_id
        let mut best: HashMap<String, (f32, FileChunkRecord)> = HashMap::new();
        for (score, chunk) in raw {
            best.entry(chunk.file_id.clone())
                .and_modify(|(s, _)| {
                    if score > *s {
                        *s = score;
                    }
                })
                .or_insert((score, chunk));
        }

        let mut results: Vec<SearchResult> = best
            .into_values()
            .map(|(score, c)| SearchResult {
                file_id: c.file_id,
                path: c.path,
                name: c.name,
                extension: c.extension,
                size: c.size,
                modified_at: c.modified_at,
                category: c.category,
                score,
                thumbnail_path: c.thumbnail_path,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(results)
    }
}
