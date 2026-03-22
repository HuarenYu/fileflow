# File Watcher Activation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up the existing `FileWatcher` so that adding/removing directories activates real-time file monitoring, with directory lists persisted across app restarts.

**Architecture:** On every `add_directory` / `remove_directory` call, update an in-memory + JSON-persisted directory list and rebuild the `FileWatcher` from scratch (Option A: rebuild-on-change). Startup loads the persisted list and resumes watching.

**Tech Stack:** Rust, Tauri 2.x, `notify` crate (FileWatcher already implemented), `serde_json` (already a transitive dep via `serde`), LanceDB 0.14 (`.only_if()` filter API), `tokio::sync::Mutex`

---

## Environment Setup (Required before any `cargo` command)

```bash
export PATH="/tmp/local-sys/usr/bin:/home/huarenyu/.cargo/bin:$PATH"
export LD_LIBRARY_PATH="/tmp/local-sys/usr/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu"
export LIBRARY_PATH="/tmp/local-sys/usr/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu"
export PKG_CONFIG="/tmp/local-sys/usr/bin/pkgconf"
export PKG_CONFIG_PATH="/tmp/local-sys/usr/lib/x86_64-linux-gnu/pkgconfig:/tmp/local-sys/usr/share/pkgconfig:/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/share/pkgconfig"
export PROTOC="/tmp/local-sys/usr/bin/protoc"
export PROTOC_INCLUDE="/tmp/local-sys/usr/include"
```

Run tests from the repo root: `cargo test --manifest-path src-tauri/Cargo.toml`

---

## File Map

| File | Action | What changes |
|------|--------|--------------|
| `src-tauri/src/db/store.rs` | Modify | Add `soft_delete_by_prefix` method + test |
| `src-tauri/src/lib.rs` | Modify | Add `watched_dirs`/`app_dir` fields; add `load_watched_dirs`, `save_watched_dirs`, `rebuild_watcher`; update `add_directory`, `remove_directory`; wire startup |

No other files change.

---

## Task 1: Add `soft_delete_by_prefix` to `FileStore`

**Files:**
- Modify: `src-tauri/src/db/store.rs`

- [ ] **Step 1: Write the failing test**

Add this test to the `#[cfg(test)]` block at the bottom of `src-tauri/src/db/store.rs`, after the existing `test_insert_and_query_chunk` test:

```rust
#[tokio::test]
async fn test_soft_delete_by_prefix() {
    let dir = tempdir().unwrap();
    let store = FileStore::new(dir.path().to_str().unwrap()).await.unwrap();

    // Insert two chunks under /watched/dir/ and one outside
    let make_chunk = |id: &str, path: &str| FileChunkRecord {
        id: id.to_string(),
        file_id: id.to_string(),
        path: path.to_string(),
        name: "f.txt".to_string(),
        extension: "txt".to_string(),
        size: 1,
        modified_at: 0,
        category: "document".to_string(),
        user_category: None,
        chunk_index: 0,
        content_text: "x".to_string(),
        vector: vec![0.0f32; 384],
        thumbnail_path: None,
        indexed_at: 0,
        deleted_at: None,
    };

    store.insert_chunks(vec![
        make_chunk("a", "/watched/dir/file1.txt"),
        make_chunk("b", "/watched/dir/sub/file2.txt"),
        make_chunk("c", "/other/file3.txt"),
    ]).await.unwrap();

    store.soft_delete_by_prefix("/watched/dir/").await.unwrap();

    // a and b should be soft-deleted (not returned by list_by_file_id)
    assert!(store.list_by_file_id("a").await.unwrap().is_empty());
    assert!(store.list_by_file_id("b").await.unwrap().is_empty());
    // c should still be visible
    assert_eq!(store.list_by_file_id("c").await.unwrap().len(), 1);
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml \
  db::store::tests::test_soft_delete_by_prefix 2>&1 | tail -20
```

Expected: compile error `no method named soft_delete_by_prefix`

- [ ] **Step 3: Implement `soft_delete_by_prefix`**

Add this method to `FileStore` in `src-tauri/src/db/store.rs`, after `soft_delete_by_path` (around line 97):

```rust
pub async fn soft_delete_by_prefix(&self, prefix: &str) -> Result<()> {
    let tbl = self.table().await?;
    let now = chrono::Utc::now().timestamp_millis();
    let safe_prefix = prefix.replace('\'', "''");
    tbl.update()
        .only_if(format!("path LIKE '{safe_prefix}%' AND deleted_at IS NULL"))
        .column("deleted_at", now.to_string())
        .execute()
        .await?;
    Ok(())
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml \
  db::store::tests::test_soft_delete_by_prefix 2>&1 | tail -10
```

Expected: `test db::store::tests::test_soft_delete_by_prefix ... ok`

- [ ] **Step 5: Commit**

```bash
cd /home/huarenyu/work/work-auto/fileflow
git add src-tauri/src/db/store.rs
git commit -m "feat: add soft_delete_by_prefix to FileStore"
```

---

## Task 2: Add persistence helpers (`load_watched_dirs` / `save_watched_dirs`)

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add this test module at the bottom of `src-tauri/src/lib.rs`:

```rust
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
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml \
  tests::test_watched_dirs_round_trip 2>&1 | tail -20
```

