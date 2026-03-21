export function OfficePreview({
  data,
}: {
  data:
    | { type: "office_images"; image_paths: string[] }
    | { type: "office_fallback"; text: string };
}) {
  if (data.type === "office_images") {
    return (
      <div className="overflow-y-auto h-full p-4 space-y-4">
        {data.image_paths.map((p, i) => (
          <img
            key={i}
            src={`tauri://localhost/${p}`}
            alt={`Page ${i + 1}`}
            className="max-w-full rounded shadow"
          />
        ))}
      </div>
    );
  }
  return (
    <pre className="p-4 text-xs text-gray-300 whitespace-pre-wrap font-mono overflow-auto h-full">
      {data.text}
    </pre>
  );
}
