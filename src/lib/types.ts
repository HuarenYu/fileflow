export interface FileChunk {
  id: string;
  file_id: string;
  path: string;
  name: string;
  extension: string;
  size: number;
  modified_at: string;
  category: string;
  user_category: string | null;
  chunk_index: number;
  content_text: string;
  thumbnail_path: string | null;
  indexed_at: string;
  deleted_at: string | null;
}

export interface SearchResult {
  file_id: string;
  path: string;
  name: string;
  extension: string;
  size: number;
  modified_at: string;
  category: string;
  score: number;
  thumbnail_path: string | null;
}

export interface IndexStatus {
  total: number;
  indexed: number;
  failed: number;
  is_running: boolean;
}

export type Category =
  | "all" | "document" | "spreadsheet" | "image"
  | "video" | "audio" | "code" | "archive"
  | "installer" | "font" | "other";
