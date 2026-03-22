# FileFlow 全栈测试覆盖设计

**日期：** 2026-03-22
**状态：** 已批准
**背景：** 预览模块使用了错误的 URL 协议（`tauri://localhost` 替代 `convertFileSrc`），该 bug 因项目缺乏测试基础设施而未被发现。本设计补充四层测试覆盖，防止同类问题重现。

---

## 目标

- 为 FileFlow（Tauri 2.x + React + Rust + LanceDB）建立完整测试基础设施
- 重点覆盖 Tauri IPC 协议边界（Rust 后端 ↔ React 前端）
- 确保预览 URL 生成逻辑在前端组件层被验证
- 不引入过度抽象，测试与实现保持 1:1 对应

---

## 技术选型

| 层次 | 工具 | 运行方式 |
|------|------|----------|
| Rust 单元测试 | `cargo test` (内置) | `cargo test` |
| 前端单元测试 | Vitest + React Testing Library + jsdom | `npm test` |
| Rust 集成测试 | `cargo test --test *` + tempfile | `cargo test` |
| E2E 测试 | WebdriverIO + tauri-driver | `npm run test:e2e` |

---

## 架构

```
Layer 4: E2E (WebdriverIO + tauri-driver)
         └─ 真实 Tauri 窗口，模拟用户操作完整流程

Layer 3: Rust 集成测试 (cargo test --test *)
         └─ 跨模块交互：FileStore + IndexPipeline + Searcher

Layer 2: 前端单元测试 (Vitest + React Testing Library)
         └─ 组件行为：PreviewPanel、SearchBar、FileList 等
         └─ Tauri IPC 全部 mock

Layer 1: Rust 单元测试 (现有 14 个 + 补充)
         └─ 纯逻辑：chunker、classifier、preview URL 生成等
```

**边界原则：**
- Layer 1：无 IO、无 IPC，纯函数逻辑
- Layer 2：mock 全部 `invoke`，只测前端组件行为
- Layer 3：真实 LanceDB 和文件系统，不启动 Tauri 窗口
- Layer 4：启动完整应用，断言 UI 状态

---

## Layer 1：Rust 单元测试补充

### `src-tauri/src/db/store.rs`

现有 `#[cfg(test)]` 块但无测试函数，新增：

- `insert_and_query`：插入一批 chunk，向量查询返回正确结果
- `soft_delete_hides_record`：soft delete 后查询不再返回该记录
- `list_by_category_filters_correctly`：按 category 过滤返回正确子集

### `src-tauri/src/preview.rs`

完全无测试，新增：

- `preview_pdf_returns_path`：`.pdf` 文件返回 `PreviewData::Pdf { path }`
- `preview_image_returns_path`：`.jpg/.png` 返回 `PreviewData::Image { path }`
- `preview_text_reads_content`：`.txt` 返回 `PreviewData::Text { content, language }`
- `preview_unknown_returns_metadata`：未知扩展名返回 `PreviewData::Metadata`
- `preview_missing_file_returns_error`：不存在的文件返回 `Err`

### `src-tauri/src/search.rs`

无测试，新增：

- `filter_by_category`：category filter 正确传递给 LanceDB query
- `aggregate_best_score_per_file`：同一 file_id 多个 chunk 只返回最高分

---

## Layer 2：前端单元测试

### 配置文件

**`vitest.config.ts`**
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

**`src/test/setup.ts`**
```ts
import '@testing-library/jest-dom'

// Mock Tauri internals
Object.defineProperty(window, '__TAURI_INTERNALS__', {
  value: { invoke: vi.fn() },
  writable: true,
})
```

**`src/test/mocks/tauri.ts`**
```ts
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://localhost${path}`,
}))
```

### 测试文件

**`src/components/preview/__tests__/PreviewPanel.test.tsx`**（覆盖预览 bug）

- 选中 PDF 文件时，调用 `get_preview`，渲染 `<canvas>`
- 选中图片时，`<img src>` 包含 `asset://localhost`（不含 `tauri://localhost`）
- 选中视频时，`<video src>` 包含 `asset://localhost`
- `file=null` 时显示"选择文件以预览"
- `get_preview` 抛错时不崩溃，不显示 Loading 状态

