import { useEffect, useState } from "react";
import { api } from "../../lib/tauri";
import type { SearchResult } from "../../lib/types";
import { PdfPreview } from "./PdfPreview";
import { ImagePreview } from "./ImagePreview";
import { VideoPreview } from "./VideoPreview";
import { TextPreview } from "./TextPreview";
import { OfficePreview } from "./OfficePreview";
import { MetadataPreview } from "./MetadataPreview";

interface Props {
  file: SearchResult | null;
}

export function PreviewPanel({ file }: Props) {
  const [preview, setPreview] = useState<unknown>(null);

  useEffect(() => {
    if (!file) {
      setPreview(null);
      return;
    }
    api.getPreview(file.path).then(setPreview).catch(console.error);
  }, [file?.path]);

  if (!file) {
    return (
      <div className="w-80 flex items-center justify-center text-gray-600 text-sm border-l border-gray-800">
        选择文件以预览
      </div>
    );
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const p = preview as any;
  return (
    <div className="w-80 border-l border-gray-800 overflow-hidden flex flex-col">
      <div className="px-3 py-2 border-b border-gray-800 text-xs text-gray-400 truncate">
        {file.name}
      </div>
      <div className="flex-1 overflow-auto">
        {!p && (
          <div className="flex items-center justify-center h-full text-gray-600 text-sm">
            加载中...
          </div>
        )}
        {p?.type === "pdf" && <PdfPreview path={p.path} />}
        {p?.type === "image" && <ImagePreview path={p.path} />}
        {p?.type === "video" && <VideoPreview path={p.path} />}
        {p?.type === "text" && (
          <TextPreview content={p.content} language={p.language} />
        )}
        {p?.type === "office_images" && <OfficePreview data={p} />}
        {p?.type === "office_fallback" && <OfficePreview data={p} />}
        {p?.type === "metadata" && (
          <MetadataPreview
            path={p.path}
            size={p.size}
            modifiedAt={p.modified_at}
          />
        )}
      </div>
    </div>
  );
}
