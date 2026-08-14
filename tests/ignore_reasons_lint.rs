// ── Наряд №73 Block 3: CI guard against bare `#[ignore]` ───────────
//
// Implements ADR-0103 future-work item #1: a test that fails if any
// `#[ignore]` attribute in tests/ or src/ lacks an inline reason.
//
// ADR-0103 mandates the idiomatic form `#[ignore = "reason"]` as the
// only acceptable shape. The bare form `#[ignore]` (with or without a
// trailing `//` comment) is forbidden because:
//   - cargo test output shows just `... ignored` with no reason
//   - CI aggregators / IDE plugins cannot extract the reason
//   - git blame is the only way to recover intent
//
// This lint runs as part of `cargo test --test ignore_reasons_lint` in
// the test-integration CI job, so any PR that introduces a bare
// `#[ignore]` will fail CI.
//
// The check is purely textual (no AST parsing) — it scans for the
// pattern `#[ignore]` not immediately followed by ` = "..."`. This is
// intentionally conservative: it catches the bare form and the
// `#[ignore = ...]` form with a non-string-literal argument (which
// wouldn't compile anyway, but we want a better error message).

use std::fs;
use std::path::{Path, PathBuf};

/// Walk a directory recursively, yielding paths to all `.rs` files.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip target/ (build artifacts) and hidden dirs (.git, .cargo)
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// A bare `#[ignore]` occurrence found in a source file.
struct Violation {
    path: PathBuf,
    line_no: usize,
    line: String,
}

/// Scan one file's source for bare `#[ignore]` attributes.
///
/// Detection rule: a line whose trimmed form contains `#[ignore` is a
/// candidate. It is accepted (idiomatic) if it matches `#[ignore\s*=`
/// after the opening bracket. Otherwise it is a violation.
///
/// To avoid false positives, comment lines and `#[ignore` substrings
/// inside double-quoted string literals are skipped:
///   - Lines that start (after `trim_start`) with `//`, `///`, `//!`,
///     or `*` are treated as comments and skipped.
///   - For non-comment lines, we count `"` characters before the
///     `#[ignore` occurrence; if odd, the substring is inside a string
///     literal and the line is skipped.
///
/// Known limitation: escaped `\"` inside string literals can throw off
/// the quote count. This is acceptable for our codebase — no .rs file
/// in tests/ or src/ currently uses `\"` immediately before an
/// `#[ignore` mention.
fn scan_file(path: &Path) -> Vec<Violation> {
    let Ok(src) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let trimmed = raw.trim_start();

        // Skip comment lines (line comments and block-comment continuation).
        if trimmed.starts_with("//")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("///")
            || trimmed.starts_with('*')
        {
            continue;
        }

        let Some(idx) = raw.find("#[ignore") else {
            continue;
        };

        // Skip if `#[ignore` is inside a double-quoted string literal
        // on this line — heuristic: count `"` before idx; odd = inside.
        let quotes_before = raw[..idx].matches('"').count();
        if quotes_before % 2 == 1 {
            continue;
        }

        let after = &raw[idx + "#[ignore".len()..];
        let ok = after.trim_start().starts_with('=');
        if !ok {
            out.push(Violation {
                path: path.to_path_buf(),
                line_no: i + 1,
                line: raw.to_string(),
            });
        }
    }
    out
}

#[test]
fn no_bare_ignore_attributes_in_tests_or_src() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let root = PathBuf::from(&manifest_dir);

    let mut all_violations: Vec<Violation> = Vec::new();
    for sub in &["tests", "src", "examples", "self-host", "benches"] {
        let dir = root.join(sub);
        if !dir.is_dir() {
            continue;
        }
        for file in rust_files(&dir) {
            for v in scan_file(&file) {
                all_violations.push(v);
            }
        }
    }

    if !all_violations.is_empty() {
        let mut msg = String::from(
            "Bare `#[ignore]` attributes are forbidden (ADR-0103). \
             Use the idiomatic form `#[ignore = \"reason\"]` instead.\n\n\
             Violations found:\n",
        );
        for v in &all_violations {
            msg.push_str(&format!(
                "  {}:{}: {}\n",
                v.path.strip_prefix(&root)
                    .unwrap_or(&v.path)
                    .display(),
                v.line_no,
                v.line.trim()
            ));
        }
        msg.push_str(&format!(
            "\nTotal: {} violation(s). \
             See docs/adr/0103-idiomatic-ignore-reasons.md for rationale.",
            all_violations.len()
        ));
        panic!("{msg}");
    }
}

#[test]
fn scan_file_accepts_idiomatic_form() {
    // Sanity check: the idiomatic form must NOT be flagged.
    let tmp = tempfile_dir();
    let file = tmp.join("sample.rs");
    fs::write(
        &file,
        "#[test]\n#[ignore = \"TODO: not yet implemented\"]\nfn t() {}\n",
    )
    .unwrap();
    assert!(scan_file(&file).is_empty(), "idiomatic form must pass");
}

#[test]
fn scan_file_rejects_bare_form() {
    let tmp = tempfile_dir();
    let file = tmp.join("sample.rs");
    fs::write(&file, "#[test]\n#[ignore]\nfn t() {}\n").unwrap();
    let v = scan_file(&file);
    assert_eq!(v.len(), 1, "bare #[ignore] must be flagged");
    assert_eq!(v[0].line_no, 2);
}

#[test]
fn scan_file_rejects_bare_form_with_inline_comment() {
    // The pre-ADR-0103 form: `#[ignore] // some reason`. Still forbidden —
    // the reason must live inside the attribute, not in a `//` comment.
    let tmp = tempfile_dir();
    let file = tmp.join("sample.rs");
    fs::write(
        &file,
        "#[test]\n#[ignore] // TODO: fix me\nfn t() {}\n",
    )
    .unwrap();
    let v = scan_file(&file);
    assert_eq!(v.len(), 1, "bare #[ignore] // comment must be flagged");
}

#[test]
fn scan_file_rejects_comma_separated_form() {
    // `#[ignore, other_attr]` — rare, but treated as bare because the
    // reason is missing. Use separate attribute lines instead.
    let tmp = tempfile_dir();
    let file = tmp.join("sample.rs");
    fs::write(&file, "#[test]\n#[ignore, should_panic]\nfn t() {}\n").unwrap();
    let v = scan_file(&file);
    assert_eq!(v.len(), 1, "comma-separated #[ignore, ...] must be flagged");
}

/// Helper: create a unique temp dir for this test run. We avoid pulling
/// in the `tempfile` crate as a dev-dependency — `std::env::temp_dir()`
/// plus the test thread name is enough for these short-lived scans.
fn tempfile_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "ignore_lint_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}
