# FileFlow 全栈测试覆盖 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 FileFlow 建立四层测试基础设施（Rust 单元测试 → 前端单元测试 → Rust 集成测试 → E2E 测试），重点防止预览 URL 协议类 bug 重现。

**Architecture:** 分层由下至上：Layer 1 为纯 Rust 函数逻辑测试（无 IO）；Layer 2 为前端组件测试（mock 全部 Tauri IPC）；Layer 3 为 Rust 集成测试（真实 LanceDB，绕过 AppHandle）；Layer 4 为 WebdriverIO E2E 测试（真实 Tauri 窗口）。

**Tech Stack:** Rust/cargo test, Vitest 3.x, React Testing Library 16.x, @testing-library/jest-dom, jsdom, WebdriverIO 9.x, wdio-tauri-service, tauri-driver

---

## 文件映射

### 新建文件
- `src-tauri/src/preview.rs`（修改：添加测试模块）
- `vitest.config.ts`
- `src/test/setup.ts`
- `src/components/preview/__tests__/PreviewPanel.test.tsx`
- `src/components/__tests__/SearchBar.test.tsx`
- `src/components/__tests__/FileList.test.tsx`
- `src/components/__tests__/StatusBar.test.tsx`
- `src/hooks/__tests__/useSearch.test.ts`
- `src-tauri/tests/store_integration.rs`
- `src-tauri/tests/search_integration.rs`
- `src-tauri/tests/retry_queue.rs`
- `e2e/fixtures/sample.txt`
- `e2e/fixtures/sample.pdf`（最小 PDF）
- `e2e/fixtures/sample.png`（1x1 PNG）
- `e2e/indexing.test.ts`
- `e2e/search.test.ts`
- `e2e/preview.test.ts`
- `wdio.conf.ts`

### 修改文件
- `src-tauri/src/preview.rs`（添加 `#[cfg(test)]` 模块）
- `package.json`（添加 test 脚本和 devDependencies）

---

## Task 1：Layer 1 — `preview.rs` 单元测试

**Files:**
- Modify: `src-tauri/src/preview.rs`

- [ ] **Step 1：写失败测试**

在 `src-tauri/src/preview.rs` 末尾追加（`tempfile` 已在 `[dev-dependencies]`）：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    fn make_file(ext: &str) -> NamedTempFile {
        tempfile::Builder::new()
            .suffix(&format!(".{ext}"))
            .tempfile()
            .unwrap()
    }

    fn make_file_with_content(ext: &str, content: &str) -> NamedTempFile {
        let mut f = make_file(ext);
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    fn cache() -> tempfile::TempDir {
        tempdir().unwrap()
    }

    #[test]
    fn preview_pdf_returns_path() {
        let f = make_file("pdf");
        let result = preview(f.path(), cache().path()).unwrap();
        assert!(matches!(result, PreviewData::Pdf { .. }));
        if let PreviewData::Pdf { path } = result {
            assert!(path.ends_with(".pdf"));
        }
    }

    #[test]
    fn preview_image_returns_path() {
        let f = make_file("jpg");
        let result = preview(f.path(), cache().path()).unwrap();
        assert!(matches!(result, PreviewData::Image { .. }));
    }

    #[test]
    fn preview_text_reads_content() {
        let f = make_file_with_content("txt", "hello fileflow");
        let result = preview(f.path(), cache().path()).unwrap();
        assert!(matches!(result, PreviewData::Text { .. }));
        if let PreviewData::Text { content, language } = result {
            assert!(content.contains("hello fileflow"));
            assert_eq!(language, "txt");
        }
    }

    #[test]
    fn preview_unknown_returns_metadata() {
        let f = make_file("xyz");
        let result = preview(f.path(), cache().path()).unwrap();
        assert!(matches!(result, PreviewData::Metadata { .. }));
    }

    #[test]
    fn preview_missing_file_returns_error() {
        // .xyz (unknown ext) goes through fs::metadata — will error if file missing
        // Note: .pdf with missing path returns Ok(Pdf{path}) — no existence check
        let p = std::path::Path::new("/tmp/nonexistent_fileflow_test.xyz");
        assert!(preview(p, cache().path()).is_err());
    }
}
```

- [ ] **Step 2：运行测试，确认失败**

```bash
cd /home/huarenyu/work/work-auto/fileflow/src-tauri
cargo test preview -- --nocapture 2>&1
```

期望：所有 5 个 `preview::tests` 测试 FAIL（函数不存在或 panic）

> 注意：因为 `preview()` 函数已存在，实际应该会 pass。如果直接 pass，跳到 Step 4。

- [ ] **Step 3：确认测试通过**

```bash
cargo test preview -- --nocapture 2>&1
```

期望输出：
```
test preview::tests::preview_pdf_returns_path ... ok
test preview::tests::preview_image_returns_path ... ok
test preview::tests::preview_text_reads_content ... ok
test preview::tests::preview_unknown_returns_metadata ... ok
test preview::tests::preview_missing_file_returns_error ... ok
```

- [ ] **Step 4：提交**

```bash
cd /home/huarenyu/work/work-auto/fileflow
git add src-tauri/src/preview.rs
git commit -m "test(layer1): add preview.rs unit tests"
```

---

## Task 2：Layer 2 — 安装 Vitest 并配置

**Files:**
- Modify: `package.json`
- Create: `vitest.config.ts`
- Create: `src/test/setup.ts`

- [ ] **Step 1：安装依赖**

```bash
cd /home/huarenyu/work/work-auto/fileflow
npm install -D vitest@3 @vitest/coverage-v8 \
  @testing-library/react@16 \
  @testing-library/jest-dom@6 \
  @testing-library/user-event@14 \
  jsdom
