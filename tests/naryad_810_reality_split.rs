// ── tests/naryad_810_reality_split.rs ────────────────────────────────
// №810 (issue #1566, wave 66 — the C4-a T2 owner decision, 23.10):
// docs/REALITY.md carries the CURRENT TRUTH only (≤30 KB — «текущая
// правда» отдельно от помесячного архива), the §6.x wave-record history
// lives in docs/reality/YYYY-MM.md — transferred VERBATIM («закрытое
// не удаляется молча» preserved by the transfer: the archive carries
// every section the pre-split REALITY.md carried, and the REALITY.md
// generated stub maps the archive explicitly).
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()))
}

/// The current-truth ceiling (the owner decision: «≤30 КБ»).
const REALITY_CEILING_BYTES: usize = 30 * 1024;

/// The pre-split §6.x section count — the TRANSFER FLOOR, not a freeze
/// (the №810 weekly-record episode: the first --archive append added §6.66
/// and the rigid equality pin went red; the archive is append-only by
/// design, so the pin keeps the floor — the transfer's reconciliation —
/// and lets the count grow with every weekly record; a count BELOW the
/// floor means a transferred section was dropped, the exact silent-
/// deletion class the transfer forbids).
const PRE_SPLIT_SECTION_COUNT: usize = 50;

#[test]
fn n810_reality_md_stays_under_the_current_truth_ceiling() {
    let p = repo_root().join("docs").join("REALITY.md");
    let bytes = fs::metadata(&p).expect("REALITY.md exists").len() as usize;
    assert!(
        bytes <= REALITY_CEILING_BYTES,
        "REALITY.md is {} bytes — over the 30 KB current-truth ceiling (№810); \
         the history belongs to docs/reality/, regenerate the stub",
        bytes
    );
}

#[test]
fn n810_reality_md_carries_no_history_sections() {
    let text =
        fs::read_to_string(repo_root().join("docs").join("REALITY.md")).expect("REALITY.md exists");
    // The history headers (### 6.1 … ### 6.65 with the record prose) must
    // be gone — the generated stub's §6/§7 are the only 6.x-named H2s now.
    let history_headers: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("### 6.") && l.contains("Wave"))
        .collect();
    assert!(
        history_headers.is_empty(),
        "REALITY.md still carries {} history section header(s) — the split moved them to docs/reality/: {:?}",
        history_headers.len(),
        history_headers.first(),
    );
    assert!(
        text.contains("BEGIN GENERATED REALITY STUB"),
        "the generated stub (the machine truth + the archive map) is missing — regenerate"
    );
}

#[test]
fn n810_archive_carries_every_transferred_section() {
    let reality_dir = repo_root().join("docs").join("reality");
    let mut total = 0usize;
    let mut files = 0usize;
    for entry in fs::read_dir(&reality_dir).expect("docs/reality/ exists") {
        let path = entry.expect("readdir").path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        files += 1;
        let text = fs::read_to_string(&path).expect("readable archive");
        let count = text.lines().filter(|l| l.starts_with("### 6.")).count();
        assert!(count > 0, "{} carries no sections", path.display());
        total += count;
    }
    assert!(
        files >= 2,
        "the monthly archive split (2026-09 + 2026-10) exists"
    );
    assert!(
        total >= PRE_SPLIT_SECTION_COUNT,
        "the archive carries {} section(s) but the pre-split transfer floor is {} — \
         a transferred section was dropped (the silent-deletion class); \
         the archive is append-only, so the count may only grow",
        total,
        PRE_SPLIT_SECTION_COUNT
    );
}

#[test]
fn n810_reality_stub_maps_the_archive() {
    // The generated §7 map must name every archive file with its real
    // section count — the reconciliation is visible in the doc itself.
    let text =
        fs::read_to_string(repo_root().join("docs").join("REALITY.md")).expect("REALITY.md exists");
    let reality_dir = repo_root().join("docs").join("reality");
    for entry in fs::read_dir(&reality_dir).expect("docs/reality/ exists") {
        let path = entry.expect("readdir").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let count = fs::read_to_string(&path)
            .expect("readable")
            .lines()
            .filter(|l| l.starts_with("### 6."))
            .count();
        assert!(
            text.contains(&format!("(reality/{})", name)),
            "the REALITY.md archive map does not list docs/reality/{} — regenerate the stub",
            name
        );
        assert!(
            text.contains(&format!("| {} |", count)) || text.contains(&format!("| **{}**", count)),
            "the map row for {} does not carry its section count {}",
            name,
            count
        );
    }
}
