// ── Наряд №270: REFERENCE.md 100% coverage gate ──────────────────────────
//
// Contract: EVERY builtin registered in `BUILTIN_REGISTRY` (src/builtins/
// registry.rs `spec!` macros — the SSOT per AGENT.md §5) must have a
// REFERENCE.md entry. A builtin is considered documented when REFERENCE
// contains `` `name(` `` (the same mention-style rule
// scripts/gen_reference_check.py pioneered — cases tested here are the
// strict superset: a name mentioned ONLY in prose does satisfy the gate,
// but the generated §6 index structurally guarantees a real table row for
// every name).
//
// The generated index (§6, between the BEGIN/END markers) is produced by
// `scripts/gen_reference.py`; this test additionally pins the markers'
// presence so an accidental deletion of the generated block fails loudly
// instead of silently regressing coverage.
//
// Аудит-инвариант: до №270 REFERENCE documented ~59% of the registry —
// for a grant reviewer (NLnet/Restack) an incomplete reference reads as
// project immaturity, and nothing failed CI when builtins were added.

use regex::Regex;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn registry_builtin_names() -> Vec<String> {
    let path = repo_root().join("src").join("builtins").join("registry.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    let re = Regex::new(r#"spec!\(\s*"([^"]+)""#).unwrap();
    let mut names = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for cap in re.captures_iter(&content) {
        let name = cap.get(1).unwrap().as_str().to_string();
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }
    names
}

fn reference_text() -> String {
    let path = repo_root().join("REFERENCE.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e))
}

#[test]
fn reference_documents_every_registered_builtin() {
    let names = registry_builtin_names();
    assert!(
        names.len() >= 392,
        "sanity: the registry keeps growing, but the harness must see the full list (got {})",
        names.len()
    );

    let reference = reference_text();
    let missing: Vec<&String> = names
        .iter()
        .filter(|n| !reference.contains(&format!("`{}(", n)))
        .collect();

    assert!(
        missing.is_empty(),
        "REFERENCE.md is missing {} builtin(s) — run `python3 scripts/gen_reference.py` to regenerate \
         the §6 index, then describe the `TODO(doc)` rows in the handlers' /// docs:\n  {}",
        missing.len(),
        missing
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[test]
fn reference_generated_index_block_is_present() {
    let reference = reference_text();
    assert!(
        reference.contains("BEGIN GENERATED BUILTIN INDEX"),
        "the generated §6 index block is gone — regenerate with `python3 scripts/gen_reference.py`"
    );
    assert!(
        reference.contains("END GENERATED BUILTIN INDEX"),
        "the generated §6 index block is unterminated — regenerate with `python3 scripts/gen_reference.py`"
    );
    // The block's headline must claim exactly the registry size.
    let count = registry_builtin_names().len();
    let expected = format!("## 6. Builtin Index — {} registered builtins", count);
    assert!(
        reference.contains(&expected),
        "the §6 index headline does not match the registry size (expected {:?}) — regenerate",
        expected
    );
}

#[test]
fn reference_has_no_unresolved_todo_markers() {
    let reference = reference_text();
    let begin = reference
        .find("BEGIN GENERATED BUILTIN INDEX")
        .expect("marker pinned by the block test");
    let end = reference
        .find("END GENERATED BUILTIN INDEX")
        .expect("marker pinned by the block test");
    let block = &reference[begin..end];

    let todo_re = Regex::new(r"\|\s*`[a-z_][a-z0-9_]*\(\.\.\.\)`.*TODO\(doc\)").unwrap();
    let todos: Vec<&str> = todo_re
        .find_iter(block)
        .map(|m| {
            let s = m.as_str();
            let stop = s.find('|').map(|p| p + 1).unwrap_or(0);
            s[stop..].trim().split('|').next().unwrap_or("").trim()
        })
        .collect();

    assert!(
        todos.is_empty(),
        "{} generated index row(s) still carry TODO(doc) — write the missing `///` docs on the \
         handlers (or extend MANUAL_DESCRIPTIONS in scripts/gen_reference.py for handler-less \
         registry stubs):\n  {}",
        todos.len(),
        todos.join(", ")
    );
}
