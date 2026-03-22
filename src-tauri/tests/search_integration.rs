// src-tauri/tests/search_integration.rs
use fileflow_lib::{
    db::store::{FileChunkRecord, FileStore},
    embedder::Embedder,
    search::{SearchFilters, Searcher},
};
use std::sync::Arc;
use tempfile::{tempdir, TempDir};

async fn setup() -> (TempDir, Arc<FileStore>, Arc<Embedder>, Searcher) {
    let dir = tempdir().unwrap();
    let store = Arc::new(FileStore::new(dir.path().to_str().unwrap()).await.unwrap());
    let embedder = Arc::new(Embedder::new().unwrap());
    let searcher = Searcher::new(store.clone(), embedder.clone());
    (dir, store, embedder, searcher)
}

fn make_chunk_with_text(
    id: &str,
    path: &str,
    category: &str,
    text: &str,
    vector: Vec<f32>,
) -> FileChunkRecord {
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
        size: 512,
        modified_at: 0,
        category: category.to_string(),
        user_category: None,
        chunk_index: 0,
        content_text: text.to_string(),
        vector,
        thumbnail_path: None,
        indexed_at: 0,
        deleted_at: None,
    }
}

#[tokio::test]
async fn search_returns_semantically_relevant_file() {
    let (_dir, store, embedder, searcher) = setup().await;

    // Embed "machine learning algorithms" as document content
    let doc_text = "machine learning algorithms neural network";
    let doc_vec = embedder.embed(&[doc_text]).unwrap().remove(0);
    store
        .insert_chunks(vec![make_chunk_with_text(
            "ml-doc", "/docs/ml.txt", "document", doc_text, doc_vec,
        )])
        .await
        .unwrap();

    // Also insert an unrelated document
    let other_text = "quarterly revenue report spreadsheet";
    let other_vec = embedder.embed(&[other_text]).unwrap().remove(0);
    store
        .insert_chunks(vec![make_chunk_with_text(
            "biz-doc", "/docs/report.txt", "document", other_text, other_vec,
        )])
        .await
        .unwrap();

    let results = searcher
        .search("deep learning", SearchFilters::default())
        .await
        .unwrap();

    assert!(!results.is_empty(), "should return at least one result");
    // The ML document should rank higher than the business report
    assert_eq!(results[0].file_id, "ml-doc", "ML document should be top result");
}

#[tokio::test]
async fn search_filter_by_category_excludes_other_categories() {
    let (_dir, store, embedder, searcher) = setup().await;

    let text = "project documentation report";
    let vec_doc = embedder.embed(&[text]).unwrap().remove(0);
    let vec_img = vec_doc.clone();

    store
        .insert_chunks(vec![
            make_chunk_with_text("d1", "/docs/notes.txt", "document", text, vec_doc),
            make_chunk_with_text("i1", "/imgs/photo.png", "image", text, vec_img),
        ])
        .await
        .unwrap();

    let results = searcher
        .search(
            "documentation",
            SearchFilters {
                category: Some("document".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert!(results.iter().all(|r| r.category == "document"),
        "category filter should exclude non-document results");
    assert!(results.iter().any(|r| r.file_id == "d1"));
    assert!(results.iter().all(|r| r.file_id != "i1"));
}

#[tokio::test]
async fn search_aggregates_best_score_per_file() {
    let (_dir, store, embedder, searcher) = setup().await;

    // Same file_id, two chunks with different content
    let text1 = "introduction and overview of the project";
    let text2 = "conclusion and future work section";
    let v1 = embedder.embed(&[text1]).unwrap().remove(0);
    let v2 = embedder.embed(&[text2]).unwrap().remove(0);

    store
        .insert_chunks(vec![
            {
                let mut c = make_chunk_with_text("multi-chunk-0", "/doc.txt", "document", text1, v1);
                c.file_id = "multi-file".to_string();
                c.chunk_index = 0;
                c
            },
            {
                let mut c = make_chunk_with_text("multi-chunk-1", "/doc.txt", "document", text2, v2);
                c.file_id = "multi-file".to_string();
                c.chunk_index = 1;
                c
            },
        ])
        .await
        .unwrap();

    let results = searcher
        .search("project overview", SearchFilters::default())
        .await
        .unwrap();

    // Should only appear once — best chunk wins
    let count = results.iter().filter(|r| r.file_id == "multi-file").count();
    assert_eq!(count, 1, "multi-chunk file should appear exactly once in results");
}
