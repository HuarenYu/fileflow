import { useEffect, useState } from "react";
import { api } from "../lib/tauri";
import type { FileChunk, Category } from "../lib/types";

export function useFiles(category: Category) {
  const [files, setFiles] = useState<FileChunk[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setLoading(true);
    api
      .listFiles(category === "all" ? undefined : category)
      .then(setFiles)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [category]);

  return { files, loading };
}
