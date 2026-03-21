import { useFiles } from "../hooks/useFiles";
import { useSearch } from "../hooks/useSearch";
import { FileItem } from "./FileItem";
import type { Category, SearchResult, FileChunk } from "../lib/types";

interface Props {
  category: Category;
  query: string;
  selectedId?: string;
  onSelect: (file: SearchResult) => void;
}

export function FileList({ category, query, selectedId, onSelect }: Props) {
  const { files, loading: filesLoading } = useFiles(category);
  const { results, loading: searchLoading } = useSearch(query);
  const isSearching = query.trim().length > 0;
  const items = isSearching ? results : files;
  const loading = isSearching ? searchLoading : filesLoading;

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
        加载中...
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
        {isSearching ? "未找到匹配文件" : "此分类暂无文件"}
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto border-r border-gray-800">
      {items.map((file) => {
        const fid =
          "file_id" in file
            ? file.file_id
            : (file as FileChunk).file_id;
        return (
          <FileItem
            key={fid}
            file={file as SearchResult}
            selected={selectedId === fid}
            onClick={() => onSelect(file as SearchResult)}
          />
        );
      })}
    </div>
  );
}
