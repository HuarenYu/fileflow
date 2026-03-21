export function ImagePreview({ path }: { path: string }) {
  return (
    <div className="flex items-center justify-center h-full p-4">
      <img
        src={`tauri://localhost/${path}`}
        alt=""
        className="max-w-full max-h-full object-contain rounded"
      />
    </div>
  );
}
