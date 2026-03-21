use crate::db::schema::file_chunks_schema;
use anyhow::Result;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int32Array, Int64Array, RecordBatch,
    RecordBatchIterator, StringArray, TimestampMillisecondArray,
};
use futures::TryStreamExt;
use lancedb::{connect, query::{ExecutableQuery, QueryBase}, Connection, Table};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunkRecord {
    pub id: String,
    pub file_id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub modified_at: i64, // unix ms
    pub category: String,
    pub user_category: Option<String>,
    pub chunk_index: i32,
    pub content_text: String,
    pub vector: Vec<f32>, // 384 dims
    pub thumbnail_path: Option<String>,
    pub indexed_at: i64, // unix ms
    pub deleted_at: Option<i64>,
}

pub struct FileStore {
    conn: Connection,
}

impl FileStore {
    pub async fn new(db_path: &str) -> Result<Self> {
        let conn = connect(db_path).execute().await?;
        // create table if not exists
        let schema = Arc::new(file_chunks_schema());
        let _ = conn
            .create_empty_table("file_chunks", schema)
            .execute()
            .await; // ignore AlreadyExists error
        Ok(Self { conn })
    }

    async fn table(&self) -> Result<Table> {
        Ok(self.conn.open_table("file_chunks").execute().await?)
    }

    pub async fn insert_chunks(&self, chunks: Vec<FileChunkRecord>) -> Result<()> {
        let tbl = self.table().await?;
        let batch = chunks_to_record_batch(chunks)?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
        tbl.add(reader).execute().await?;
        Ok(())
    }

    pub async fn list_by_file_id(&self, file_id: &str) -> Result<Vec<FileChunkRecord>> {
        let tbl = self.table().await?;
        let results = tbl
            .query()
            .only_if(format!("file_id = '{file_id}' AND deleted_at IS NULL"))
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        record_batches_to_chunks(results)
    }

    pub async fn list_by_category(&self, category: Option<&str>) -> Result<Vec<FileChunkRecord>> {
        let tbl = self.table().await?;
        let filter = match category {
            Some(cat) => format!("category = '{cat}' AND deleted_at IS NULL AND chunk_index = 0"),
            None => "deleted_at IS NULL AND chunk_index = 0".to_string(),
        };
        let results = tbl
            .query()
            .only_if(filter)
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        record_batches_to_chunks(results)
    }

    pub async fn soft_delete_by_path(&self, path: &str) -> Result<()> {
        let tbl = self.table().await?;
        let now = chrono::Utc::now().timestamp_millis();
        tbl.update()
            .only_if(format!("path = '{path}'"))
            .column("deleted_at", now.to_string())
            .execute()
            .await?;
        Ok(())
    }

    pub async fn update_user_category(&self, file_id: &str, category: &str) -> Result<()> {
        let tbl = self.table().await?;
        tbl.update()
            .only_if(format!("file_id = '{file_id}'"))
            .column("user_category", format!("'{category}'"))
            .execute()
            .await?;
        Ok(())
    }

    pub async fn vector_search(
        &self,
        query: &[f32],
        limit: usize,
        filters: &crate::search::SearchFilters,
    ) -> Result<Vec<(f32, FileChunkRecord)>> {
        let tbl = self.table().await?;
        // build filter string
        let mut conditions = vec!["deleted_at IS NULL".to_string()];
        if let Some(cat) = &filters.category {
            conditions.push(format!("category = '{cat}'"));
        }
        if let Some(ext) = &filters.extension {
            conditions.push(format!("extension = '{ext}'"));
        }
        if let Some(min) = filters.min_size {
            conditions.push(format!("size >= {min}"));
        }
        if let Some(max) = filters.max_size {
            conditions.push(format!("size <= {max}"));
        }
        if let Some(after) = filters.after_ms {
            conditions.push(format!("modified_at >= {after}"));
        }
        if let Some(before) = filters.before_ms {
            conditions.push(format!("modified_at <= {before}"));
        }
        let q = tbl
            .query()
            .nearest_to(query.to_vec())?
            .limit(limit)
            .only_if(conditions.join(" AND "));
        let batches = q.execute().await?.try_collect::<Vec<_>>().await?;
        // _distance is appended as last column by LanceDB
        let scores: Vec<f32> = batches
            .iter()
            .flat_map(|b| {
                let col = b.column_by_name("_distance").unwrap();
                let arr = col
                    .as_any()
                    .downcast_ref::<arrow_array::Float32Array>()
                    .unwrap();
                (0..arr.len()).map(|i| 1.0 - arr.value(i))
            })
            .collect();
        let chunks = record_batches_to_chunks(batches)?;
        Ok(scores.into_iter().zip(chunks).collect())
    }
}

