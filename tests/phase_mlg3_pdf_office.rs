// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Наряд MLG-3: Integration tests for PDF office automation ────────────
// Tests: pdf_draw_table, pdf_add_image, pdf_set_page_header/footer,
//        pdf_page_numbers, pdf_watermark, pdf_fill_form,
//        pdf_rotate_page, pdf_delete_pages, pdf_extract_images,
//        html_to_pdf Rust renderer, pdf_merge multi, pdf_split ranges

use metalogos::builtins::{Builtins, BUILTIN_REGISTRY};
use metalogos::interpreter::Value;

/// Helper: call a builtin by name with the given arguments.
fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
    let builtins = Builtins::new();
    let func = builtins
        .get(name)
        .ok_or_else(|| format!("builtin '{}' not found", name))?;
    func(args)
}

/// Helper: extract "id" field from a Value::Struct
fn extract_doc_id(val: &Value) -> String {
    match val {
        Value::Struct { fields, .. } => {
            if let Some(Value::String(id)) = fields.get("id") {
                id.clone()
            } else {
                panic!("no 'id' field in struct")
            }
        }
        _ => panic!("expected Struct, got {}", val.type_name()),
    }
}

// №475: the gate refuses ABSOLUTE paths — the fixtures use RELATIVE
// per-test directories (auto-cleaned on drop; each name unique per call).
struct TmpDirRel(&'static str);
impl TmpDirRel {
    fn new(name: &'static str) -> Self {
        let _ = std::fs::remove_dir_all(name);
        std::fs::create_dir_all(name).unwrap();
        TmpDirRel(name)
    }
    fn p(&self, file: &str) -> String {
        format!("{}/{}", self.0, file)
    }
}
impl Drop for TmpDirRel {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0);
    }
}

#[test]
fn test_pdf_draw_table_basic() {
    let dir = TmpDirRel::new("n475t_po_1");
    let output_path = dir.p("table_test.pdf");

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    call_builtin(
        "pdf_add_page",
        &[
            Value::String(doc_id.clone()),
            Value::Float(595.28),
            Value::Float(841.89),
        ],
    )
    .unwrap();

    call_builtin("pdf_draw_table", &[
        Value::String(doc_id.clone()),
        Value::Float(72.0),
        Value::Float(700.0),
        Value::String("[150,100,200]".to_string()),
        Value::String("[[\"Metric\",\"Q1\",\"Q2\"],[\"Revenue\",\"$1.2M\",\"$1.5M\"],[\"Costs\",\"$0.8M\",\"$0.9M\"]]".to_string()),
    ]).unwrap();

    let save_result = call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(output_path.clone())],
    );
    assert!(save_result.is_ok());

    let file_size = std::fs::metadata(&output_path).unwrap().len();
    assert!(
        file_size > 100,
        "PDF with table should be non-trivial, got {} bytes",
        file_size
    );
}

#[test]
fn test_pdf_draw_table_with_style() {
    let dir = TmpDirRel::new("n475t_po_2");
    let output_path = dir.p("styled_table.pdf");

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    call_builtin(
        "pdf_add_page",
        &[
            Value::String(doc_id.clone()),
            Value::Float(595.28),
            Value::Float(841.89),
        ],
    )
    .unwrap();

    call_builtin("pdf_draw_table", &[
        Value::String(doc_id.clone()),
        Value::Float(72.0),
        Value::Float(700.0),
        Value::String("[100,100]".to_string()),
        Value::String("[[\"A\",\"B\"],[\"1\",\"2\"]]".to_string()),
        Value::String("{\"font\":\"Courier\",\"font_size\":9,\"border\":true,\"header_bg\":\"0.8,0.8,0.8\"}".to_string()),
    ]).unwrap();

    let save_result = call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(output_path.clone())],
    );
    assert!(save_result.is_ok());
}

