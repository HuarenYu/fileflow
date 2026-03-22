# FileFlow 全栈测试覆盖设计

**日期：** 2026-03-22
**状态：** 已批准
**背景：** 预览模块使用了错误的 URL 协议（`tauri://localhost` 替代 `convertFileSrc`），该 bug 因项目缺乏测试基础设施而未被发现。本设计补充四层测试覆盖，防止同类问题重现。

---

## 目标

- 为 FileFlow（Tauri 2.x + React + TypeScript + Rust + LanceDB）建立完整测试基础设施
- 重点覆盖 Tauri IPC 协议边界（Rust 后端 ↔ React 前端）
- 确保预览 URL 生成逻辑在前端组件层被验证
- 不引入过度抽象，测试与实现保持 1:1 对应

---

## 技术选型

| 层次 | 工具 | 运行方式 |
|------|------|----------|
| Rust 单元测试 | `cargo test`（内置） | `cargo test` |
| 前端单元测试 | Vitest + React Testing Library + jsdom | `npm test` |
| Rust 集成测试 | `cargo test --test *` + tempfile | `cargo test` |
| E2E 测试 | WebdriverIO + tauri-driver | `npm run test:e2e` |

---

## 架构

```
Layer 4: E2E (WebdriverIO + tauri-driver)
         └─ 真实 Tauri 窗口，模拟用户操作完整流程

Layer 3: Rust 集成测试 (cargo test --test *)
         └─ 跨模块交互：FileStore + Embedder + Searcher
         └─ 不经过 IndexPipeline（原因见下文）

Layer 2: 前端单元测试 (Vitest + React Testing Library)
         └─ 组件行为：PreviewPanel、SearchBar、FileList 等
         └─ Tauri IPC 全部 mock（全局注入，对所有测试文件生效）

Layer 1: Rust 单元测试（现有 14 个 + 补充）
         └─ 纯函数逻辑：chunker、classifier、preview 返回值等
```

**边界原则：**
- Layer 1：无 IO、无 IPC，纯函数逻辑，不依赖 LanceDB 或 fastembed
- Layer 2：mock 全部 `invoke` 和 `convertFileSrc`，只测前端组件行为
- Layer 3：真实 LanceDB 和文件系统，不启动 Tauri 窗口，不使用 `AppHandle`
- Layer 4：启动完整应用，断言 UI 状态

---

## Layer 1：Rust 单元测试补充

### 归类原则

Layer 1 只包含**不依赖 LanceDB、fastembed、文件系统或 Tauri 运行时**的测试。`search.rs` 中的 `Searcher` 依赖 `Arc<FileStore>` 和 `Arc<Embedder>`（均有 IO），因此 search 相关测试归入 Layer 3。

### `src-tauri/src/db/store.rs`

现有 `#[cfg(test)]` 块但无测试函数，新增：

- `insert_and_query`：插入一批 chunk，向量查询返回正确结果
- `soft_delete_hides_record`：soft delete 后查询不再返回该记录
- `list_by_category_filters_correctly`：按 category 过滤返回正确子集

> **注意**：这三个测试依赖真实 LanceDB（涉及 IO），因此实际归入 Layer 3 的 `store_integration.rs`，不放在 `store.rs` 的 `#[cfg(test)]` 块中。

### `src-tauri/src/preview.rs`

完全无测试，新增纯逻辑测试（使用 `tempfile::NamedTempFile` 创建临时文件）：

> **实现说明**：`preview()` 签名为 `fn preview(path: &Path, cache_dir: &Path)`，非 Office 类型的测试可传任意临时目录作为 `cache_dir`。另外，`preview()` 对 PDF/Image/Video 类型**不检查文件是否存在**，直接返回 `Ok`；只有未知扩展名走 `fs::metadata` 才会在文件不存在时返回 `Err`。`preview_missing_file_returns_error` 应使用未知扩展名（如 `.xyz`）的不存在路径来触发错误。

- `preview_pdf_returns_path`：`.pdf` 临时文件返回 `PreviewData::Pdf { path }`
- `preview_image_returns_path`：`.jpg` 临时文件返回 `PreviewData::Image { path }`
- `preview_text_reads_content`：`.txt` 临时文件（含已知内容）返回 `PreviewData::Text { content, language: "txt" }`
- `preview_unknown_returns_metadata`：`.xyz` 临时文件返回 `PreviewData::Metadata`
- `preview_missing_file_returns_error`：**不存在**的 `.xyz` 路径返回 `Err`（注意：不存在的 `.pdf` 路径会返回 `Ok`）

这些测试只验证 `preview()` 函数的返回类型与路径，不涉及向量或嵌入，属于纯单元测试。

---

## Layer 2：前端单元测试

### Mock 策略（关键）

所有 Tauri mock 通过 `vitest.config.ts` 的 `setupFiles` 全局注入，对所有测试文件自动生效，无需在每个测试文件中重复 `vi.mock`。