fn chunks_to_record_batch(chunks: Vec<FileChunkRecord>) -> Result<RecordBatch> {
    let schema = Arc::new(crate::db::schema::file_chunks_schema());
    let ids: StringArray = chunks.iter().map(|c| Some(c.id.as_str())).collect();
    let file_ids: StringArray = chunks.iter().map(|c| Some(c.file_id.as_str())).collect();
    let paths: StringArray = chunks.iter().map(|c| Some(c.path.as_str())).collect();
    let names: StringArray = chunks.iter().map(|c| Some(c.name.as_str())).collect();
    let exts: StringArray = chunks.iter().map(|c| Some(c.extension.as_str())).collect();
    let sizes: Int64Array = chunks.iter().map(|c| Some(c.size)).collect();
    let modified_ats: TimestampMillisecondArray =
        chunks.iter().map(|c| Some(c.modified_at)).collect();
    let categories: StringArray = chunks.iter().map(|c| Some(c.category.as_str())).collect();
    let user_cats: StringArray = chunks.iter().map(|c| c.user_category.as_deref()).collect();
    let chunk_idxs: Int32Array = chunks.iter().map(|c| Some(c.chunk_index)).collect();
    let texts: StringArray = chunks.iter().map(|c| Some(c.content_text.as_str())).collect();
    // vector: FixedSizeList<Float32, 384>
    let flat_vecs: Float32Array = chunks.iter().flat_map(|c| c.vector.iter().copied()).collect();
    let vectors = FixedSizeListArray::try_new(
        Arc::new(arrow_schema::Field::new(
            "item",
            arrow_schema::DataType::Float32,
            true,
        )),
        384,
        Arc::new(flat_vecs),
        None,
    )?;
    let thumbs: StringArray = chunks
        .iter()
        .map(|c| c.thumbnail_path.as_deref())
        .collect();
    let indexed_ats: TimestampMillisecondArray =
        chunks.iter().map(|c| Some(c.indexed_at)).collect();
    let deleted_ats: TimestampMillisecondArray =
        chunks.iter().map(|c| c.deleted_at).collect();

    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(file_ids),
            Arc::new(paths),
            Arc::new(names),
            Arc::new(exts),
            Arc::new(sizes),
            Arc::new(modified_ats),
            Arc::new(categories),
            Arc::new(user_cats),
            Arc::new(chunk_idxs),
            Arc::new(texts),
            Arc::new(vectors),
            Arc::new(thumbs),
            Arc::new(indexed_ats),
            Arc::new(deleted_ats),
        ],
    )?)
}

fn record_batches_to_chunks(batches: Vec<RecordBatch>) -> Result<Vec<FileChunkRecord>> {
    let mut out = vec![];
    for batch in &batches {
        let n = batch.num_rows();
        let ids = batch
            .column_by_name("id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let file_ids = batch
            .column_by_name("file_id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let paths = batch
            .column_by_name("path")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let names = batch
            .column_by_name("name")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let exts = batch
            .column_by_name("extension")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let sizes = batch
            .column_by_name("size")
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        let modified_ats = batch
            .column_by_name("modified_at")
            .unwrap()
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .unwrap();
        let categories = batch
            .column_by_name("category")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let user_cats = batch
            .column_by_name("user_category")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let chunk_idxs = batch
            .column_by_name("chunk_index")
            .unwrap()
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        let texts = batch
            .column_by_name("content_text")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let thumbs = batch
            .column_by_name("thumbnail_path")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let indexed_ats = batch
            .column_by_name("indexed_at")
            .unwrap()
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .unwrap();
        let deleted_ats = batch
            .column_by_name("deleted_at")
            .unwrap()
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .unwrap();
        for i in 0..n {
            out.push(FileChunkRecord {
                id: ids.value(i).to_string(),
                file_id: file_ids.value(i).to_string(),
                path: paths.value(i).to_string(),
                name: names.value(i).to_string(),
                extension: exts.value(i).to_string(),
                size: sizes.value(i),
                modified_at: modified_ats.value(i),
                category: categories.value(i).to_string(),
                user_category: if user_cats.is_null(i) {
                    None
                } else {
                    Some(user_cats.value(i).to_string())
                },
                chunk_index: chunk_idxs.value(i),
                content_text: texts.value(i).to_string(),
                vector: vec![], // not deserialized by default (large)
                thumbnail_path: if thumbs.is_null(i) {
                    None
                } else {
                    Some(thumbs.value(i).to_string())
                },
                indexed_at: indexed_ats.value(i),
                deleted_at: if deleted_ats.is_null(i) {
                    None
                } else {
                    Some(deleted_ats.value(i))
                },
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_insert_and_query_chunk() {
        let dir = tempdir().unwrap();
        let store = FileStore::new(dir.path().to_str().unwrap()).await.unwrap();
        let chunk = FileChunkRecord {
            id: "abc_0".to_string(),
            file_id: "abc".to_string(),
            path: "/test/file.pdf".to_string(),
            name: "file.pdf".to_string(),
            extension: "pdf".to_string(),
            size: 1024,
            modified_at: 0,
            category: "document".to_string(),
            user_category: None,
            chunk_index: 0,
            content_text: "hello world".to_string(),
            vector: vec![0.0f32; 384],
            thumbnail_path: None,
            indexed_at: 0,
            deleted_at: None,
        };
        store.insert_chunks(vec![chunk]).await.unwrap();
        let results = store.list_by_file_id("abc").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "file.pdf");
    }
}
