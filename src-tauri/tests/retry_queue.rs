// src-tauri/tests/retry_queue.rs
use fileflow_lib::db::retry_queue::RetryQueue;
use tempfile::tempdir;

#[test]
fn push_and_drain_returns_paths_in_order() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("retry.db");
    let q = RetryQueue::new(db_path.to_str().unwrap()).unwrap();

    q.push("/a/file1.txt", "extraction failed").unwrap();
    q.push("/b/file2.txt", "embed failed").unwrap();
    q.push("/c/file3.txt", "write failed").unwrap();

    let paths = q.drain().unwrap();
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0], "/a/file1.txt");
    assert_eq!(paths[1], "/b/file2.txt");
    assert_eq!(paths[2], "/c/file3.txt");
}

#[test]
fn drain_clears_queue() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("retry.db");
    let q = RetryQueue::new(db_path.to_str().unwrap()).unwrap();

    q.push("/some/file.txt", "error").unwrap();
    let first = q.drain().unwrap();
    assert_eq!(first.len(), 1);

    // Second drain should be empty
    let second = q.drain().unwrap();
    assert!(second.is_empty(), "drain should clear the queue");
}

#[test]
fn empty_queue_drain_returns_empty_vec() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("retry.db");
    let q = RetryQueue::new(db_path.to_str().unwrap()).unwrap();

    let paths = q.drain().unwrap();
    assert!(paths.is_empty());
}
