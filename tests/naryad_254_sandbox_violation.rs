// ── НАРЯД №254: read_file — soft-failure не маскирует нарушение песочницы ──
//
// Контракт (issue #257, docs/naryads-252-257-security-bugfix.md §254):
// - read_file("нет_такого.txt") → пустая строка (мягкий исход сохранён);
// - read_file("../x") → громкая ошибка со стабильным кодом [SANDBOX_VIOLATION]
//   (ADR-0131) — дефект программы больше не неотличим от опечатки;
// - тот же разбор для write_file / append_file / delete_file;
// - существующие мягкие сценарии не сломаны (позитивный путь целиком).
//
// Билтин-уровень через run_program (как tests/n88_html_render_contract.rs).
// CWD тестов = корень крейта: "Cargo.toml" существует, "../x" — всегда
// нарушение, "нет_такого…" — всегда отсутствует. CWD никем не меняется.

fn eval_expr(src: &str) -> Result<String, String> {
    let full = format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err("eval returned None".to_string()),
        Err(e) => Err(e),
    }
}

// ── Точный контракт наряда ──────────────────────────────────────────

#[test]
fn n254_read_missing_file_returns_empty_string() {
    let out = eval_expr("read_file(\"нет_такого_254.txt\")").expect("soft-failure должен быть Ok");
    assert_eq!(out, "", "файла нет → пустая строка (контракт сохранён)");
}

#[test]
fn n254_read_traversal_is_loud_with_code() {
    let err = eval_expr("read_file(\"../x\")").unwrap_err();
    assert!(
        err.contains("[SANDBOX_VIOLATION]"),
        "нарушение песочницы должно быть громким с кодом, got: {}",
        err
    );
}

#[test]
fn n254_read_absolute_is_loud_with_code() {
    let err = eval_expr("read_file(\"/etc/passwd\")").unwrap_err();
    assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
}

// ── Тот же разбор для write/append/delete ───────────────────────────

#[test]
fn n254_write_traversal_is_loud_with_code() {
    let err = eval_expr("write_file(\"../x\", \"v\")").unwrap_err();
    assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
}

#[test]
fn n254_append_traversal_is_loud_with_code() {
    let err = eval_expr("append_file(\"../x\", \"v\")").unwrap_err();
    assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
}

#[test]
fn n254_delete_missing_soft_traversal_loud() {
    let out =
        eval_expr("delete_file(\"нет_такого_254.txt\")").expect("soft-failure должен быть Ok");
    assert_eq!(out, "", "нет файла → мягко");
    let err = eval_expr("delete_file(\"../x\")").unwrap_err();
    assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
}

// ── Позитивный путь не сломан ───────────────────────────────────────

#[test]
fn n254_positive_roundtrip_unbroken() {
    // write → read → delete → read: полный мягкий контракт в силе.
    let out = eval_expr("write_file(\"n254_tmp.txt\", \"v254\")").expect("write should succeed");
    assert_eq!(out, "ok");
    let out = eval_expr("read_file(\"n254_tmp.txt\")").expect("read should succeed");
    assert_eq!(out, "v254");
    let out = eval_expr("delete_file(\"n254_tmp.txt\")").expect("delete should succeed");
    assert_eq!(out, "ok");
    let out = eval_expr("read_file(\"n254_tmp.txt\")").expect("read after delete is soft");
    assert_eq!(out, "", "после удаления — пустая строка");
}

#[test]
fn n254_read_existing_file_still_works() {
    // Существующий файл (CARGO_MANIFEST_DIR/Cargo.toml при CWD=корень крейта).
    let out = eval_expr("read_file(\"Cargo.toml\")").expect("read existing should succeed");
    assert!(
        out.contains("[package]"),
        "содержимое файла должно читаться"
    );
}
