// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

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
const ALLOWLIST: &[&str] = &[
    "docs/adr/0043-unicode-fix.md",
    // №810 (the C4-a T2 owner decision): the monthly REALITY archives —
    // the verbatim wave-record transfer (Russian prose is the RECORD
    // content; the doc is a historical archive, not the documentation
    // language). Growing this list stays an explicit, reviewed PR edit.
    "docs/reality/2026-09.md",
    "docs/reality/2026-10.md",
    // №871 (the №810 class carried forward — the same owner decision):
    // the November REALITY archive opens with the wave-76 record — the
    // verbatim Russian prose stays the RECORD content (the monthly
    // archive is a historical record, not the documentation language).
    "docs/reality/2026-11.md",
];

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

/// The NON-`.md` agent-input documents (№798, the audit-23.10 B6/D-2):
/// `llms.txt` is the agent-facing index (the RAG/IDE entry point) and is
/// documentation like any `.md` — the owner's 2026-09-17 English-only
/// directive applies to it too. It lived OUTSIDE the lint's `.md`-only
/// walk, so the directive was not machine-enforced for it.
const NON_MD_AGENT_INPUTS: &[&str] = &["llms.txt"];

/// Walk a directory recursively, yielding paths to all `.md` files PLUS
/// the explicit non-`.md` agent inputs (`llms.txt`).
///
/// `target/` (build artifacts) and `.git/` are skipped; other hidden
/// directories (e.g. `.github`) ARE scanned — the PR template lives there
/// and is documentation like everything else.
fn md_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, &mut out);
    for rel in NON_MD_AGENT_INPUTS {
        let path = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if path.is_file() {
            out.push(path);
        }
    }
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

/// №798 (the audit-23.10 B6/D-2): the coverage pin — the non-`.md`
/// agent inputs are INSIDE the lint's scanned set (the pre-№798 lint
/// walked `.md` only, so `llms.txt` carried Russian prose unflagged —
/// the directive was declared but not enforced for the file).
#[test]
fn n798_lint_covers_the_non_md_agent_inputs() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let root = PathBuf::from(&manifest_dir);
    let scanned = md_files(&root);
    for rel in NON_MD_AGENT_INPUTS {
        let target = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert!(
            scanned.contains(&target),
            "the lint must scan `{rel}` (the №798 agent-input coverage)"
        );
    }
}

/// The negative control: the scanner FLAGS a synthetic Russian document
/// and PASSES an English one — the mechanism's self-test (the planted
/// regression form; no repo file is mutated).
#[test]
fn n798_scanner_flags_russian_and_passes_english() {
    let dir = std::env::temp_dir().join("n798-lint-negative-control");
    let _ = std::fs::create_dir_all(&dir);
    let russian = dir.join("russian_fixture.md");
    std::fs::write(
        &russian,
        "Это тестовый документ на русском языке для проверки lint-механизма.\n         Он содержит достаточно кириллицы, чтобы пересечь порог срабатывания.\n",
    )
    .expect("write the fixture");
    assert!(
        scan_file(&russian).is_some(),
        "the Russian document must be flagged"
    );
    let english = dir.join("english_fixture.md");
    std::fs::write(
        &english,
        "This is a test document in technical English for the lint mechanism.\n         It contains no Cyrillic letters at all, so the threshold is never crossed.\n",
    )
    .expect("write the fixture");
    assert!(scan_file(&english).is_none(), "the English document passes");
    let _ = std::fs::remove_file(&russian);
    let _ = std::fs::remove_file(&english);
    let _ = std::fs::remove_dir(&dir);
}

// ── №815: the conflict-marker tripwire ──────────────────────────────────
//
// llms.txt landed on main (0e4ce59, №806) carrying a RAW git conflict
// block — `<<<<<<< HEAD` / `=======` / `>>>>>>> …` — the resolution
// committed the markers themselves, and four merges (№807/№808/№811/№812)
// plus every gate passed them through: the agent-index gate covers the
// machine-generated index lines, not this hand-maintained region, and the
// language lint checks the language, not the structure. The tripwire: no
// tracked text document carries a conflict-marker line. The synthetic pin
// (the negative test) proves the scanner catches the exact shape it
// exists for — the class repeats silently unless it is grounded.

/// A git conflict-marker line, in git's own spelling: exactly seven
/// characters. The `<`/`>` forms carry the branch name after a space.
/// The `=======` separator is counted ONLY inside an open marker block
/// (a stateful scan, not a bare line match): a setext-style underline of
/// exactly seven equals outside any block is content, not a conflict.
fn conflict_marker_lines(content: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_block = false;
    for (i, ln) in content.lines().enumerate() {
        let t = ln.trim_end_matches('\r');
        let is_marker = if t.starts_with("<<<<<<< ") {
            in_block = true;
            true
        } else if t.starts_with(">>>>>>> ") {
            in_block = false;
            true
        } else {
            t == "=======" && in_block
        };
        if is_marker {
            out.push((i + 1, t.to_string()));
        }
    }
    out
}

#[test]
fn n815_conflict_marker_scanner_catches_the_synthetic_shape() {
    let marked = "prose line\n<<<<<<< HEAD\nkept line\n=======\n>>>>>>> other-branch\n";
    let hits = conflict_marker_lines(marked);
    assert_eq!(
        hits.len(),
        3,
        "all three git marker shapes are caught: {hits:?}"
    );
    // The content-level shapes are NOT markers (no false trip): a bare
    // seven-equals underline OUTSIDE a block (the setext style), short
    // arrow runs, and mid-line mentions all pass.
    let clean =
        "title\n=======\nthe <<<<<<< and >>>>>>> shapes are discussed mid-line\n<<< short\n";
    assert!(
        conflict_marker_lines(clean).is_empty(),
        "content-level separators and prose mentions pass"
    );
}

#[test]
fn no_tracked_doc_carries_conflict_markers() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let root = PathBuf::from(&manifest_dir);

    let mut violations: Vec<String> = Vec::new();
    for file in md_files(&root) {
        if let Ok(src) = fs::read_to_string(&file) {
            let hits = conflict_marker_lines(&src);
            if !hits.is_empty() {
                let rel = file.strip_prefix(&root).unwrap_or(&file).display();
                violations.push(format!("{rel}: {} marker line(s)", hits.len()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "conflict-marker lines in tracked docs (the №806 llms.txt class — \
         resolve the conflict, never commit the markers; check \
         `git grep -nE '^(<{{7}}|={{7}}$|>{{7}})'` before staging):\n  {}",
        violations.join("\n  ")
    );
}
