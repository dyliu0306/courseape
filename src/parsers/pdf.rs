use std::path::Path;

/// Extract all text from a PDF file.
/// Returns the concatenated text content of all pages.
pub fn extract_text(pdf_path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(pdf_path)?;
    let text = pdf_extract::extract_text_from_mem(&bytes)?;
    Ok(text)
}

/// Extract text and save as a .txt file next to the PDF.
/// Returns the path to the saved text file.
pub fn extract_and_save(pdf_path: &Path) -> anyhow::Result<std::path::PathBuf> {
    let text = extract_text(pdf_path)?;
    let txt_path = pdf_path.with_extension("txt");
    std::fs::write(&txt_path, &text)?;
    Ok(txt_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_rejects_nonexistent() {
        let result = extract_text(Path::new("nonexistent.pdf"));
        assert!(result.is_err());
    }
}
