use anyhow::Result;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    // cap at 50_000 chars to avoid feeding huge files entirely
    Ok(content.chars().take(50_000).collect())
}