#[test]
fn test_pdf_set_page_header_footer() {
    let dir = TmpDirRel::new("n475t_po_3");
    let output_path = dir.p("header_footer.pdf");

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    call_builtin(
        "pdf_add_page",
        &[
            Value::String(doc_id.clone()),
            Value::Float(595.28),
            Value::Float(841.89),
        ],
    )
    .unwrap();

    call_builtin(
        "pdf_set_page_header",
        &[
            Value::String(doc_id.clone()),
            Value::String("Metalogos Office Report".to_string()),
        ],
    )
    .unwrap();

    call_builtin(
        "pdf_set_page_footer",
        &[
            Value::String(doc_id.clone()),
            Value::String("Confidential".to_string()),
        ],
    )
    .unwrap();

    call_builtin(
        "pdf_write_text",
        &[
            Value::String(doc_id.clone()),
            Value::Float(72.0),
            Value::Float(700.0),
            Value::String("Test content".to_string()),
        ],
    )
    .unwrap();

    let save_result = call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(output_path.clone())],
    );
    assert!(save_result.is_ok());
    assert!(std::fs::metadata(&output_path).unwrap().len() > 100);
}

#[test]
fn test_pdf_page_numbers() {
    let dir = TmpDirRel::new("n475t_po_4");
    let output_path = dir.p("page_numbers.pdf");

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    // Add two pages
    for _ in 0..2 {
        call_builtin(
            "pdf_add_page",
            &[
                Value::String(doc_id.clone()),
                Value::Float(595.28),
                Value::Float(841.89),
            ],
        )
        .unwrap();
    }

    call_builtin(
        "pdf_page_numbers",
        &[
            Value::String(doc_id.clone()),
            Value::String("N of M".to_string()),
        ],
    )
    .unwrap();

    let save_result = call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(output_path.clone())],
    );
    assert!(save_result.is_ok());
}

#[test]
fn test_pdf_watermark() {
    let dir = TmpDirRel::new("n475t_po_5");
    let output_path = dir.p("watermark.pdf");

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    call_builtin(
        "pdf_add_page",
        &[
            Value::String(doc_id.clone()),
            Value::Float(595.28),
            Value::Float(841.89),
        ],
    )
    .unwrap();

    call_builtin(
        "pdf_watermark",
        &[
            Value::String(doc_id.clone()),
            Value::String("DRAFT".to_string()),
            Value::String("Helvetica-Bold".to_string()),
            Value::Float(60.0),
            Value::Float(0.3),
        ],
    )
    .unwrap();

    let save_result = call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(output_path.clone())],
    );
    assert!(save_result.is_ok());

    // Verify PDF is valid
    let bytes = std::fs::read(&output_path).unwrap();
    assert_eq!(&bytes[0..4], b"%PDF");
}

#[test]
fn test_pdf_rotate_page() {
    // Create a basic PDF first, then rotate
    let dir = TmpDirRel::new("n475t_po_6");
    let input_path = dir.p("input.pdf");
    let output_path = dir.p("rotated.pdf");

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    call_builtin(
        "pdf_add_page",
        &[
            Value::String(doc_id.clone()),
            Value::Float(595.28),
            Value::Float(841.89),
        ],
    )
    .unwrap();

    call_builtin(
        "pdf_write_text",
        &[
            Value::String(doc_id.clone()),
            Value::Float(72.0),
            Value::Float(700.0),
            Value::String("Test".to_string()),
        ],
    )
    .unwrap();

    call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(input_path.clone())],
    )
    .unwrap();

    // Rotate page 1 by 90 degrees
    let result = call_builtin(
        "pdf_rotate_page",
        &[
            Value::String(input_path.clone()),
            Value::Float(1.0),
            Value::Float(90.0),
            Value::String(output_path.clone()),
        ],
    );
    assert!(result.is_ok());
    assert!(std::fs::metadata(&output_path).unwrap().len() > 0);
}

#[test]
fn test_pdf_delete_pages() {
    let dir = TmpDirRel::new("n475t_po_7");
    let input_path = dir.p("multi_page.pdf");
    let output_path = dir.p("after_delete.pdf");

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    // Create 3-page PDF
    for i in 0..3 {
        call_builtin(
            "pdf_add_page",
            &[
                Value::String(doc_id.clone()),
                Value::Float(595.28),
                Value::Float(841.89),
            ],
        )
        .unwrap();
        call_builtin(
            "pdf_write_text",
            &[
                Value::String(doc_id.clone()),
                Value::Float(72.0),
                Value::Float(700.0),
                Value::String(format!("Page {}", i + 1)),
            ],
        )
        .unwrap();
    }

    call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(input_path.clone())],
    )
    .unwrap();

    // Delete page 2
    let result = call_builtin(
        "pdf_delete_pages",
        &[
            Value::String(input_path.clone()),
            Value::String("[2]".to_string()),
            Value::String(output_path.clone()),
        ],
    );
    assert!(result.is_ok());
}

