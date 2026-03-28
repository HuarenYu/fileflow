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
