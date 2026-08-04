// ── PDF builtins (Наряд №48) ──────────────────────────────────────────
// Native PDF classification and markdown extraction via pdf-inspector crate.
// Pure Rust, zero IPC, <200ms on text-based PDFs.

use crate::interpreter::Value;

/// Helper: extract a String argument or return error.
fn expect_string_arg(name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(s.clone()),
        other => Err(format!(
            "{}() expected String as argument {}, got {}",
            name,
            idx + 1,
            other.map(|v| v.type_name()).unwrap_or("none")
        )),
    }
}

/// Helper: convert pdf-inspector PdfType to a Metalogos string.
fn pdf_type_to_string(pt: &pdf_inspector::PdfType) -> String {
    match pt {
        pdf_inspector::PdfType::TextBased => "TextBased",
        pdf_inspector::PdfType::Scanned => "Scanned",
        pdf_inspector::PdfType::ImageBased => "ImageBased",
        pdf_inspector::PdfType::Mixed => "Mixed",
    }
    .to_string()
}

/// Helper: build a Metalogos dict (List of key-value pairs) from key names and values.
fn make_dict(keys: &[&str], values: &[Value]) -> Value {
    let mut items = Vec::with_capacity(keys.len() * 2);
    for (k, v) in keys.iter().zip(values.iter()) {
        items.push(Value::String(k.to_string()));
        items.push(v.clone());
    }
    Value::List(items)
}

/// `pdf_classify(path) → { type, confidence, pages_needing_ocr, page_count }`
///
/// Classify a PDF file: TextBased / Scanned / ImageBased / Mixed.
/// Uses pdf-inspector's lightweight detection (no text extraction).
///
/// # Arguments
/// - `path` (String): file path to the PDF
///
/// # Returns
/// Dict with keys: type, confidence, pages_needing_ocr (list), page_count
pub fn builtin_pdf_classify(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("pdf_classify", args, 0)?;

    let result = pdf_inspector::classify_pdf_mem(
        &std::fs::read(&path)
            .map_err(|e| format!("pdf_classify: failed to read '{}': {}", path, e))?,
    )
    .map_err(|e| format!("pdf_classify: {}", e))?;

    let ocr_pages: Vec<Value> = result
        .pages_needing_ocr
        .iter()
        .map(|p| Value::Float(*p as f64))
        .collect();

    Ok(make_dict(
        &[
            "type",
            "confidence",
            "pages_needing_ocr",
            "page_count",
        ],
        &[
            Value::String(pdf_type_to_string(&result.pdf_type)),
            Value::Float(result.confidence as f64),
            Value::List(ocr_pages),
            Value::Float(result.page_count as f64),
        ],
    ))
}

/// `pdf_to_markdown(path) → { markdown, page_count, pdf_type, has_tables }`
///
/// Full PDF processing pipeline: classify + extract text + convert to Markdown.
/// Uses pdf-inspector's `process_pdf_mem` (default Full mode).
///
/// # Arguments
/// - `path` (String): file path to the PDF
///
/// # Returns
/// Dict with keys: markdown, page_count, pdf_type, has_tables, pages_needing_ocr,
///   confidence, processing_time_ms
pub fn builtin_pdf_to_markdown(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("pdf_to_markdown", args, 0)?;

    let bytes = std::fs::read(&path)
        .map_err(|e| format!("pdf_to_markdown: failed to read '{}': {}", path, e))?;

    let result = pdf_inspector::process_pdf_mem(&bytes)
        .map_err(|e| format!("pdf_to_markdown: {}", e))?;

    let markdown = result.markdown.unwrap_or_default();
    let ocr_pages: Vec<Value> = result
        .pages_needing_ocr
        .iter()
        .map(|p| Value::Float(*p as f64))
        .collect();

    Ok(make_dict(
        &[
            "markdown",
            "page_count",
            "pdf_type",
            "has_tables",
            "pages_needing_ocr",
            "confidence",
            "processing_time_ms",
        ],
        &[
            Value::String(markdown),
            Value::Float(result.page_count as f64),
            Value::String(pdf_type_to_string(&result.pdf_type)),
            Value::Bool(result.layout.is_complex),
            Value::List(ocr_pages),
            Value::Float(result.confidence as f64),
            Value::Float(result.processing_time_ms as f64),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_type_to_string_all_variants() {
        assert_eq!(
            pdf_type_to_string(&pdf_inspector::PdfType::TextBased),
            "TextBased"
        );
        assert_eq!(
            pdf_type_to_string(&pdf_inspector::PdfType::Scanned),
            "Scanned"
        );
        assert_eq!(
            pdf_type_to_string(&pdf_inspector::PdfType::ImageBased),
            "ImageBased"
        );
        assert_eq!(
            pdf_type_to_string(&pdf_inspector::PdfType::Mixed),
            "Mixed"
        );
    }

    #[test]
    fn test_pdf_classify_invalid_path() {
        let result = builtin_pdf_classify(&[Value::String("/nonexistent/file.pdf".to_string())]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("failed to read"), "error: {}", err);
    }

    #[test]
    fn test_pdf_to_markdown_invalid_path() {
        let result =
            builtin_pdf_to_markdown(&[Value::String("/nonexistent/file.pdf".to_string())]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("failed to read"), "error: {}", err);
    }

    #[test]
    fn test_pdf_classify_not_a_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let fake_path = dir.path().join("not_a_pdf.txt");
        std::fs::write(&fake_path, "this is not a PDF").unwrap();

        let result = builtin_pdf_classify(&[Value::String(fake_path.to_string_lossy().to_string())]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("pdf_to_markdown") || err.contains("pdf_classify") || err.contains("NotAPdf") || err.contains("Parse"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_pdf_to_markdown_not_a_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let fake_path = dir.path().join("not_a_pdf.txt");
        std::fs::write(&fake_path, "this is not a PDF").unwrap();

        let result = builtin_pdf_to_markdown(&[Value::String(
            fake_path.to_string_lossy().to_string(),
        )]);
        assert!(result.is_err());
    }

    #[test]
    fn test_pdf_classify_wrong_arg_type() {
        let result = builtin_pdf_classify(&[Value::Float(42.0)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected String"));
    }

    #[test]
    fn test_pdf_to_markdown_wrong_arg_type() {
        let result = builtin_pdf_to_markdown(&[Value::Float(42.0)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected String"));
    }
}
