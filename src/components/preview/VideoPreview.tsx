export function VideoPreview({ path }: { path: string }) {
  return (
    <div className="flex items-center justify-center h-full p-4">
      <video
        controls
        className="max-w-full max-h-full rounded"
        src={`tauri://localhost/${path}`}
      />
    </div>
  );
}