Expected: compile error (functions don't exist yet)

- [ ] **Step 3: Implement `load_watched_dirs` and `save_watched_dirs`**

Add these two free functions to `src-tauri/src/lib.rs` (after the `walkdir_files` function, before `run()`):

```rust
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
```

Note: `serde_json` is already a dependency (via `serde` + existing usage in `commands/`). No `Cargo.toml` change and no `use` import needed — the helper functions call `serde_json::from_str` and `serde_json::to_string` using the full crate path.

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml \
  tests::test_watched_dirs 2>&1 | tail -10
```

Expected:
```
test tests::test_watched_dirs_round_trip ... ok
test tests::test_load_watched_dirs_missing_file ... ok
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add watched_dirs JSON persistence helpers"
```

---

## Task 3: Update `AppState` — add fields and `rebuild_watcher`

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Note on TDD:** This task has no failing-test-first step. `rebuild_watcher` calls `FileWatcher::new`, which starts a real OS file-system watcher and requires actual directories — mocking it would need significant infrastructure that is out of scope. Instead, correctness is validated by a full `cargo build` check here, and end-to-end by the manual test in Task 7.

- [ ] **Step 1: Add `watched_dirs` and `app_dir` fields to `AppState`**

In `src-tauri/src/lib.rs`, update the `AppState` struct:

```rust
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
```

- [ ] **Step 2: Add `rebuild_watcher` method to `AppState`**

Add this `impl AppState` method (after the existing `get_index_status` method):

```rust
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
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error" | head -20
```

Expected: no `error` lines (warnings OK)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add watched_dirs/app_dir fields and rebuild_watcher to AppState"
```

---

## Task 4: Update `add_directory` to persist and watch

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace `add_directory` implementation**

Replace the current `add_directory` method body in `src-tauri/src/lib.rs`:

```rust
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

    // 2. Append to watched_dirs (skip if already present)
    {
        let mut dirs = self.watched_dirs.lock().await;
        if !dirs.contains(&dir) {
            dirs.push(dir.clone());
        }
    }

    // 3. Persist (after releasing the lock)
    save_watched_dirs(&self.app_dir, &self.watched_dirs.lock().await.clone());

    // 4. Rebuild watcher (after releasing the lock)
    self.rebuild_watcher().await;

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error" | head -20
```

Expected: no `error` lines

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add_directory now persists dir and activates file watcher"
```

---

## Task 5: Implement `remove_directory`

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace `remove_directory` stub**

Replace the current `remove_directory` stub in `src-tauri/src/lib.rs`:

```rust
pub async fn remove_directory(&self, path: &str) -> anyhow::Result<()> {
    // 1. Soft-delete all records under this path prefix
    let prefix = if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    };
    self.store.soft_delete_by_prefix(&prefix).await?;

    // 2. Remove from watched_dirs
    let dir = PathBuf::from(path);
    {
        let mut dirs = self.watched_dirs.lock().await;
        dirs.retain(|d| d != &dir);
    }

    // 3. Persist (after releasing the lock)
    save_watched_dirs(&self.app_dir, &self.watched_dirs.lock().await.clone());

    // 4. Rebuild watcher (after releasing the lock)
    self.rebuild_watcher().await;

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error" | head -20
```

Expected: no `error` lines

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: implement remove_directory (soft-delete + unwatch + persist)"
```

---

## Task 6: Wire startup — load persisted dirs and resume watching

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update `AppState` construction in `run()`**

In the `run()` function's `rt.block_on(async { ... })` block, update the `AppState { ... }` literal and the code that follows it:

**Before (current code):**
```rust
AppState {
    store,
    searcher,
    pipeline,
    cache_dir: app_dir.join("cache"),
    retry_queue,
    libreoffice_available: lo,
    _watcher: Mutex::new(None),
}
```

**After:**
```rust
let watched_dirs = load_watched_dirs(&app_dir);
AppState {
    store,
    searcher,
    pipeline,
    cache_dir: app_dir.join("cache"),
    app_dir: app_dir.clone(),
    retry_queue,
    libreoffice_available: lo,
    watched_dirs: Mutex::new(watched_dirs),
    _watcher: Mutex::new(None),
}
```

- [ ] **Step 2: Resume watching after `app.manage(state)`**

After `app.manage(state);` and before `Ok(())`, add:

```rust
// Resume watching for persisted directories
// Use app.handle().clone() to get a 'static-capable handle — tokio::spawn requires 'static.
// tauri::State<'_, T> is not 'static, so we cannot move it into spawn directly.
let handle = app.handle().clone();
tokio::spawn(async move {
    handle.state::<AppState>().rebuild_watcher().await;
});
```

- [ ] **Step 3: Verify the full build**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error" | head -30
```

Expected: no `error` lines

- [ ] **Step 4: Run all tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
```

Expected: all existing tests pass plus the new ones:
```
test db::store::tests::test_soft_delete_by_prefix ... ok
test tests::test_watched_dirs_round_trip ... ok
test tests::test_load_watched_dirs_missing_file ... ok
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: resume file watching on app startup from persisted dirs"
```

---

## Task 7: Manual verification

- [ ] Build and run the app:

```bash
cargo tauri dev --manifest-path src-tauri/Cargo.toml 2>&1
```

Or via npm:
```bash
cd /home/huarenyu/work/work-auto/fileflow && npm run tauri dev
```

- [ ] Add a directory via the UI ("添加目录" button in Sidebar)
- [ ] Create a new text file inside that directory
- [ ] Verify the file appears in the index (check StatusBar counter increments)
- [ ] Remove the directory via UI
- [ ] Verify the file no longer appears in search results
- [ ] Restart the app — verify the previously added directory resumes watching without needing re-add
- [ ] Add a directory with a path containing a single quote (if possible on your OS) — verify no crash

---

## Summary of changes

| File | Lines changed |
|------|--------------|
| `src-tauri/src/db/store.rs` | +12 (method) +35 (test) |
| `src-tauri/src/lib.rs` | +2 struct fields, +8 rebuild_watcher, +15 load/save helpers, +12 add_directory update, +14 remove_directory impl, +8 startup wiring, +20 tests |