```

- [ ] **Step 2：创建 `vitest.config.ts`**

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: ['src/test/setup.ts'],
    globals: true,
  },
})
```

- [ ] **Step 3：创建 `src/test/setup.ts`**

```typescript
// src/test/setup.ts
import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock @tauri-apps/api/core — covers invoke + convertFileSrc
// vi.mock is hoisted by Vitest; placing here in setupFiles ensures all
// sub-components (ImagePreview, VideoPreview, PdfPreview) are covered.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://localhost${path}`,
}))

// Mock @tauri-apps/api/event — covers listen (used by useIndexProgress)
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}))

// Mock pdfjs-dist — jsdom has no canvas 2D context;
// new URL(..., import.meta.url) also fails in test environment.
vi.mock('pdfjs-dist', () => ({
  GlobalWorkerOptions: { workerSrc: '' },
  getDocument: vi.fn(() => ({
    promise: Promise.resolve({
      getPage: vi.fn(() =>
        Promise.resolve({
          getViewport: vi.fn(() => ({ width: 100, height: 100 })),
          render: vi.fn(() => ({ promise: Promise.resolve() })),
        })
      ),
    }),
  })),
}))
```

- [ ] **Step 4：在 `package.json` 添加脚本**

在 `scripts` 字段添加：
```json
"test": "vitest run",
"test:watch": "vitest",
"test:coverage": "vitest run --coverage"
```

- [ ] **Step 5：验证配置正确（空跑）**

```bash
npm test 2>&1
```

期望：`No test files found, exiting with code 1`（或 0 tests run，无报错）

- [ ] **Step 6：提交**

```bash
git add vitest.config.ts src/test/setup.ts package.json package-lock.json
git commit -m "test(layer2): install Vitest and configure global mocks"
```

---

## Task 3：Layer 2 — PreviewPanel 测试（核心回归测试）

**Files:**
- Create: `src/components/preview/__tests__/PreviewPanel.test.tsx`

这是最重要的测试，直接覆盖此次预览 URL 协议 bug。

- [ ] **Step 1：写失败测试**

```typescript
// src/components/preview/__tests__/PreviewPanel.test.tsx
import { render, screen, waitFor } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { vi } from 'vitest'
import { PreviewPanel } from '../PreviewPanel'
import type { SearchResult } from '../../../lib/types'

const mockFile: SearchResult = {
  file_id: 'file-1',
  path: '/home/user/docs/report.pdf',
  name: 'report.pdf',
  extension: 'pdf',
  size: 1024,
  modified_at: '2026-01-01T00:00:00Z',
  category: 'document',
  score: 0.9,
  thumbnail_path: null,
}

