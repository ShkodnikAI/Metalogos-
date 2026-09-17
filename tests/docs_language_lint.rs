// ── Naryad #383: CI gate — all repository documentation is English-only ──
//
// Implements the owner directive (2026-09-17): every `.md` file in the
// repository is written in technical English. This lint is the mechanical
// enforcement: it scans every `.md` file in the repo and flags any file
// whose Cyrillic content crosses the publisher-scan threshold:
//
//   - at least MIN_CYRILLIC_LETTERS Cyrillic letters, AND
//   - more than MIN_SHARE_PERCENT percent of all letters in the file
//     are Cyrillic.
//
// The threshold is deliberately tolerant (small Cyrillic traces such as a
// terminology anchor like «наряд» inside an English glossary, or verbatim
// compiler-output samples, do not trip the gate) while still catching any
// document written in Russian wholesale — the pre-#383 state had 41 such
// files, the largest over 80% Cyrillic by letter share.
//
// Files listed in ALLOWLIST are frozen exceptions where Cyrillic is
// *content*, not documentation language (UTF-8 test data in examples,
// identifier-support demos). Shrinking the allowlist is welcome; growing
// it requires an owner decision (AGENTS.md §3).
//
// The scan runs in the test-integration CI job, so any PR that introduces
// a new `.md` file above the threshold fails CI.

use std::fs;
use std::path::{Path, PathBuf};

/// Files where Cyrillic is content (test data), not documentation prose.
/// Repo-relative paths with forward slashes, sorted. Keep frozen:
/// `docs/adr/0043-unicode-fix.md` is *about* Cyrillic UTF-8 handling —
/// the «Привет» string literals are the subject matter of the document.
const ALLOWLIST: &[&str] = &["docs/adr/0043-unicode-fix.md"];

/// Minimum Cyrillic letters in one file for it to be flagged.
const MIN_CYRILLIC_LETTERS: usize = 20;

/// Minimum share of Cyrillic letters among all letters of the file (%).
const MIN_SHARE_PERCENT: f64 = 2.0;

fn is_cyrillic(c: char) -> bool {
    matches!(c as u32, 0x0400..=0x04FF)
}

/// A `.md` file whose Cyrillic content crosses the threshold.
struct Violation {
    path: PathBuf,
    cyrillic_letters: usize,
    total_letters: usize,
    share_percent: f64,
}

/// Walk a directory recursively, yielding paths to all `.md` files.
///
/// `target/` (build artifacts) and `.git/` are skipped; other hidden
/// directories (e.g. `.github`) ARE scanned — the PR template lives there
/// and is documentation like everything else.
fn md_files(root: &Path) -> Vec<PathBuf> {
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
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == "target" || name == ".git" || name == "node_modules" {
                continue;
            }
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// Scan one file. Returns a violation if the file crosses the threshold:
/// `cyrillic_letters >= MIN_CYRILLIC_LETTERS` AND
/// `share > MIN_SHARE_PERCENT`.
///
/// The count is over the whole file text (matching the publisher's scan
/// methodology that produced the naryad #383 inventory), not excluding
/// code fences — a document that is Russian wholesale is Russian inside
/// its fences too.
fn scan_file(path: &Path) -> Option<Violation> {
    let src = fs::read_to_string(path).ok()?;
    let cyrillic_letters = src.chars().filter(|c| is_cyrillic(*c)).count();
    let total_letters = src.chars().filter(|c| c.is_alphabetic()).count();
    if total_letters == 0 {
        return None;
    }
    let share_percent = cyrillic_letters as f64 / total_letters as f64 * 100.0;
    if cyrillic_letters >= MIN_CYRILLIC_LETTERS && share_percent > MIN_SHARE_PERCENT {
        Some(Violation {
            path: path.to_path_buf(),
            cyrillic_letters,
            total_letters,
            share_percent,
        })
    } else {
        None
    }
}

#[test]
fn no_md_file_above_cyrillic_threshold_outside_allowlist() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let root = PathBuf::from(&manifest_dir);

    let mut violations: Vec<Violation> = Vec::new();
    for file in md_files(&root) {
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWLIST.contains(&rel.as_str()) {
            continue;
        }
        if let Some(v) = scan_file(&file) {
            violations.push(v);
        }
    }

    if !violations.is_empty() {
        let mut msg = String::from(
            "All repository documentation must be English-only (owner directive\n\
             2026-09-17, naryad #383). The following .md files exceed the Cyrillic\n\
             threshold (>= 20 Cyrillic letters AND > 2% of all letters) and are not\n\
             in the frozen allowlist:\n\n",
        );
        for v in &violations {
            msg.push_str(&format!(
                "  {} — {} Cyrillic letters / {} total ({:.1}%)\n",
                v.path.strip_prefix(&root).unwrap_or(&v.path).display(),
                v.cyrillic_letters,
                v.total_letters,
                v.share_percent
            ));
        }
        msg.push_str(
            "\nTranslate the Russian prose to technical English. Cyrillic inside\n\
             code examples / test fixtures (string literals that are data) is\n\
             content and may stay — but a document whose documentation language\n\
             is Russian is a defect. Growing the allowlist in\n\
             tests/docs_language_lint.rs requires an owner decision (AGENTS.md §3).",
        );
        panic!("{msg}");
    }
}

