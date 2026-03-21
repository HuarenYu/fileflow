use anyhow::Result;
use std::path::Path;

pub fn extract_docx(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let docx = docx_rs::read_docx(&bytes)?;
    let text: String = docx
        .document
        .children
        .iter()
        .filter_map(|child| {
            if let docx_rs::DocumentChild::Paragraph(p) = child {
                Some(
                    p.children
                        .iter()
                        .filter_map(|pc| {
                            if let docx_rs::ParagraphChild::Run(r) = pc {
                                Some(
                                    r.children
                                        .iter()
                                        .filter_map(|rc| {
                                            if let docx_rs::RunChild::Text(t) = rc {
                                                Some(t.text.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<String>(),
                                )
                            } else {
                                None
                            }
                        })
                        .collect::<String>()
                        + "\n",
                )
            } else {
                None
            }
        })
        .collect();
    Ok(text.chars().take(50_000).collect())
}

pub fn extract_xlsx(path: &Path) -> Result<String> {
    use calamine::{open_workbook_auto, Reader};
    let mut workbook = open_workbook_auto(path)?;
    let mut text = String::new();
    for sheet_name in workbook.sheet_names().to_vec() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            for row in range.rows() {
                let line: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                text.push_str(&line.join("\t"));
                text.push('\n');
            }
        }
    }
    Ok(text.chars().take(50_000).collect())
}

pub fn extract_pptx(path: &Path) -> Result<String> {
    // pptx is a zip; extract slide XML and pull text nodes
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut text = String::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.name().starts_with("ppt/slides/slide") && entry.name().ends_with(".xml") {
            let mut xml = String::new();
            entry.read_to_string(&mut xml)?;
            // naive text extraction: pull content between <a:t> tags
            let mut pos = 0;
            while let Some(start) = xml[pos..].find("<a:t>") {
                let abs_start = pos + start + 5;
                if let Some(end) = xml[abs_start..].find("</a:t>") {
                    text.push_str(&xml[abs_start..abs_start + end]);
                    text.push(' ');
                    pos = abs_start + end + 6;
                } else {
                    break;
                }
            }
            text.push('\n');
        }
    }
    Ok(text.chars().take(50_000).collect())
}
