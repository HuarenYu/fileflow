import { useEffect, useState, useRef } from "react";
import { api } from "../lib/tauri";
import type { SearchResult } from "../lib/types";

export function useSearch(query: string) {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      return;
    }
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      setLoading(true);
      api
        .searchFiles(query)
        .then(setResults)
        .catch(console.error)
        .finally(() => setLoading(false));
    }, 300); // 300ms debounce
    return () => clearTimeout(timer.current);
  }, [query]);

  return { results, loading };
}