describe('PreviewPanel', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset()
  })

  it('shows placeholder when no file selected', () => {
    render(<PreviewPanel file={null} />)
    expect(screen.getByText('选择文件以预览')).toBeInTheDocument()
  })

  it('renders image with asset:// URL (not tauri://localhost)', async () => {
    const imageFile = { ...mockFile, path: '/home/user/photo.jpg', extension: 'jpg', name: 'photo.jpg' }
    vi.mocked(invoke).mockResolvedValue({ type: 'image', path: '/home/user/photo.jpg' })

    render(<PreviewPanel file={imageFile} />)

    await waitFor(() => {
      const img = screen.getByRole('img')
      expect(img).toHaveAttribute('src', expect.stringContaining('asset://localhost'))
      expect(img).not.toHaveAttribute('src', expect.stringContaining('tauri://localhost'))
    })
  })

  it('renders video with asset:// URL', async () => {
    const videoFile = { ...mockFile, path: '/home/user/clip.mp4', extension: 'mp4', name: 'clip.mp4' }
    vi.mocked(invoke).mockResolvedValue({ type: 'video', path: '/home/user/clip.mp4' })

    render(<PreviewPanel file={videoFile} />)

    await waitFor(() => {
      const video = document.querySelector('video')
      expect(video).toBeInTheDocument()
      expect(video?.src).toContain('asset://localhost')
    })
  })

  it('renders canvas for PDF type', async () => {
    vi.mocked(invoke).mockResolvedValue({ type: 'pdf', path: '/home/user/docs/report.pdf' })

    render(<PreviewPanel file={mockFile} />)

    await waitFor(() => {
      expect(document.querySelector('canvas')).toBeInTheDocument()
    })
  })

  it('renders text content in <pre>', async () => {
    const txtFile = { ...mockFile, path: '/home/user/notes.txt', extension: 'txt', name: 'notes.txt' }
    vi.mocked(invoke).mockResolvedValue({
      type: 'text',
      content: 'hello fileflow content',
      language: 'txt',
    })

    render(<PreviewPanel file={txtFile} />)

    await waitFor(() => {
      expect(screen.getByText('hello fileflow content')).toBeInTheDocument()
    })
  })

  it('does not crash when get_preview throws', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('file not found'))

    // Should render without throwing
    expect(() => render(<PreviewPanel file={mockFile} />)).not.toThrow()

    // After the rejection resolves, should show loading or empty, not crash
    await waitFor(() => {
      expect(screen.queryByText('选择文件以预览')).not.toBeInTheDocument()
    })
  })
})
```

- [ ] **Step 2：运行，确认失败（测试文件 not found 等）**

```bash
npm test -- PreviewPanel 2>&1
```

期望：测试文件被发现，但某些测试 FAIL（组件可能未找到等）

- [ ] **Step 3：运行，确认通过**

```bash
npm test -- PreviewPanel 2>&1
```

期望：6/6 tests pass

- [ ] **Step 4：提交**

```bash
git add src/components/preview/__tests__/PreviewPanel.test.tsx
git commit -m "test(layer2): add PreviewPanel regression tests for asset:// URL"
```

---

## Task 4：Layer 2 — SearchBar 测试

**Files:**
- Create: `src/components/__tests__/SearchBar.test.tsx`

- [ ] **Step 1：写测试**

`SearchBar` 只接受 `query` 和 `onQuery` props，不调用 `invoke`。

```typescript
// src/components/__tests__/SearchBar.test.tsx
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SearchBar } from '../SearchBar'

describe('SearchBar', () => {
  it('calls onQuery with typed value', async () => {
    const onQuery = vi.fn()
    render(<SearchBar query="" onQuery={onQuery} />)

    const input = screen.getByPlaceholderText('搜索文件，支持自然语言...')
    await userEvent.type(input, 'hello')

    expect(onQuery).toHaveBeenCalledWith(expect.stringContaining('h'))
  })

  it('shows clear button when query is non-empty', () => {
    render(<SearchBar query="test" onQuery={vi.fn()} />)
    expect(screen.getByText('✕')).toBeInTheDocument()
  })

  it('hides clear button when query is empty', () => {
    render(<SearchBar query="" onQuery={vi.fn()} />)
    expect(screen.queryByText('✕')).not.toBeInTheDocument()
  })

  it('calls onQuery("") when clear button clicked', async () => {
    const onQuery = vi.fn()
    render(<SearchBar query="some text" onQuery={onQuery} />)

    await userEvent.click(screen.getByText('✕'))

    expect(onQuery).toHaveBeenCalledWith('')
  })
})
```

- [ ] **Step 2：运行并确认通过**

```bash
npm test -- SearchBar 2>&1
```

期望：4/4 tests pass

- [ ] **Step 3：提交**

```bash
git add src/components/__tests__/SearchBar.test.tsx
git commit -m "test(layer2): add SearchBar callback tests"
```

---

## Task 5：Layer 2 — useSearch hook 测试

**Files:**
- Create: `src/hooks/__tests__/useSearch.test.ts`

- [ ] **Step 1：写测试**

`useSearch` 内部有 300ms debounce，需要 `vi.useFakeTimers()`。

```typescript
// src/hooks/__tests__/useSearch.test.ts
import { renderHook, act } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { vi } from 'vitest'
import { useSearch } from '../useSearch'

