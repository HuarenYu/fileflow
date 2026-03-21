pub mod ocr;
pub mod office;
pub mod pdf;
pub mod text;
pub mod video;

use anyhow::Result;
use std::path::Path;

/// Returns extracted text for the given file path.
/// Returns empty string for unsupported or binary-only files.
pub fn extract_text(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => pdf::extract(path),
        "docx" => office::extract_docx(path),
        "xlsx" | "xls" | "csv" => office::extract_xlsx(path),
        "pptx" => office::extract_pptx(path),
        "txt" | "md" | "rst" | "log" | "json" | "yaml" | "yml" | "toml" | "xml" | "html"
        | "css" | "js" | "ts" | "tsx" | "jsx" | "py" | "rs" | "go" | "java" | "c" | "cpp"
        | "h" | "sh" | "bat" | "ps1" | "rb" | "php" | "swift" | "kt" | "scala" => {
            text::extract(path)
        }
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" => ocr::extract(path),
        _ => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_text_extraction_dispatch() {
        let mut f = NamedTempFile::with_suffix(".txt").unwrap();
        f.write_all(b"hello extraction").unwrap();
        let result = extract_text(f.path()).unwrap();
        assert!(result.contains("hello extraction"));
    }

    #[test]
    fn test_unsupported_extension_returns_empty() {
        let f = NamedTempFile::with_suffix(".xyz").unwrap();
        let result = extract_text(f.path()).unwrap();
        assert!(result.is_empty());
    }
}
