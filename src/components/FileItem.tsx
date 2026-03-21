import type { SearchResult, FileChunk } from "../lib/types";

type FileRow = SearchResult | FileChunk;

interface Props {
  file: FileRow;
  selected: boolean;
  onClick: () => void;
}

const EXT_ICONS: Record<string, string> = {
  pdf: "📄",
  docx: "📄",
  doc: "📄",
  txt: "📄",
  md: "📄",
  xlsx: "📊",
  xls: "📊",
  csv: "📊",
  jpg: "🖼",
  jpeg: "🖼",
  png: "🖼",
  gif: "🖼",
  svg: "🖼",
  webp: "🖼",
  mp4: "🎬",
  mov: "🎬",
  avi: "🎬",
  mkv: "🎬",
  mp3: "🎵",
  wav: "🎵",
  flac: "🎵",
  zip: "📦",
  rar: "📦",
  "7z": "📦",
  py: "💻",
  js: "💻",
  ts: "💻",
  rs: "💻",
  go: "💻",
};

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

export function FileItem({ file, selected, onClick }: Props) {
  const ext = "extension" in file ? file.extension : "";
  const icon = EXT_ICONS[ext] ?? "📎";
  const score = "score" in file ? (file as SearchResult).score : null;

  return (
    <div
      onClick={onClick}
      className={`flex items-center px-3 py-2 cursor-pointer gap-3 border-b border-gray-800 hover:bg-gray-800 transition-colors ${
        selected ? "bg-gray-700" : ""
      }`}
    >
      <span className="text-lg shrink-0">{icon}</span>
      <div className="flex-1 min-w-0">
        <p className="text-sm text-white truncate">{file.name}</p>
        <p className="text-xs text-gray-500">
          {formatSize(file.size)}
          {score != null && ` · ${Math.round(score * 100)}% 匹配`}
        </p>
      </div>
    </div>
  );
}
