export function TextPreview({
  content,
}: {
  content: string;
  language: string;
}) {
  return (
    <pre className="p-4 text-xs text-gray-300 overflow-auto h-full leading-relaxed whitespace-pre-wrap font-mono">
      {content}
    </pre>
  );
}