#[test]
fn test_pdf_extract_images() {
    // Test with non-existent file (should fail gracefully)
    let result = call_builtin(
        "pdf_extract_images",
        &[Value::String("/nonexistent/file.pdf".to_string())],
    );
    assert!(result.is_err());
}

#[test]
fn test_pdf_fill_form() {
    // Test with non-existent file (should fail gracefully)
    let result = call_builtin(
        "pdf_fill_form",
        &[
            Value::String("/nonexistent/form.pdf".to_string()),
            Value::String("{\"name\":\"Alice\"}".to_string()),
            Value::String("/tmp/filled.pdf".to_string()),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_pdf_merge_multi() {
    // Create two PDFs and merge them
    let dir = TmpDirRel::new("n475t_po_8");
    let path1 = dir.p("doc1.pdf");
    let path2 = dir.p("doc2.pdf");
    let merged_path = dir.p("merged.pdf");

    // Create doc1
    let create1 = call_builtin("pdf_create", &[]).unwrap();
    let id1 = extract_doc_id(&create1);
    call_builtin(
        "pdf_add_page",
        &[
            Value::String(id1.clone()),
            Value::Float(595.28),
            Value::Float(841.89),
        ],
    )
    .unwrap();
    call_builtin(
        "pdf_save",
        &[Value::String(id1), Value::String(path1.clone())],
    )
    .unwrap();

    // Create doc2
    let create2 = call_builtin("pdf_create", &[]).unwrap();
    let id2 = extract_doc_id(&create2);
    call_builtin(
        "pdf_add_page",
        &[
            Value::String(id2.clone()),
            Value::Float(595.28),
            Value::Float(841.89),
        ],
    )
    .unwrap();
    call_builtin(
        "pdf_save",
        &[Value::String(id2), Value::String(path2.clone())],
    )
    .unwrap();

    // Merge
    let paths_json = format!("[\"{}\",\"{}\"]", path1.as_str(), path2.as_str());
    let result = call_builtin(
        "pdf_merge",
        &[
            Value::String(paths_json),
            Value::String(merged_path.clone()),
        ],
    );
    assert!(result.is_ok());
}

#[test]
fn test_pdf_split_ranges() {
    // Create a 3-page PDF and split it
    let dir = TmpDirRel::new("n475t_po_9");
    let input_path = dir.p("multi.pdf");
    let output_dir = dir.p("split_output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let create_result = call_builtin("pdf_create", &[]).unwrap();
    let doc_id = extract_doc_id(&create_result);

    for _ in 0..3 {
        call_builtin(
            "pdf_add_page",
            &[
                Value::String(doc_id.clone()),
                Value::Float(595.28),
                Value::Float(841.89),
            ],
        )
        .unwrap();
    }

    call_builtin(
        "pdf_save",
        &[Value::String(doc_id), Value::String(input_path.clone())],
    )
    .unwrap();

    let result = call_builtin(
        "pdf_split",
        &[
            Value::String(input_path.clone()),
            Value::String("[[1,2],[3,3]]".to_string()),
            Value::String(output_dir.clone()),
        ],
    );
    assert!(result.is_ok());
}

#[test]
fn test_html_to_pdf_simple() {
    // Simple HTML that the Rust renderer should handle
    let dir = TmpDirRel::new("n475t_po_10");
    let output_path = dir.p("simple.pdf");

    let html = "<html><body><h1>Title</h1><p>Paragraph text</p></body></html>".to_string();
    let result = call_builtin(
        "html_to_pdf",
        &[Value::String(html), Value::String(output_path.clone())],
    );
    // Should succeed with Rust renderer for simple HTML
    assert!(result.is_ok());
}

#[test]
fn test_registry_mlg3_entries_exist() {
    let mlg3_functions = [
        "pdf_draw_table",
        "pdf_add_image",
        "pdf_set_page_header",
        "pdf_set_page_footer",
        "pdf_page_numbers",
        "pdf_watermark",
        "pdf_fill_form",
        "pdf_rotate_page",
        "pdf_delete_pages",
        "pdf_extract_images",
    ];

    let registry_names: std::collections::HashSet<&str> =
        BUILTIN_REGISTRY.iter().map(|s| s.name).collect();

    for func in &mlg3_functions {
        assert!(
            registry_names.contains(func),
            "MLG-3 function '{}' missing from BUILTIN_REGISTRY",
            func
        );
    }
}
