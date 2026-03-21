use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreviewData {
    Pdf { path: String },
    Image { path: String },
    Video { path: String },
    Text { content: String, language: String },
    OfficeImages { image_paths: Vec<String> },
    OfficeFallback { text: String },
    Metadata { path: String, size: i64, modified_at: i64 },
}

/// Detects LibreOffice on PATH or common install locations.
pub fn libreoffice_available() -> bool {
    which::which("libreoffice").is_ok()
        || which::which("soffice").is_ok()
        || std::path::Path::new("/Applications/LibreOffice.app/Contents/MacOS/soffice").exists()
        || std::path::Path::new("C:\\Program Files\\LibreOffice\\program\\soffice.exe").exists()
}

pub fn preview(path: &Path, cache_dir: &Path) -> Result<PreviewData> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "pdf" => Ok(PreviewData::Pdf {
            path: path.to_string_lossy().into(),
        }),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "heic" => {
            Ok(PreviewData::Image {
                path: path.to_string_lossy().into(),
            })
        }
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v" => Ok(PreviewData::Video {
            path: path.to_string_lossy().into(),
        }),
        "txt" | "md" | "json" | "yaml" | "toml" | "xml" | "html" | "css" | "js" | "ts" | "py"
        | "rs" | "go" | "sh" => {
            let content = std::fs::read_to_string(path)?;
            Ok(PreviewData::Text {
                content: content.chars().take(10_000).collect(),
                language: ext,
            })
        }
        "docx" | "xlsx" | "pptx" | "xls" | "doc" | "ppt" => {
            if libreoffice_available() {
                convert_office_to_images(path, cache_dir)
            } else {
                let text = crate::extractor::extract_text(path).unwrap_or_default();
                Ok(PreviewData::OfficeFallback {
                    text: text.chars().take(5_000).collect(),
                })
            }
        }
        _ => {
            let meta = std::fs::metadata(path)?;
            Ok(PreviewData::Metadata {
                path: path.to_string_lossy().into(),
                size: meta.len() as i64,
                modified_at: meta
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as i64
                    })
                    .unwrap_or(0),
            })
        }
    }
}

fn convert_office_to_images(path: &Path, cache_dir: &Path) -> Result<PreviewData> {
    std::fs::create_dir_all(cache_dir)?;
    let lo_bin = if which::which("libreoffice").is_ok() {
        "libreoffice"
    } else {
        "soffice"
    };
    std::process::Command::new(lo_bin)
        .args([
            "--headless",
            "--convert-to",
            "png",
            "--outdir",
            cache_dir.to_str().unwrap(),
            path.to_str().unwrap(),
        ])
        .output()?;
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let images: Vec<String> = (0..20)
        .filter_map(|i| {
            let p = if i == 0 {
                cache_dir.join(format!("{stem}.png"))
            } else {
                cache_dir.join(format!("{stem}{i}.png"))
            };
            if p.exists() {
                Some(p.to_string_lossy().into())
            } else {
                None
            }
        })
        .collect();
    Ok(PreviewData::OfficeImages { image_paths: images })
}
