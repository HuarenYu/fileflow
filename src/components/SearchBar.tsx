interface Props {
  query: string;
  onQuery: (q: string) => void;
}

export function SearchBar({ query, onQuery }: Props) {
  return (
    <header className="bg-gray-900 border-b border-gray-700 px-4 py-2.5">
      <div className="flex items-center gap-2 bg-gray-800 rounded-lg px-3 py-1.5">
        <span className="text-gray-400">🔍</span>
        <input
          type="text"
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          placeholder="搜索文件，支持自然语言..."
          className="flex-1 bg-transparent text-sm text-white placeholder-gray-500 outline-none"
        />
        {query && (
          <button
            onClick={() => onQuery("")}
            className="text-gray-500 hover:text-gray-300 text-xs"
          >
            ✕
          </button>
        )}
      </div>
    </header>
  );
}