**`src/test/setup.ts`**（通过 `setupFiles` 加载）：
```ts
import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock @tauri-apps/api/core（invoke + convertFileSrc）
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://localhost${path}`,
}))

// Mock @tauri-apps/api/event（listen，供 useIndexProgress 使用）
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}))
```

> **说明**：`vi.mock` 在 Vitest 中会被 hoisted 到文件顶部，通过 `setupFiles` 统一注入可确保对所有子组件的导入都生效，包括 `ImagePreview`、`VideoPreview`、`PdfPreview` 中对 `convertFileSrc` 的调用。

**`vitest.config.ts`**：
```ts
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

### pdfjs-dist Mock

`PdfPreview.tsx` 使用 `pdfjs-dist` 在 `useEffect` 中异步渲染 PDF 到 canvas。jsdom 不支持 canvas 2D 上下文，且 `new URL(..., import.meta.url)` 在测试环境下不可用。因此对 `pdfjs-dist` 整体 mock：

在 `src/test/setup.ts` 中追加：
```ts
vi.mock('pdfjs-dist', () => ({
  GlobalWorkerOptions: { workerSrc: '' },
  getDocument: vi.fn(() => ({
    promise: Promise.resolve({
      getPage: vi.fn(() => Promise.resolve({
        getViewport: vi.fn(() => ({ width: 100, height: 100 })),
        render: vi.fn(() => ({ promise: Promise.resolve() })),
      })),
    }),
  })),
}))
```

测试目标降级为验证 `<canvas>` 元素出现在 DOM 中，不验证渲染内容。

### 测试文件

**`src/components/preview/__tests__/PreviewPanel.test.tsx`**（覆盖预览 bug）

`invoke` mock 返回不同的 `PreviewData`，验证子组件正确渲染：

- `type: "image"` 时：`<img src>` 包含 `asset://localhost`（不含 `tauri://localhost`）
- `type: "video"` 时：`<video src>` 包含 `asset://localhost`
- `type: "pdf"` 时：`<canvas>` 出现在 DOM 中
- `type: "text"` 时：`<pre>` 包含 mock 返回的文本内容
- `file=null` 时：显示"选择文件以预览"
- `get_preview` 抛错时：不崩溃，不持续显示 Loading

**`src/components/__tests__/SearchBar.test.tsx`**

`SearchBar` 只接受 `query` 和 `onQuery` props，自身不调用 `invoke`，测试其回调行为：

- 输入文字后 `onQuery` 被调用，参数为输入值
- 清空按钮出现后，点击触发 `onQuery("")`

**`src/hooks/__tests__/useSearch.test.ts`**

`invoke` 触发的验证放在 hook 测试中。`useSearch` 内部有 300ms debounce（`setTimeout`），测试中需使用 `vi.useFakeTimers()` + `vi.advanceTimersByTime(300)` 推进时间，否则 `invoke` 不会被调用：

- 调用 `search` 后，推进 300ms，`invoke("search_files", ...)` 被调用
- 空查询不触发 `invoke`（即使推进时间）
- `invoke` 返回结果后，`results` 状态更新

**`src/components/__tests__/FileList.test.tsx`**

- 渲染结果列表，点击文件触发 `onSelect` 回调
- 空结果时显示空状态

**`src/components/__tests__/StatusBar.test.tsx`**

- 显示 `indexed/total` 进度（通过 mock `useIndexProgress` hook）
- `failed > 0` 时显示错误状态

### npm 脚本

```json
"test": "vitest run",
"test:watch": "vitest",
"test:coverage": "vitest run --coverage"
```

---

## Layer 3：Rust 集成测试

放在 `src-tauri/tests/` 目录，用 `tempfile` crate 提供隔离临时目录。

### `IndexPipeline` 的 `AppHandle` 限制

`IndexPipeline::new` 接受 `AppHandle` 参数（用于 `app_handle.emit()` 发送进度事件），而 `AppHandle` 是 Tauri 运行时对象，在 `cargo test` 环境下无法构造。

**解决方案**：集成测试**不经过 `IndexPipeline`**，直接测试 `FileStore` + `Embedder` + `Searcher` 的组合，绕过 `AppHandle` 依赖。完整的索引流程（含进度事件）由 Layer 4 E2E 测试覆盖。

**`[dev-dependencies]` 新增：**
```toml
tempfile = "3"
```

### `src-tauri/tests/store_integration.rs`

> **与现有测试的关系**：`store.rs` 已有两个内模块测试（`test_insert_and_query_chunk`、`test_soft_delete_by_prefix`），覆盖了基础插入和 soft delete。新的集成测试补充**尚未覆盖**的路径：向量查询精度、category 过滤、并发安全。

- `insert_then_query_vector`：插入含已知向量的 chunk，向量查询返回正确记录（补充：验证向量相似度排序）
- `insert_then_list_by_category`：不同 category 的 chunk，`list_by_category` 只返回对应分类（补充：category 过滤）
- `soft_delete_then_query`：soft delete 后向量查询不返回已删除记录（补充：验证查询层的 deleted_at 过滤）
- `concurrent_inserts`：4 个并发任务同时插入，最终记录数正确（验证 semaphore）

