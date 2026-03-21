import { useIndexProgress } from "../hooks/useIndexProgress";

export function StatusBar() {
  const { total, indexed, failed, is_running } = useIndexProgress();
  const pct = total > 0 ? Math.round((indexed / total) * 100) : 0;

  return (
    <footer className="h-7 bg-gray-900 border-t border-gray-700 flex items-center px-3 gap-4 text-xs text-gray-400">
      {is_running ? (
        <>
          <span className="animate-pulse text-blue-400">● 索引中</span>
          <span>
            {indexed} / {total} 文件
          </span>
          <div className="flex-1 max-w-32 h-1.5 bg-gray-700 rounded">
            <div
              className="h-full bg-blue-500 rounded"
              style={{ width: `${pct}%` }}
            />
          </div>
        </>
      ) : (
        <span>
          {indexed} 个文件已索引
          {failed > 0 ? ` · ${failed} 个失败` : ""}
        </span>
      )}
    </footer>
  );
}
