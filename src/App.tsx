import { useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { FileList } from "./components/FileList";
import { PreviewPanel } from "./components/preview/PreviewPanel";
import { SearchBar } from "./components/SearchBar";
import { StatusBar } from "./components/StatusBar";
import type { Category, SearchResult } from "./lib/types";

export default function App() {
  const [category, setCategory] = useState<Category>("all");
  const [query, setQuery] = useState("");
  const [selectedFile, setSelectedFile] = useState<SearchResult | null>(null);

  return (
    <div className="flex flex-col h-screen bg-gray-950 text-white font-sans">
      <SearchBar query={query} onQuery={setQuery} />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar selected={category} onSelect={setCategory} />
        <FileList
          category={category}
          query={query}
          onSelect={setSelectedFile}
          selectedId={selectedFile?.file_id}
        />
        <PreviewPanel file={selectedFile} />
      </div>
      <StatusBar />
    </div>
  );
}
