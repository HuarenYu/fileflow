use std::path::Path;

pub fn classify(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" | "docx" | "doc" | "txt" | "md" | "rst" | "rtf" | "odt" => "document",
        "xlsx" | "xls" | "csv" | "ods" => "spreadsheet",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "svg" | "heic" => "image",
        "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" => "video",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => "audio",
        "py" | "rs" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "c" | "cpp" | "h" | "cs"
        | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bat" => "code",
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "archive",
        "exe" | "dmg" | "pkg" | "msi" | "deb" | "rpm" | "appimage" => "installer",
        "ttf" | "otf" | "woff" | "woff2" | "eot" => "font",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_classify_pdf() {
        assert_eq!(classify(Path::new("report.pdf")), "document");
    }

    #[test]
    fn test_classify_xlsx() {
        assert_eq!(classify(Path::new("data.xlsx")), "spreadsheet");
    }

    #[test]
    fn test_classify_mp4() {
        assert_eq!(classify(Path::new("video.mp4")), "video");
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(classify(Path::new("file.xyz")), "other");
    }
}
