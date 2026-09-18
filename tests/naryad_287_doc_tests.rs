// ── tests/naryad_287_doc_tests.rs — Наряд №287 (P2, testing/docs) ───────
//
// Контракт (issue #342): mlog test --docs — rustdoc doc-tests-паттерн.
//   1. fixture-doc с блоками всех видов (зелёный, expect, expect-error,
//      no-run, сломанный — красный с ТОЧНЫМ якорем);
//   2. вложенные фенсы / фенсы без языка / ```json — пропуски с подсчётом;
//   3. паритет бэкендов: --backend vm исполняет блоки через компилируемый
//      путь (vm-compile skip на экспериментальных возможностях ADR-0105);
//   4. read-only профиль: сетевые/exec-заглушки громко, ephemeral cwd.
//
// Запуск: cargo test --test naryad_287_doc_tests

use metalogos::doc_tests::{extract_doc_blocks, run_doc_tests, DocBackend, DocReport};
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL_LOCK: Mutex<()> = Mutex::new(());

fn with_lock(body: impl FnOnce()) {
    let _g = SERIAL_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    body();
}

fn write_fixture(name: &str, markdown: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = dir.path().join(name);
    std::fs::write(&p, markdown).expect("write fixture");
    (dir, p)
}

const GOOD_DOC: &str = r#"
# Guide

## Simple block

```mlog
let s = upper("doc-test")
```

## Flow with expect

```mlog
pattern Shout(x: String) -> String { return upper(x) }
flow Main { input: String = "abc" -> Shout -> output }
// expect: ABC
```

## Expected error

```mlog
let x = 5.0
x = 10.0
// expect-error
```

## Expected error with code

```mlog
let y = 1.0
y = 2.0
// expect-error: cannot assign
```

## No-run (parse only)

```mlog
// no-run
let resp = http_post("https://example.com/api", "data")
```

## Skipped (grammar sheet)

```mlog
// doc-test: skip
pattern Name(param: Type, ...) -> ReturnType {
  ...
}
```
"#;

const BROKEN_DOC: &str = r#"
# Broken

```mlog
pattern Boom(x: String) -> String { return undefined_function(x) }
flow Main { input: String = "a" -> Boom -> output }
```
"#;

const LANGS_DOC: &str = r#"
# Languages

```text
not mlog — skipped by language
```

```json
{"skip": "me"}
```

```
no language at all
```

```mlog
let executed = 1.0
```
"#;

const NESTED_DOC: &str = r#"
# Nested

```mlog
let s = "fence-like inside a string: ``` is fine when quoted"
// тройные бэктики внутри mlog-строки — не фенс
```
"#;

#[test]
fn extraction_counts_and_anchors() {
    let blocks = extract_doc_blocks(GOOD_DOC, "GOOD.md");
    assert_eq!(blocks.len(), 6, "6 mlog blocks: {:?}", blocks.len());
    assert_eq!(blocks[0].section, "Simple block");
    assert_eq!(blocks[1].section, "Flow with expect");
    assert_eq!(blocks[0].index, 1);
    assert_eq!(blocks[5].index, 6);
    assert_eq!(blocks[0].file, "GOOD.md");
    // Якорь — семантический.
    assert_eq!(blocks[1].anchor(), "GOOD.md: Flow with expect: block #2");
}

#[test]
fn languages_other_than_mlog_are_ignored() {
    let blocks = extract_doc_blocks(LANGS_DOC, "LANGS.md");
    assert_eq!(blocks.len(), 1, "only ```mlog extracted");
    let nested = extract_doc_blocks(NESTED_DOC, "NESTED.md");
    assert_eq!(nested.len(), 1);
}

#[test]
fn fixture_doc_all_block_kinds_pass() {
    with_lock(|| {
        let (dir, p) = write_fixture("GOOD.md", GOOD_DOC);
        let files = vec![p];
        let report = run_doc_tests(&files, DocBackend::Tw, dir.path()).expect("run");
        assert_eq!(report.extracted, 6, "{:?}", report);
        // простой блок + flow-блок исполнены
        assert_eq!(report.executed, 2, "executed: {:?}", report);
        // expect-error ×2
        assert_eq!(report.expect_error, 2, "expect-error: {:?}", report);
        // no-run ×1
        assert_eq!(report.no_run, 1, "no-run: {:?}", report);
        // skip ×1
        assert_eq!(report.skipped, 1, "skip: {:?}", report);
        assert!(
            report.failures.is_empty(),
            "failures: {:?}",
            report.failures
        );
    });
}