### `src-tauri/tests/search_integration.rs`

（原 Layer 1 中 `search.rs` 的测试用例，使用真实 `FileStore` + `Embedder`）

- `search_returns_relevant_file`：插入含关键词的文档 → 搜索该关键词 → 结果包含该文件
- `filter_by_category`：插入不同 category 的文档 → 带 category filter 的搜索只返回对应分类
- `aggregate_best_score_per_file`：同一 file_id 多个 chunk → 搜索只返回最高分的那一条

### `src-tauri/tests/retry_queue.rs`

- `failed_file_queued_and_retried`：向 retry queue 推入失败记录，drain 后记录被消费

---

## Layer 4：E2E 测试（WebdriverIO + tauri-driver）

### tauri-driver 与 Tauri 2.x 兼容性

`tauri-driver` 在 Tauri 2.x 下的支持仍处于演进阶段。实施前需确认：

1. 安装 `tauri-driver`：`cargo install tauri-driver`
2. 确认版本：`tauri-driver --version` 应 ≥ 0.2（Tauri 2.x 支持版本）
3. Linux 构建环境需要 `Xvfb` 运行无头模式：`Xvfb :0 &` + `DISPLAY=:0`
4. 如果 `tauri-driver` 与当前 Tauri 2.x 版本不兼容，备选方案是 Playwright + `tauri://localhost` 协议

### 配置

**`wdio.conf.ts`**（关键配置项）：
```ts
services: [['tauri', { tauriDriverPath: 'tauri-driver' }]],
capabilities: [{
  'tauri:options': { application: 'src-tauri/target/debug/fileflow' }
}],
specs: ['e2e/**/*.test.ts'],
framework: 'mocha',
```

测试前须先执行 `cargo build`（debug 模式）。

### 测试数据

`e2e/fixtures/` 目录预置：
- `sample.txt`（含可搜索关键词 "fileflow-test-keyword"）
- `sample.pdf`（最小合法 PDF，< 10KB）
- `sample.png`（1x1 像素 PNG）

测试启动时复制到 tempdir，测完删除。

### 测试文件

**`e2e/indexing.test.ts`**
- 点击"添加目录" → 选择 fixtures tempdir → 等待 StatusBar `indexed > 0` → 断言 `failed === 0`

**`e2e/search.test.ts`**
- 搜索 "fileflow-test-keyword" → 等待 FileList 渲染 → 断言结果非空、`sample.txt` 文件名可见

**`e2e/preview.test.ts`**（直接覆盖此次 bug）

> **注意**：此测试同时验证两个修复：(1) 前端使用 `convertFileSrc` 生成正确 URL；(2) `tauri.conf.json` 中 `assetProtocol.enable: true` 已配置。两者缺一，`<img src>` 将包含错误协议或图片无法加载。

- 点击 `sample.png` → 断言 PreviewPanel 内 `<img src>` 以 `asset://localhost` 开头（不含 `tauri://localhost`）
- 点击 `sample.txt` → 断言 `<pre>` 有内容
- 点击 `sample.pdf` → 断言 `<canvas>` 出现

### npm 脚本

```json
"test:e2e": "wdio run wdio.conf.ts"
```

---

## 目录结构（完成后）

```
fileflow/
├── src/
│   ├── components/
│   │   ├── preview/
│   │   │   └── __tests__/
│   │   │       └── PreviewPanel.test.tsx
│   │   └── __tests__/
│   │       ├── SearchBar.test.tsx
│   │       ├── FileList.test.tsx
│   │       └── StatusBar.test.tsx
│   ├── hooks/
│   │   └── __tests__/
│   │       └── useSearch.test.ts
│   └── test/
│       └── setup.ts
├── e2e/
│   ├── fixtures/
│   │   ├── sample.txt
│   │   ├── sample.pdf
│   │   └── sample.png
│   ├── indexing.test.ts
│   ├── search.test.ts
│   └── preview.test.ts
├── src-tauri/
│   └── tests/
│       ├── store_integration.rs
│       ├── search_integration.rs
│       └── retry_queue.rs
├── vitest.config.ts
└── wdio.conf.ts
```

---

## 实施顺序

1. **Layer 1**：补充 `preview.rs` 单元测试（使用 `tempfile`）
2. **Layer 2**：配置 Vitest + mock，写前端测试（`PreviewPanel` 优先）
3. **Layer 3**：添加 `tempfile` 依赖，写 `store_integration`、`search_integration`、`retry_queue`
4. **Layer 4**：验证 `tauri-driver` 兼容性，配置 WebdriverIO，写 E2E 测试

---

## 不在范围内

- CI/CD 流水线配置（另立 issue）
- 性能测试
- OCR、LibreOffice 路径的测试（依赖外部工具，暂跳过）
- 视觉回归测试
- `IndexPipeline` 的测试（需先重构以移除 `AppHandle` 依赖，另立 issue）
