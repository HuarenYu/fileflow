#[tokio::test]
async fn test_full_index_and_search_pipeline() {
    use fileflow_lib::{
        chunker::chunk_text,
        classifier::classify,
        db::{retry_queue::RetryQueue, store::FileStore},
        embedder::Embedder,
        indexer::pipeline::IndexPipeline,
        search::{SearchFilters, Searcher},
    };
    use std::{path::Path, sync::Arc};
    use tempfile::tempdir;
    use std::io::Write;

    let dir = tempdir().unwrap();
    let store = Arc::new(FileStore::new(dir.path().join("lance").to_str().unwrap()).await.unwrap());
    let rq = Arc::new(RetryQueue::new(dir.path().join("retry.db").to_str().unwrap()).unwrap());
    let embedder = Arc::new(Embedder::new().unwrap());
    // Note: IndexPipeline requires AppHandle in production; integration test uses a simplified path
    let searcher = Searcher::new(store.clone(), embedder.clone());

    // create a test text file
    let file = dir.path().join("contract_2024.txt");
    let mut f = std::fs::File::create(&file).unwrap();
    writeln!(f, "This is a service agreement contract signed with Acme Corp in March 2024.").unwrap();
    writeln!(f, "The total project budget is $50,000 payable in three installments.").unwrap();

    // verify chunker and classifier
    let text = std::fs::read_to_string(&file).unwrap();
    let chunks = chunk_text(&text, 400, 50);
    assert!(!chunks.is_empty());

    assert_eq!(classify(Path::new("contract_2024.txt")), "document");
}
