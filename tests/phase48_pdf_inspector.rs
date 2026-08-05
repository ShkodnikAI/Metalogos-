// ── Integration tests for PDF builtins (Наряд №48) ───────────────────
// Tests pdf_classify and pdf_to_markdown against real PDF fixture files.

#[cfg(test)]
mod tests {
    use metalogos::builtins::pdf::{builtin_pdf_classify, builtin_pdf_to_markdown};
    use metalogos::interpreter::Value;

    /// Helper: extract a field from a dict (List of key-value pairs).
    fn dict_get(dict: &Value, key: &str) -> Option<Value> {
        if let Value::List(items) = dict {
            for i in 0..items.len() / 2 {
                if let Value::String(k) = &items[i * 2] {
                    if k == key {
                        return Some(items[i * 2 + 1].clone());
                    }
                }
            }
        }
        None
    }

    /// Helper: get string value from dict.
    fn dict_str(dict: &Value, key: &str) -> String {
        match dict_get(dict, key) {
            Some(Value::String(s)) => s,
            other => format!("{:?}", other),
        }
    }

    /// Helper: get float value from dict.
    fn dict_float(dict: &Value, key: &str) -> f64 {
        match dict_get(dict, key) {
            Some(Value::Float(f)) => f,
            Some(Value::String(s)) => s.parse().unwrap_or(0.0),
            other => {
                eprintln!("dict_float('{}') got unexpected: {:?}", key, other);
                0.0
            }
        }
    }

    #[test]
    fn test_pdf_classify_text_based() {
        let result =
            builtin_pdf_classify(&[Value::String("tests/fixtures/text_based.pdf".to_string())])
                .expect("pdf_classify should succeed on text_based.pdf");

        let pdf_type = dict_str(&result, "type");
        let confidence = dict_float(&result, "confidence");
        let page_count = dict_float(&result, "page_count") as u32;

        assert_eq!(page_count, 1, "text_based.pdf should have 1 page");
        // The fixture is a valid text PDF — should be classified as TextBased or Mixed
        assert!(
            pdf_type == "TextBased" || pdf_type == "Mixed",
            "Expected TextBased or Mixed, got: {}",
            pdf_type
        );
        // Confidence should be reasonable (> 0 is the minimum check)
        assert!(
            confidence > 0.0,
            "Confidence should be > 0, got: {}",
            confidence
        );
    }

    #[test]
    fn test_pdf_to_markdown_text_based() {
        let result =
            builtin_pdf_to_markdown(&[Value::String("tests/fixtures/text_based.pdf".to_string())])
                .expect("pdf_to_markdown should succeed on text_based.pdf");

        let markdown = dict_str(&result, "markdown");
        let page_count = dict_float(&result, "page_count") as u32;
        let pdf_type = dict_str(&result, "pdf_type");

        assert_eq!(page_count, 1, "text_based.pdf should have 1 page");
        assert!(
            !markdown.is_empty(),
            "Markdown should not be empty for text-based PDF"
        );
        // The markdown should contain our test fixture text
        assert!(
            markdown.contains("Hello") || markdown.contains("Metalogos"),
            "Markdown should contain text from the PDF. Got: {}",
            &markdown[..markdown.len().min(200)]
        );
        assert!(
            pdf_type == "TextBased" || pdf_type == "Mixed",
            "Expected TextBased or Mixed, got: {}",
            pdf_type
        );
    }

    #[test]
    fn test_pdf_classify_scanned() {
        let result =
            builtin_pdf_classify(&[Value::String("tests/fixtures/scanned.pdf".to_string())])
                .expect("pdf_classify should succeed on scanned.pdf");

        let pdf_type = dict_str(&result, "type");
        let page_count = dict_float(&result, "page_count") as u32;

        assert_eq!(page_count, 1, "scanned.pdf should have 1 page");
        // The scanned fixture has only an image, no text operators
        assert!(
            pdf_type == "Scanned" || pdf_type == "ImageBased" || pdf_type == "Mixed",
            "Scanned PDF should be Scanned/ImageBased/Mixed, got: {}",
            pdf_type
        );
    }

    #[test]
    fn test_pdf_to_markdown_scanned() {
        let result =
            builtin_pdf_to_markdown(&[Value::String("tests/fixtures/scanned.pdf".to_string())])
                .expect("pdf_to_markdown should succeed on scanned.pdf");

        let pdf_type = dict_str(&result, "pdf_type");
        let markdown = dict_str(&result, "markdown");

        // The scanned fixture has only an image XObject (no text operators).
        // pdf-inspector may classify it as any type — the key point is that
        // there should be little to no extractable text content.
        let trimmed = markdown.trim();
        assert!(
            trimmed.is_empty() || trimmed.len() < 50,
            "Image-only PDF should have minimal text. Got: {} chars: '{}'",
            trimmed.len(),
            &trimmed[..trimmed.len().min(100)]
        );
        // pdf_type should be one of the known types (not empty/error)
        assert!(
            !pdf_type.is_empty() && pdf_type != "None",
            "pdf_type should be present, got: '{}'",
            pdf_type
        );
    }

    #[test]
    fn test_pdf_to_markdown_returns_all_fields() {
        let result =
            builtin_pdf_to_markdown(&[Value::String("tests/fixtures/text_based.pdf".to_string())])
                .expect("pdf_to_markdown should succeed");

        // Verify all expected keys are present
        for key in &[
            "markdown",
            "page_count",
            "pdf_type",
            "has_tables",
            "pages_needing_ocr",
            "confidence",
            "processing_time_ms",
        ] {
            assert!(
                dict_get(&result, key).is_some(),
                "Missing key '{}' in result",
                key
            );
        }

        // processing_time_ms should be reasonable (< 10 seconds)
        let time_ms = dict_float(&result, "processing_time_ms");
        assert!(
            time_ms < 10_000.0,
            "Processing should be < 10s, took: {}ms",
            time_ms
        );
    }
}
