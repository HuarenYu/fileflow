import { api } from "../../lib/tauri";

function formatSize(b: number) {
  if (b < 1024) return `${b} B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1048576).toFixed(1)} MB`;
}

export function MetadataPreview({
  path,
  size,
  modifiedAt,
}: {
  path: string;
  size: number;
  modifiedAt: number;
}) {
  return (
    <div className="p-6 space-y-3 text-sm text-gray-300">
      <p>
        <span className="text-gray-500">路径：</span>
        {path}
      </p>
      <p>
        <span className="text-gray-500">大小：</span>
        {formatSize(size)}
      </p>
      <p>
        <span className="text-gray-500">修改时间：</span>
        {new Date(modifiedAt).toLocaleString()}
      </p>
      <button
        onClick={() => api.openFile(path)}
        className="mt-4 px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-white text-sm"
      >
        用默认程序打开
      </button>
    </div>
  );
}