#[test]
fn broken_block_fails_with_exact_anchor() {
    with_lock(|| {
        let (dir, p) = write_fixture("BROKEN.md", BROKEN_DOC);
        let files = vec![p];
        let report = run_doc_tests(&files, DocBackend::Tw, dir.path()).expect("run");
        assert_eq!(report.failures.len(), 1, "{:?}", report);
        let f = &report.failures[0];
        assert!(
            f.contains("BROKEN.md: Broken: block #1"),
            "exact semantic anchor required: {f}"
        );
        assert!(f.contains("undefined"), "runtime cause: {f}");
    });
}

#[test]
fn missing_files_are_reported_not_panic() {
    with_lock(|| {
        let files = vec![PathBuf::from("/nonexistent/nope.md")];
        let report = run_doc_tests(&files, DocBackend::Tw, std::path::Path::new(".")).expect("run");
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].contains("cannot read file"));
    });
}

#[test]
fn expect_mismatch_is_a_failure() {
    with_lock(|| {
        let md = r#"
# T

```mlog
pattern Shout(x: String) -> String { return upper(x) }
flow Main { input: String = "abc" -> Shout -> output }
// expect: WRONG
```
"#;
        let (dir, p) = write_fixture("EXPECT.md", md);
        let files = vec![p];
        let report = run_doc_tests(&files, DocBackend::Tw, dir.path()).expect("run");
        assert_eq!(report.failures.len(), 1, "{:?}", report);
        assert!(
            report.failures[0].contains("expect 'WRONG', got 'ABC'"),
            "{:?}",
            report.failures
        );
    });
}

#[test]
fn vm_backend_executes_simple_blocks() {
    with_lock(|| {
        // VM-совместимый блок: полный запуск через компилятор + VM.
        let md = r#"
# VM

```mlog
pattern Shout(x: String) -> String { return upper(x) }
flow Main { input: String = "abc" -> Shout -> output }
// expect: ABC
```
"#;
        let (dir, p) = write_fixture("VM.md", md);
        let files = vec![p];
        let report = run_doc_tests(&files, DocBackend::Vm, dir.path()).expect("run");
        assert_eq!(report.executed, 1, "{:?}", report);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    });
}

#[test]
fn vm_backend_skips_uncompilable_grammar() {
    with_lock(|| {
        // Блок, который TW-парсер берёт, но VM-компилятор не поддерживает
        // (ADR-0105: block if/else в let — не в VM-подмножестве) → skip.
        let md = r#"
# VM-limited

```mlog
each item in items {
  print(item)
}
```
"#;
        let (dir, p) = write_fixture("VML.md", md);
        let files = vec![p];
        let report = run_doc_tests(&files, DocBackend::Vm, dir.path()).expect("run");
        // Либо skip (vm-compile), либо исполнено — но НЕ failure.
        assert!(
            report.failures.is_empty(),
            "vm-compile limitations must skip, not fail: {:?}",
            report.failures
        );
    });
}

#[test]
fn default_files_resolution_and_report_summary() {
    with_lock(|| {
        let root = std::env::current_dir().expect("cwd");
        let files = metalogos::doc_tests::resolve_doc_files(&root, &[]);
        // Дефолт: REFERENCE.md + README.md + docs/book/**/*.md
        assert!(files.iter().any(|f| f.ends_with("REFERENCE.md")));
        assert!(files.iter().any(|f| f.ends_with("README.md")));
        assert!(files
            .iter()
            .any(|f| f.to_string_lossy().contains("docs/book")));
        // ADR/research НЕ в дефолте (исторические записи).
        assert!(
            !files
                .iter()
                .any(|f| f.to_string_lossy().contains("docs/adr")),
            "ADR must not be scanned by default"
        );
        // Явный glob бьёт дефолт.
        let only = metalogos::doc_tests::resolve_doc_files(&root, &["REFERENCE.md".into()]);
        assert_eq!(only.len(), 1);

        let report = DocReport::default();
        assert!(report.summary().contains("0 extracted"));
    });
}

#[test]
fn real_repo_docs_are_green() {
    with_lock(|| {
        // «Сделано, когда»: mlog test --docs зелёный на текущих доках.
        let root = std::env::current_dir().expect("cwd");
        let files = metalogos::doc_tests::resolve_doc_files(&root, &[]);
        let report = run_doc_tests(&files, DocBackend::Tw, &root).expect("run");
        assert!(
            report.failures.is_empty(),
            "repo docs must be green: {:#?}",
            report.failures
        );
        assert!(
            report.extracted > 100,
            "real corpus: {:?}",
            report.summary()
        );
        assert!(report.executed > 50, "real corpus: {:?}", report.summary());
    });
}