describe('useSearch', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('calls invoke("search_files") after 300ms debounce', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const { rerender } = renderHook(({ q }) => useSearch(q), {
      initialProps: { q: '' },
    })

    rerender({ q: 'hello' })

    // Before debounce fires — invoke should NOT have been called
    expect(invoke).not.toHaveBeenCalled()

    // Advance past debounce
    await act(async () => {
      vi.advanceTimersByTime(300)
    })

    expect(invoke).toHaveBeenCalledWith('search_files', expect.objectContaining({
      query: 'hello',
    }))
  })

  it('does not call invoke for empty or whitespace query', async () => {
    const { rerender } = renderHook(({ q }) => useSearch(q), {
      initialProps: { q: 'hello' },
    })

    rerender({ q: '   ' })

    await act(async () => {
      vi.advanceTimersByTime(500)
    })

    expect(invoke).not.toHaveBeenCalled()
  })

  it('updates results state after invoke resolves', async () => {
    const mockResults = [{
      file_id: 'f1', path: '/a.txt', name: 'a.txt',
      extension: 'txt', size: 100, modified_at: '2026-01-01',
      category: 'document', score: 0.9, thumbnail_path: null,
    }]
    vi.mocked(invoke).mockResolvedValue(mockResults)

    const { result, rerender } = renderHook(({ q }) => useSearch(q), {
      initialProps: { q: '' },
    })

    rerender({ q: 'search term' })

    await act(async () => {
      vi.advanceTimersByTime(300)
      // Let the promise resolve
      await Promise.resolve()
    })

    expect(result.current.results).toHaveLength(1)
    expect(result.current.results[0].name).toBe('a.txt')
  })
})
```

- [ ] **Step 2：运行并确认通过**

```bash
npm test -- useSearch 2>&1
```

期望：3/3 tests pass

- [ ] **Step 3：提交**

```bash
git add src/hooks/__tests__/useSearch.test.ts
git commit -m "test(layer2): add useSearch hook tests with fake timers"
```

---

## Task 6：Layer 2 — FileList 测试

**Files:**
- Create: `src/components/__tests__/FileList.test.tsx`

- [ ] **Step 1：写测试**

`FileList` 使用 `useFiles` 和 `useSearch` hooks，直接 mock hooks 而非底层 `invoke`，使测试更聚焦。

```typescript
// src/components/__tests__/FileList.test.tsx
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { vi } from 'vitest'
import { FileList } from '../FileList'
import type { SearchResult } from '../../lib/types'

// Mock hooks at module level — cleaner than controlling invoke return values
vi.mock('../../hooks/useFiles', () => ({
  useFiles: vi.fn(() => ({ files: [], loading: false })),
}))
vi.mock('../../hooks/useSearch', () => ({
  useSearch: vi.fn(() => ({ results: [], loading: false })),
}))

import { useFiles } from '../../hooks/useFiles'
import { useSearch } from '../../hooks/useSearch'

const mockResult: SearchResult = {
  file_id: 'f1',
  path: '/docs/report.pdf',
  name: 'report.pdf',
  extension: 'pdf',
  size: 2048,
  modified_at: '2026-01-01T00:00:00Z',
  category: 'document',
  score: 0.95,
  thumbnail_path: null,
}

