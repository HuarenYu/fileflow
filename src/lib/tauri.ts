import { invoke } from "@tauri-apps/api/core";
import type { SearchResult, IndexStatus, FileChunk, Category } from "./types";

export interface SearchFilters {
  category?: string;
  extension?: string;
  min_size?: number;
  max_size?: number;
  after_ms?: number;
  before_ms?: number;
}

export const api = {
  addDirectory: (path: string) => invoke<void>("add_directory", { path }),

  removeDirectory: (path: string) => invoke<void>("remove_directory", { path }),

  getIndexStatus: () => invoke<IndexStatus>("get_index_status"),

  searchFiles: (query: string, filters: SearchFilters = {}) =>
    invoke<SearchResult[]>("search_files", { query, filters }),

  listFiles: (category?: string) =>
    invoke<FileChunk[]>("list_files", { category }),

  updateCategory: (fileId: string, category: string) =>
    invoke<void>("update_category", { fileId, category }),

  openFile: (path: string) => invoke<void>("open_file", { path }),

  getPreview: (path: string) => invoke<unknown>("get_preview", { path }),
};
