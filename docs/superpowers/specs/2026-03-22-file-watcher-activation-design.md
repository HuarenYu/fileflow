# File Watcher Activation — Design Spec

**Date:** 2026-03-22
**Status:** Approved
**Scope:** P0 bug fix — wire up real-time file system monitoring

---

## Problem

FileFlow has a fully implemented `FileWatcher` (via the `notify` crate) that monitors directories for create/modify/remove events and triggers the index pipeline accordingly. However, it is never instantiated: `AppState._watcher` is always `None`. Additionally:

- `add_directory` indexes existing files but does not start watching for future changes
- `remove_directory` is a stub that returns `Ok(())` without soft-deleting files or updating the watch list
- Watched directories are not persisted — they are lost on app restart

---

## Goals

1. Activate file watching when a directory is added
2. Stop watching and soft-delete records when a directory is removed
3. Persist watched directories across app restarts
4. Keep changes minimal — do not modify the `FileWatcher` API

---

## Architecture

### Approach: Rebuild watcher on change (Option A)

Each time a directory is added or removed, the full `FileWatcher` is recreated with the updated directory list. The old watcher is dropped (which stops its internal notify thread). This avoids concurrent state management and works naturally with the existing `FileWatcher::new(dirs, pipeline)` signature.

---

## Components

### 1. Directory persistence — `AppState` helper methods

Two new private helpers on `AppState`:

```
fn load_watched_dirs(app_dir: &Path) -> Vec<PathBuf>
fn save_watched_dirs(app_dir: &Path, dirs: &[PathBuf])
```

File: `app_dir/watched_dirs.json` — a JSON array of absolute path strings.

Serialization: use `path.to_string_lossy().into_owned()` for each path when writing, and `PathBuf::from(string)` when reading. This round-trips correctly on all platforms including Windows (backslashes, drive letters).

`load_watched_dirs` returns an empty vec if the file does not exist or cannot be parsed. `save_watched_dirs` logs and ignores write failures (non-fatal).

### 2. `AppState` new fields

```rust
pub watched_dirs: Mutex<Vec<PathBuf>>,    // in-memory list
pub app_dir: PathBuf,                      // needed to locate watched_dirs.json
```

`_watcher: Mutex<Option<FileWatcher>>` already exists and is kept.

### 3. `rebuild_watcher` helper on `AppState`

```rust
async fn rebuild_watcher(&self) {
    let dirs = self.watched_dirs.lock().await.clone();
    // Filter out directories that no longer exist (e.g. unplugged USB drives)
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

Called after every directory add or remove. Both `watched_dirs` and `_watcher` use `tokio::sync::Mutex` (not `std::sync::Mutex`) to satisfy the `Send` bound required by Tauri async command handlers.

**Task lifetime note:** `FileWatcher::new` spawns a tokio task that owns the channel receiver. When the old `FileWatcher` is dropped, its `RecommendedWatcher` closes the channel sender. The spawned task drains remaining buffered messages (channel capacity: 100) before exiting — it does not stop instantaneously. During this brief drain window, the old task and the new task may both call `pipeline.index_file()` for the same path. `IndexPipeline.index_file()` is **not** idempotent at the database level — `tbl.add()` has no deduplication guard, so a duplicate call can insert a second set of rows for the same file. This is tolerated because: (a) watcher rebuilds only happen when the user explicitly adds or removes a directory, which is infrequent; (b) the drain window lasts only as long as it takes to flush at most 100 buffered messages; (c) duplicate rows are a minor data quality issue, not a crash or data loss. A future improvement could add a "delete existing chunks for file_id before insert" step in the pipeline.

When rebuilding from `None` (first call), no old task exists — no special case needed.

### 4. `add_directory` (updated)

Steps:
1. Walk existing files, spawn background indexing task (existing behaviour)
2. Lock `watched_dirs`; append path if not already present; unlock
3. Save `watched_dirs.json` (after releasing the lock)
4. Call `rebuild_watcher()` (after releasing the lock)

Locking discipline: `watched_dirs` is locked only for the mutation in step 2. Steps 3 and 4 run after the lock is released. Concurrent `add_directory` or `remove_directory` calls may interleave between steps 3 and 4, causing multiple `rebuild_watcher` calls — each rebuild will read the current state of `watched_dirs` at that moment, so the final watcher reflects the most recently completed mutation. This is safe.

### 5. `remove_directory` (implemented from stub)

Steps:
1. Soft-delete all `FileStore` records whose `path` starts with the given directory prefix
2. Lock `watched_dirs`; remove the directory entry; unlock
3. Save `watched_dirs.json` (after releasing the lock)
4. Call `rebuild_watcher()` (after releasing the lock)

Same locking discipline as `add_directory` — `watched_dirs` is locked only for step 2.

Requires a new `FileStore` method: `soft_delete_by_prefix(prefix: &str)` — issues an `.only_if(...)` update with a `LIKE` expression. Single quotes in the path must be escaped (replace `'` with `''`) before interpolation to avoid malformed SQL. Example:

```rust
let safe_prefix = prefix.replace('\'', "''");
tbl.update()
    .only_if(format!("path LIKE '{safe_prefix}%'"))
    ...
```

**Note:** The existing `soft_delete_by_path` and other query methods in `store.rs` have the same SQL injection exposure (unescaped string interpolation). Fixing those is out of scope for this PR but should be tracked as follow-up technical debt.

### 6. App startup (`run()`)

After building `AppState`:
1. Load `watched_dirs.json` → populate `watched_dirs` field
2. If list is non-empty, call `rebuild_watcher()` to resume monitoring (non-existent paths are filtered inside `rebuild_watcher`)

**Missed-event recovery is out of scope for this step.** Files created or modified while the app was not running will not appear in the index until they are modified again. This is acceptable: the user can remove and re-add the directory to force a re-index if needed. A future improvement could do a walkdir pass over `watched_dirs` on startup.

---

## Data Flow

```
User adds directory
  → add_directory command
    → spawn walkdir indexing task (background)
    → lock watched_dirs, append, unlock
    → save watched_dirs.json
    → rebuild_watcher (drops old watcher, creates new with full dir list)

User removes directory
  → remove_directory command
    → soft_delete_by_prefix in LanceDB
    → lock watched_dirs, remove entry, unlock
    → save watched_dirs.json
    → rebuild_watcher

App starts
  → load watched_dirs.json
  → rebuild_watcher (resumes monitoring all persisted dirs)

File changes on disk (while watcher active)
  → notify event → FileWatcher event loop
    → Create/Modify → pipeline.index_file(path)
    → Remove → pipeline.delete_file(path)
```

---

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/src/lib.rs` | Add `watched_dirs`, `app_dir` fields; add `rebuild_watcher`, `load_watched_dirs`, `save_watched_dirs`; update `add_directory`, `remove_directory`; wire startup |
| `src-tauri/src/db/store.rs` | Add `soft_delete_by_prefix` method |

No changes to `watcher.rs`, `commands/`, or frontend.

---

## Error Handling

- `FileWatcher::new()` returns `Result` — on failure, `_watcher` stays `None` and a `tracing::error!` is logged. The app continues without watching.
- `save_watched_dirs` failures are logged but non-fatal (in-memory list is still correct for the session).
- `soft_delete_by_prefix` failure is propagated as an error to the frontend caller.

---

## Testing

- Unit test for `soft_delete_by_prefix` in `store.rs` (alongside existing store tests)
- Unit test for `load_watched_dirs` / `save_watched_dirs` round-trip (write vec, read back, assert equal)
- Manual test: add dir → modify file inside → verify it appears indexed; remove dir → verify records soft-deleted