**`src/components/__tests__/SearchBar.test.tsx`**

- 输入查询后触发 `search_files` invoke
- 空查询不触发 invoke

**`src/components/__tests__/FileList.test.tsx`**

- 渲染结果列表，点击文件触发 `onSelect` 回调
- 空结果时显示空状态

**`src/components/__tests__/StatusBar.test.tsx`**

- 显示 `indexed/total` 进度
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

**`[dev-dependencies]` 新增：**
```toml
tempfile = "3"
```

### `src-tauri/tests/indexing_pipeline.rs`

- `index_and_search`：写 `.txt` 文件 → `IndexPipeline::index_file` → `Searcher::search` 关键词 → 结果包含该文件
- `reindex_updates_content`：修改文件内容后重新索引 → 旧 chunk 被替换，搜索返回新内容
- `deleted_file_not_in_results`：soft delete 后搜索不返回该文件

### `src-tauri/tests/store_integration.rs`

- `insert_then_list_by_category`：不同 category 的 chunk，`list_by_category` 只返回对应分类
- `soft_delete_then_query`：soft delete 后向量查询不返回已删除记录
- `concurrent_inserts`：4 个并发任务同时插入，记录数正确（验证 semaphore）

### `src-tauri/tests/retry_queue.rs`

- `failed_file_queued_and_retried`：索引失败的文件进入 retry queue，drain 后重新处理

---

## Layer 4：E2E 测试（WebdriverIO + tauri-driver）

### 配置

**`wdio.conf.ts`**（关键配置项）
```ts
services: [['tauri', { tauriDriverPath: 'tauri-driver' }]],
capabilities: [{
  'tauri:options': { application: 'src-tauri/target/debug/fileflow' }
}],
specs: ['e2e/**/*.test.ts'],
```

测试前须先执行 `cargo build`（debug 模式）。

### 测试数据

`e2e/fixtures/` 目录预置：
- `sample.txt`（含可搜索关键词）
- `sample.pdf`（小型 PDF）
- `sample.png`（小图片）

测试启动时复制到 tempdir，测完删除。

### 测试文件

**`e2e/indexing.test.ts`**
- 点击"添加目录" → 选择 fixtures tempdir → 等待 StatusBar `indexed > 0` → 断言 `failed === 0`

**`e2e/search.test.ts`**
- 搜索关键词 → 等待 FileList 渲染 → 断言结果非空、文件名可见

**`e2e/preview.test.ts`**（直接覆盖此次 bug）
- 点击图片文件 → 断言 `<img src>` 以 `asset://localhost` 开头
- 点击文本文件 → 断言 `<pre>` 有内容
- 点击 PDF → 断言 `<canvas>` 出现

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
│   │   └── preview/
│   │       └── __tests__/
│   │           └── PreviewPanel.test.tsx
│   │   └── __tests__/
│   │       ├── SearchBar.test.tsx
│   │       ├── FileList.test.tsx
│   │       └── StatusBar.test.tsx
│   └── test/
│       ├── setup.ts
│       └── mocks/
│           └── tauri.ts
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
│       ├── indexing_pipeline.rs
│       ├── store_integration.rs
│       └── retry_queue.rs
├── vitest.config.ts
└── wdio.conf.ts
```

---

## 实施顺序

1. **Layer 1**：补充 Rust 单元测试（`preview.rs`、`store.rs`、`search.rs`）
2. **Layer 2**：配置 Vitest，写前端组件测试（`PreviewPanel` 优先）
3. **Layer 3**：添加 `tempfile` 依赖，写集成测试
4. **Layer 4**：配置 WebdriverIO + tauri-driver，写 E2E 测试

---

## 不在范围内

- CI/CD 流水线配置（另立 issue）
- 性能测试
- OCR、LibreOffice 相关路径的测试（依赖外部工具，暂跳过）
- 视觉回归测试
