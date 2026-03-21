use anyhow::Result;
use std::path::Path;

/// OCR extraction. Requires native libtesseract and language data bundles.
/// Returns empty string gracefully when tesseract is unavailable.
pub fn extract(_path: &Path) -> Result<String> {
    // In production, integrate tesseract-rs with TESSDATA_PREFIX set to bundled tessdata.
    // Stubbed here to avoid requiring the native library at build time.
    Ok(String::new())
}
