import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/tauri";
import type { Category } from "../lib/types";

const CATEGORIES: { id: Category; label: string; icon: string }[] = [
  { id: "all", label: "全部文件", icon: "📁" },
  { id: "document", label: "文档", icon: "📄" },
  { id: "spreadsheet", label: "表格", icon: "📊" },
  { id: "image", label: "图片", icon: "🖼" },
  { id: "video", label: "视频", icon: "🎬" },
  { id: "audio", label: "音频", icon: "🎵" },
  { id: "code", label: "代码", icon: "💻" },
  { id: "archive", label: "压缩包", icon: "📦" },
  { id: "other", label: "其他", icon: "📎" },
];

interface Props {
  selected: Category;
  onSelect: (cat: Category) => void;
}

export function Sidebar({ selected, onSelect }: Props) {
  async function handleAddDirectory() {
    const dir = await open({ directory: true, multiple: false });
    if (dir) await api.addDirectory(dir as string);
  }

  return (
    <aside className="w-44 bg-gray-900 flex flex-col h-full border-r border-gray-700">
      <div className="p-3 font-bold text-blue-400 text-sm tracking-wide">
        FileFlow
      </div>
      <nav className="flex-1 px-2 space-y-0.5">
        {CATEGORIES.map((cat) => (
          <button
            key={cat.id}
            onClick={() => onSelect(cat.id)}
            className={`w-full text-left px-3 py-1.5 rounded text-sm flex items-center gap-2 transition-colors ${
              selected === cat.id
                ? "bg-blue-600 text-white"
                : "text-gray-300 hover:bg-gray-800"
            }`}
          >
            <span>{cat.icon}</span>
            <span>{cat.label}</span>
          </button>
        ))}
      </nav>
      <div className="p-2 border-t border-gray-700">
        <button
          data-testid="add-directory"
          onClick={handleAddDirectory}
          className="w-full text-xs text-gray-400 hover:text-white py-1.5 px-3 rounded hover:bg-gray-800 text-left"
        >
          + 添加目录
        </button>
      </div>
    </aside>
  );
}
