use anyhow::Result;
use std::path::Path;

/// Returns metadata string (duration, resolution) for vector indexing.
pub fn extract_metadata(path: &Path) -> Result<String> {
    Ok(format!(
        "video filename:{}",
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ))
}

/// Extracts a thumbnail at ~1s mark and saves to output_path.
pub fn extract_thumbnail(path: &Path, output_path: &Path) -> Result<()> {
    // Use ffmpeg CLI via std::process::Command for simplicity
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            path.to_str().unwrap(),
            "-ss",
            "00:00:01",
            "-vframes",
            "1",
            "-vf",
            "scale=320:-1",
            output_path.to_str().unwrap(),
        ])
        .output()?;
    if !status.status.success() {
        anyhow::bail!("ffmpeg thumbnail failed");
    }
    Ok(())
}
