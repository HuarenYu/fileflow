// src-tauri/tests/store_integration.rs
//
// 与 store.rs 内模块测试的关系：
// - 已有测试覆盖：list_by_file_id, soft_delete_by_prefix（基础 CRUD）
// - 这里新增：vector_search, list_by_category, concurrent inserts
use fileflow_lib::db::store::{FileChunkRecord, FileStore};
use std::sync::Arc;
use tempfile::tempdir;

fn make_chunk(id: &str, path: &str, category: &str, vector: Vec<f32>) -> FileChunkRecord {
    FileChunkRecord {
        id: id.to_string(),
        file_id: id.to_string(),
        path: path.to_string(),
        name: std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        extension: std::path::Path::new(path)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        size: 1024,
        modified_at: 0,
        category: category.to_string(),
        user_category: None,
        chunk_index: 0,
        content_text: "test content".to_string(),
        vector,
        thumbnail_path: None,
        indexed_at: 0,
        deleted_at: None,
    }
}

#[tokio::test]
async fn insert_then_list_by_category() {
    let dir = tempdir().unwrap();
    let store = FileStore::new(dir.path().to_str().unwrap()).await.unwrap();

    store
        .insert_chunks(vec![
            make_chunk("doc1", "/a/report.pdf", "document", vec![0.0f32; 384]),
            make_chunk("img1", "/b/photo.jpg", "image", vec![0.1f32; 384]),
            make_chunk("doc2", "/c/notes.txt", "document", vec![0.2f32; 384]),
        ])
        .await
        .unwrap();

    let docs = store.list_by_category(Some("document")).await.unwrap();
    assert_eq!(docs.len(), 2, "should return 2 documents");
    assert!(docs.iter().all(|d| d.category == "document"));

    let images = store.list_by_category(Some("image")).await.unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].id, "img1");

    let all = store.list_by_category(None).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn soft_delete_then_vector_query_excludes_deleted() {
    let dir = tempdir().unwrap();
    let store = FileStore::new(dir.path().to_str().unwrap()).await.unwrap();

    // Two chunks: one will be deleted
    let v1: Vec<f32> = (0..384).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
    let v2: Vec<f32> = (0..384).map(|i| if i == 1 { 1.0 } else { 0.0 }).collect();

    store
        .insert_chunks(vec![
            make_chunk("alive", "/alive.txt", "document", v1.clone()),
            make_chunk("dead", "/dead.txt", "document", v2),
        ])
        .await
        .unwrap();

    store.soft_delete_by_path("/dead.txt").await.unwrap();

    let filters = fileflow_lib::search::SearchFilters::default();
    let results = store.vector_search(&v1, 10, &filters).await.unwrap();

    // "alive" chunk must appear (non-empty results)
    assert!(!results.is_empty(), "vector search should return results");
    // "dead" chunk should not appear in results
    assert!(
        results.iter().all(|(_, c)| c.id != "dead"),
        "soft-deleted chunk should not appear in vector search"
    );
}

#[tokio::test]
async fn concurrent_inserts_all_persisted() {
    let dir = tempdir().unwrap();
    let store = Arc::new(FileStore::new(dir.path().to_str().unwrap()).await.unwrap());

    let mut handles = vec![];
    for i in 0..4 {
        let s = store.clone();
        handles.push(tokio::spawn(async move {
            let chunk = make_chunk(
                &format!("chunk-{i}"),
                &format!("/file-{i}.txt"),
                "document",
                vec![i as f32 / 10.0; 384],
            );
            s.insert_chunks(vec![chunk]).await.unwrap();
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let all = store.list_by_category(None).await.unwrap();
    assert_eq!(all.len(), 4, "all 4 concurrent inserts should be persisted");
}
