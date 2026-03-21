use anyhow::Result;
use lopdf::Document;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let doc = Document::load(path)?;
    let mut text = String::new();
    for page_id in doc.page_iter() {
        if let Ok(page_text) = doc.extract_text(&[page_id.0]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }
    Ok(text.chars().take(50_000).collect())
}