describe('FileList', () => {
  it('shows empty state when no query and no files', () => {
    render(<FileList category="all" query="" onSelect={vi.fn()} />)
    expect(screen.getByText('此分类暂无文件')).toBeInTheDocument()
  })

  it('shows "未找到匹配文件" when searching with no results', () => {
    render(<FileList category="all" query="xyz123" onSelect={vi.fn()} />)
    expect(screen.getByText('未找到匹配文件')).toBeInTheDocument()
  })

  it('renders search results when query is non-empty', () => {
    vi.mocked(useSearch).mockReturnValue({ results: [mockResult], loading: false })

    render(<FileList category="all" query="report" onSelect={vi.fn()} />)

    expect(screen.getByText('report.pdf')).toBeInTheDocument()
  })

  it('calls onSelect when a file item is clicked', async () => {
    vi.mocked(useSearch).mockReturnValue({ results: [mockResult], loading: false })
    const onSelect = vi.fn()

    render(<FileList category="all" query="report" onSelect={onSelect} />)

    await userEvent.click(screen.getByText('report.pdf'))

    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ file_id: 'f1' }))
  })
})
```

- [ ] **Step 2：运行并确认通过**

```bash
npm test -- FileList 2>&1
```

期望：4/4 tests pass

- [ ] **Step 3：提交**

```bash
git add src/components/__tests__/FileList.test.tsx
git commit -m "test(layer2): add FileList tests"
```

---

## Task 7：Layer 2 — StatusBar 测试

**Files:**
- Create: `src/components/__tests__/StatusBar.test.tsx`

- [ ] **Step 1：写测试**

`StatusBar` 使用 `useIndexProgress` hook，mock 该 hook 控制渲染。

```typescript
// src/components/__tests__/StatusBar.test.tsx
import { render, screen } from '@testing-library/react'
import { vi } from 'vitest'
import { StatusBar } from '../StatusBar'

vi.mock('../../hooks/useIndexProgress', () => ({
  useIndexProgress: vi.fn(() => ({
    total: 0,
    indexed: 0,
    failed: 0,
    is_running: false,
  })),
}))

import { useIndexProgress } from '../../hooks/useIndexProgress'

describe('StatusBar', () => {
  it('shows indexed count when idle', () => {
    vi.mocked(useIndexProgress).mockReturnValue({
      total: 10, indexed: 8, failed: 0, is_running: false,
    })
    render(<StatusBar />)
    expect(screen.getByText('8 个文件已索引')).toBeInTheDocument()
  })

  it('shows failed count when failed > 0', () => {
    vi.mocked(useIndexProgress).mockReturnValue({
      total: 10, indexed: 8, failed: 2, is_running: false,
    })
    render(<StatusBar />)
    expect(screen.getByText(/2 个失败/)).toBeInTheDocument()
  })

  it('shows indexing state when is_running', () => {
    vi.mocked(useIndexProgress).mockReturnValue({
      total: 20, indexed: 5, failed: 0, is_running: true,
    })
    render(<StatusBar />)
    expect(screen.getByText('● 索引中')).toBeInTheDocument()
    expect(screen.getByText('5 / 20 文件')).toBeInTheDocument()
  })
})
```

- [ ] **Step 2：运行所有 Layer 2 测试，确认全部通过**

```bash
npm test 2>&1
```

期望输出类似：
```
Test Files  6 passed (6)
Tests      20 passed (20)
```

- [ ] **Step 3：提交**

```bash
git add src/components/__tests__/StatusBar.test.tsx
git commit -m "test(layer2): add StatusBar tests — complete Layer 2"
```

---

## Task 8：Layer 3 — store_integration.rs

**Files:**
- Create: `src-tauri/tests/store_integration.rs`

> `tempfile` 和 `tokio` 已在 `[dev-dependencies]`，无需新增依赖。

- [ ] **Step 1：写测试**

```rust
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
```

- [ ] **Step 2：运行，确认通过**

```bash
cd /home/huarenyu/work/work-auto/fileflow/src-tauri
cargo test --test store_integration -- --nocapture 2>&1
```

期望：3/3 tests pass

- [ ] **Step 3：提交**

```bash
cd /home/huarenyu/work/work-auto/fileflow
git add src-tauri/tests/store_integration.rs
git commit -m "test(layer3): add store integration tests — vector search, category filter, concurrent inserts"
```

---

## Task 9：Layer 3 — retry_queue.rs 集成测试

**Files:**
- Create: `src-tauri/tests/retry_queue.rs`

- [ ] **Step 1：写测试**

```rust
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
```

- [ ] **Step 2：运行，确认通过**

```bash
cd /home/huarenyu/work/work-auto/fileflow/src-tauri
cargo test --test retry_queue -- --nocapture 2>&1
```

期望：3/3 tests pass

- [ ] **Step 3：提交**

```bash
cd /home/huarenyu/work/work-auto/fileflow
git add src-tauri/tests/retry_queue.rs
git commit -m "test(layer3): add retry_queue integration tests"
```

---

## Task 10：Layer 3 — search_integration.rs

**Files:**
- Create: `src-tauri/tests/search_integration.rs`

> **注意**：`Embedder::new()` 在首次运行时会下载 AllMiniLML6V2 模型（约 22MB），后续使用缓存。`Embedder` 使用 `OnceCell` 全局单例，同一测试二进制内只初始化一次。

- [ ] **Step 1：写测试**

```rust
// src-tauri/tests/search_integration.rs
use fileflow_lib::{
    db::store::{FileChunkRecord, FileStore},
    embedder::Embedder,
    search::{SearchFilters, Searcher},
};
use std::sync::Arc;
use tempfile::tempdir;

