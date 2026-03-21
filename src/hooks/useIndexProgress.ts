import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/tauri";
import type { IndexStatus } from "../lib/types";

export function useIndexProgress() {
  const [status, setStatus] = useState<IndexStatus>({
    total: 0,
    indexed: 0,
    failed: 0,
    is_running: false,
  });

  useEffect(() => {
    api.getIndexStatus().then(setStatus).catch(console.error);
    const unlisten = listen<IndexStatus>("index_progress", (e) => {
      setStatus(e.payload);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return status;
}