#[test]
fn allowlist_paths_exist_in_repo() {
    // The allowlist is frozen — a moved/renamed/deleted entry must be
    // pruned deliberately, not silently left behind.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let root = PathBuf::from(&manifest_dir);
    for entry in ALLOWLIST {
        let path = root.join(entry.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert!(
            path.is_file(),
            "allowlist entry `{entry}` does not exist in the repo — prune it from \
             tests/docs_language_lint.rs"
        );
    }
}

#[test]
fn allowlist_is_sorted_and_deduplicated() {
    let mut sorted = ALLOWLIST.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted, ALLOWLIST,
        "ALLOWLIST must be sorted and free of duplicates"
    );
}

#[test]
fn scan_file_flags_russian_prose() {
    // Mutation-style sanity: a document written in Russian must be flagged.
    let tmp = tempfile_dir();
    let file = tmp.join("sample.md");
    fs::write(
        &file,
        "# Заголовок\n\nЭтот документ написан по-русски, и линт обязан его поймать, \
         потому что документация проекта ведётся только на английском языке.\n",
    )
    .unwrap();
    let v = scan_file(&file);
    assert!(v.is_some(), "Russian-prose .md must be flagged");
    let v = v.unwrap();
    assert!(v.cyrillic_letters >= MIN_CYRILLIC_LETTERS);
    assert!(v.share_percent > MIN_SHARE_PERCENT);
}

#[test]
fn scan_file_accepts_english_document() {
    let tmp = tempfile_dir();
    let file = tmp.join("sample.md");
    fs::write(
        &file,
        "# Heading\n\nThis document is written in technical English. The naryad \
         protocol requires a branch, a PR, green CI, and a report. All good here.\n",
    )
    .unwrap();
    assert!(scan_file(&file).is_none(), "English .md must pass");
}

#[test]
fn scan_file_accepts_below_threshold_file() {
    // A few Cyrillic letters (a terminology anchor in an English glossary)
    // stay under both prongs of the threshold.
    let tmp = tempfile_dir();
    let file = tmp.join("sample.md");
    fs::write(
        &file,
        "# Glossary\n\nThe fixed terminology maps the Russian term «наряд» to \
         *naryad* and «диспатч» to *dispatch*; the rest of this document is \
         ordinary English prose with plenty of alphabetic content to keep the \
         Cyrillic share far below the threshold.\n",
    )
    .unwrap();
    assert!(scan_file(&file).is_none(), "below-threshold .md must pass");
}

#[test]
fn scan_file_accepts_cyrillic_data_in_code_example() {
    // Cyrillic string literals inside a code example are content (test data),
    // not documentation language — a small demo file must pass.
    let tmp = tempfile_dir();
    let file = tmp.join("sample.md");
    fs::write(
        &file,
        "# UTF-8 demo\n\nThe example below prints a Cyrillic greeting literal:\n\n\
         ```mlog\nlet greeting = \"Привет\"\nprint(greeting)\n```\n\n\
         The literal itself is test data; the surrounding prose is English.\n",
    )
    .unwrap();
    assert!(
        scan_file(&file).is_none(),
        "data-literal demo .md must pass"
    );
}

/// Helper: create a unique temp dir for each test invocation.
///
/// Uses `process::id()` + nanosecond timestamp + an atomic counter to
/// guarantee uniqueness across parallel test threads within the same
/// process (same pattern as `tests/ignore_reasons_lint.rs`).
fn tempfile_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!(
        "docs_language_lint_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        seq,
    ));
    fs::create_dir_all(&p).unwrap();
    p
}