async fn setup() -> (Arc<FileStore>, Arc<Embedder>, Searcher) {
    let dir = tempdir().unwrap();
    let store = Arc::new(FileStore::new(dir.path().to_str().unwrap()).await.unwrap());
    let embedder = Arc::new(Embedder::new().unwrap());
    let searcher = Searcher::new(store.clone(), embedder.clone());
    (store, embedder, searcher)
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
    let (store, embedder, searcher) = setup().await;

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
    let (store, embedder, searcher) = setup().await;

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
    let (store, embedder, searcher) = setup().await;

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
```

- [ ] **Step 2：运行（首次运行会下载模型）**

```bash
cd /home/huarenyu/work/work-auto/fileflow/src-tauri
cargo test --test search_integration -- --nocapture 2>&1
```

期望：3/3 tests pass（首次运行可能需要 1-2 分钟下载模型）

- [ ] **Step 3：提交**

```bash
cd /home/huarenyu/work/work-auto/fileflow
git add src-tauri/tests/search_integration.rs
git commit -m "test(layer3): add search integration tests — semantic search, category filter, multi-chunk aggregation"
```

---

## Task 11：Layer 4 — E2E 配置（WebdriverIO + tauri-driver）

**Files:**
- Modify: `package.json`
- Create: `wdio.conf.ts`
- Create: `e2e/fixtures/sample.txt`
- Create: `e2e/fixtures/sample.pdf`（二进制）
- Create: `e2e/fixtures/sample.png`（二进制）

- [ ] **Step 1：验证 tauri-driver 可用**

```bash
cargo install tauri-driver 2>&1 | tail -5
tauri-driver --version
```

期望：版本 ≥ 0.2。如果版本不兼容 Tauri 2.x，停止 Layer 4 并记录 issue。

- [ ] **Step 2：安装 WebdriverIO 依赖**

```bash
cd /home/huarenyu/work/work-auto/fileflow
npm install -D webdriverio@9 @wdio/local-runner@9 \
  @wdio/mocha-framework@9 @wdio/spec-reporter@9 \
  wdio-tauri-service ts-node
```

- [ ] **Step 3：创建 fixture 文件**

```bash
# sample.txt
echo "This document contains the keyword fileflow-test-keyword for search testing." \
  > e2e/fixtures/sample.txt

# sample.pdf — minimum valid PDF (manually create or copy a tiny one)
# Use this one-liner to create a minimal valid PDF:
printf '%%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>\nendobj\nxref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000058 00000 n\n0000000115 00000 n\ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n190\n%%%%EOF' > e2e/fixtures/sample.pdf

# sample.png — 1x1 white pixel PNG
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00\x00\x00\x00IEND\xaeB`\x82' > e2e/fixtures/sample.png
```

- [ ] **Step 4：创建 `wdio.conf.ts`**

```typescript
// wdio.conf.ts
import type { Options } from '@wdio/types'

export const config: Options.Testrunner = {
  runner: 'local',
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tsconfig.json',
      transpileOnly: true,
    },
  },

  specs: ['./e2e/**/*.test.ts'],
  maxInstances: 1,

  capabilities: [
    {
      // @ts-expect-error tauri-specific capability
      'tauri:options': {
        application: './src-tauri/target/debug/fileflow',
      },
      browserName: '',
    },
  ],

  services: [
    [
      'tauri',
      {
        tauriDriverPath: 'tauri-driver',
      },
    ],
  ],

  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
}
```

- [ ] **Step 5：在 `package.json` 添加 E2E 脚本**

```json
"test:e2e": "cargo build --manifest-path src-tauri/Cargo.toml && wdio run wdio.conf.ts"
```

- [ ] **Step 6：提交配置**

```bash
git add wdio.conf.ts e2e/fixtures/ package.json package-lock.json
git commit -m "test(layer4): add WebdriverIO + tauri-driver E2E configuration"
```

---

## Task 12：Layer 4 — E2E 测试文件

**Files:**
- Create: `e2e/indexing.test.ts`
- Create: `e2e/search.test.ts`
- Create: `e2e/preview.test.ts`

- [ ] **Step 1：写 `e2e/indexing.test.ts`**

```typescript
// e2e/indexing.test.ts
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

describe('Directory Indexing', () => {
  let fixtureDir: string

  before(async () => {
    // Copy fixtures to a temp dir so we don't pollute the source tree
    fixtureDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fileflow-e2e-'))
    fs.copyFileSync('e2e/fixtures/sample.txt', path.join(fixtureDir, 'sample.txt'))
    fs.copyFileSync('e2e/fixtures/sample.png', path.join(fixtureDir, 'sample.png'))
  })

  after(() => {
    fs.rmSync(fixtureDir, { recursive: true, force: true })
  })

  it('indexes a directory and shows progress in StatusBar', async () => {
    // Click "添加目录" button
    const addBtn = await $('[data-testid="add-directory"]')
    await addBtn.click()

    // The dialog is handled by Tauri — inject the path via keyboard shortcut
    // Since file dialogs are OS-native, we instead invoke the command directly
    // through WebdriverIO's execute to call the Tauri command
    await browser.execute(
      (dirPath: string) => {
        // @ts-expect-error tauri is available in the WebView
        window.__TAURI__.core.invoke('add_directory', { path: dirPath })
      },
      fixtureDir
    )

    // Wait for StatusBar to show indexed > 0
    await browser.waitUntil(
      async () => {
        const footer = await $('footer')
        const text = await footer.getText()
        return text.includes('个文件已索引') && !text.includes('0 个文件已索引')
      },
      { timeout: 30000, timeoutMsg: 'Indexing did not complete within 30s' }
    )

    const footer = await $('footer')
    expect(await footer.getText()).not.toContain('个失败')
  })
})
```

- [ ] **Step 2：写 `e2e/search.test.ts`**

```typescript
// e2e/search.test.ts
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

describe('Search', () => {
  let fixtureDir: string

  before(async () => {
    fixtureDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fileflow-e2e-search-'))
    fs.copyFileSync('e2e/fixtures/sample.txt', path.join(fixtureDir, 'sample.txt'))

    // Index the directory
    await browser.execute(
      (dirPath: string) => {
        // @ts-expect-error
        window.__TAURI__.core.invoke('add_directory', { path: dirPath })
      },
      fixtureDir
    )

    // Wait for indexing to complete
    await browser.waitUntil(
      async () => {
        const footer = await $('footer')
        const text = await footer.getText()
        return text.includes('个文件已索引') && !text.includes('0 个文件已索引')
      },
      { timeout: 30000 }
    )
  })

  after(() => {
    fs.rmSync(fixtureDir, { recursive: true, force: true })
  })

  it('returns results for a keyword present in indexed file', async () => {
    const input = await $('input[type="text"]')
    await input.setValue('fileflow-test-keyword')

    await browser.waitUntil(
      async () => {
        const items = await $$('[data-testid="file-item"]')
        return items.length > 0
      },
      { timeout: 10000, timeoutMsg: 'No search results appeared' }
    )

    const items = await $$('[data-testid="file-item"]')
    expect(items.length).toBeGreaterThan(0)

    // sample.txt should appear in results
    const texts = await Promise.all(items.map((i) => i.getText()))
    expect(texts.some((t) => t.includes('sample.txt'))).toBe(true)
  })
})
```

- [ ] **Step 3：写 `e2e/preview.test.ts`**

```typescript
// e2e/preview.test.ts
// This test validates TWO fixes simultaneously:
// 1. Frontend uses convertFileSrc() — produces asset://localhost URLs
// 2. tauri.conf.json has assetProtocol.enable: true — allows file loading
// If either is missing, the img src will have the wrong protocol.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

describe('Preview Panel', () => {
  let fixtureDir: string

  before(async () => {
    fixtureDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fileflow-e2e-preview-'))
    fs.copyFileSync('e2e/fixtures/sample.txt', path.join(fixtureDir, 'sample.txt'))
    fs.copyFileSync('e2e/fixtures/sample.png', path.join(fixtureDir, 'sample.png'))
    fs.copyFileSync('e2e/fixtures/sample.pdf', path.join(fixtureDir, 'sample.pdf'))

    await browser.execute(
      (dirPath: string) => {
        // @ts-expect-error
        window.__TAURI__.core.invoke('add_directory', { path: dirPath })
      },
      fixtureDir
    )

    await browser.waitUntil(
      async () => {
        const footer = await $('footer')
        const text = await footer.getText()
        return text.includes('个文件已索引') && !text.includes('0 个文件已索引')
      },
      { timeout: 30000 }
    )
  })

  after(() => {
    fs.rmSync(fixtureDir, { recursive: true, force: true })
  })

  it('image preview uses asset://localhost (not tauri://localhost)', async () => {
    // List all files and click on sample.png
    const input = await $('input[type="text"]')
    await input.setValue('sample')
    await browser.waitUntil(async () => {
      const items = await $$('[data-testid="file-item"]')
      return items.length > 0
    }, { timeout: 10000 })

    const items = await $$('[data-testid="file-item"]')
    const pngItem = await Promise.all(
      items.map(async (i) => ({ el: i, text: await i.getText() }))
    ).then((all) => all.find((x) => x.text.includes('sample.png'))?.el)

    expect(pngItem).toBeDefined()
    await pngItem!.click()

    // Wait for preview panel to show an img
    await browser.waitUntil(
      async () => !!(await $('img[src*="asset://localhost"]').isExisting()),
      { timeout: 5000, timeoutMsg: 'Image preview did not appear with asset:// URL' }
    )

    const img = await $('img[src*="asset://localhost"]')
    const src = await img.getAttribute('src')
    expect(src).toContain('asset://localhost')
    expect(src).not.toContain('tauri://localhost')
  })

  it('text preview shows file content in <pre>', async () => {
    const input = await $('input[type="text"]')
    await input.setValue('')
    await browser.pause(500)

    const items = await $$('[data-testid="file-item"]')
    const txtItem = await Promise.all(
      items.map(async (i) => ({ el: i, text: await i.getText() }))
    ).then((all) => all.find((x) => x.text.includes('sample.txt'))?.el)

    if (txtItem) await txtItem.click()

    await browser.waitUntil(
      async () => !!(await $('pre').isExisting()),
      { timeout: 5000 }
    )

    const pre = await $('pre')
    expect(await pre.getText()).toContain('fileflow-test-keyword')
  })

  it('PDF preview shows a canvas element', async () => {
    const items = await $$('[data-testid="file-item"]')
    const pdfItem = await Promise.all(
      items.map(async (i) => ({ el: i, text: await i.getText() }))
    ).then((all) => all.find((x) => x.text.includes('sample.pdf'))?.el)

    if (pdfItem) await pdfItem.click()

    await browser.waitUntil(
      async () => !!(await $('canvas').isExisting()),
      { timeout: 5000, timeoutMsg: 'PDF canvas did not appear' }
    )
  })
})
```

- [ ] **Step 4：在前端组件中添加 `data-testid` 属性**

E2E 测试依赖 `data-testid` 选择器。修改以下文件：

**`src/components/StatusBar.tsx`**：在 `<footer>` 添加 `data-testid="status-bar"`

**`src/components/FileItem.tsx`**：在根元素添加 `data-testid="file-item"`

**`src/components/Sidebar.tsx`**：在"添加目录"按钮添加 `data-testid="add-directory"`

> 具体位置：打开文件，找到对应 JSX 元素，加上 `data-testid` 属性。

- [ ] **Step 5：构建并运行 E2E（需要图形环境）**

```bash
# 如果是 Linux 无头环境，先启动 Xvfb
Xvfb :99 -screen 0 1280x800x24 &
export DISPLAY=:99

npm run test:e2e 2>&1
```

期望：3 个 test suite，全部 pass（或记录已知 tauri-driver 兼容性问题）

- [ ] **Step 6：提交**

```bash
git add e2e/ wdio.conf.ts src/components/FileItem.tsx src/components/StatusBar.tsx src/components/Sidebar.tsx
git commit -m "test(layer4): add E2E tests for indexing, search, and preview URL validation"
```

---

## 验收标准

运行以下命令，全部通过：

```bash
# Layer 1 + Layer 3 (Rust)
cd src-tauri && cargo test 2>&1 | tail -5

# Layer 2 (前端)
cd .. && npm test 2>&1 | tail -5

# Layer 4 (E2E) — 需要图形环境
npm run test:e2e 2>&1 | tail -10
```

期望输出：
```
# Rust: test result: ok. N passed; 0 failed
# Frontend: Tests N passed (N)
# E2E: 3 passing
```
