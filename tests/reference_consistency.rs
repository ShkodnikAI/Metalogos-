// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Наряд №270: REFERENCE.md 100% coverage gate ──────────────────────────
//
// Contract: EVERY builtin registered in `BUILTIN_REGISTRY` (src/builtins/
// registry.rs `spec!` macros — the SSOT per AGENTS.md §5) must have a
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
// for an external reviewer an incomplete reference reads as
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

// ── №928: the REFERENCE.md head pin (the «шапка вне пина» class closed) ──
//
// Lesson of the R67 fact-check (2026-11-07): the №270 gate covered the
// §4/§6 BODY of the reference while the head silently drifted for six
// weeks (Version 0.20.0 / 439 builtins / 156 ADR files against the
// machine's 0.39.0 / 530 / 209) with a fully green 57/57 CI. A fact in
// a reader surface lives together with its checker (the №735 class):
// every head number is now pinned to its generating machine.
//
// The three head facts and their machines:
//   1. `Version:`            == Cargo.toml `[workspace.package] version`
//      (the same source the README badge reads);
//   2. builtins in the `Synced with code` line == the named `spec!`
//      count in the registry (the SAME convention as the §4 gate above —
//      `spec!("name")` rows, deduplicated);
//   3. ADR files == the `docs/adr/*.md` files minus the README index
//      (the index itself is not a decision record).

fn workspace_package_version() -> String {
    let path = repo_root().join("Cargo.toml");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    // The `[workspace.package]` version — the workspace SSOT the README
    // badge and the release lockstep read. Scoped to the section so a
    // future per-crate `version =` row cannot shadow it.
    let section = content
        .split("[workspace.package]")
        .nth(1)
        .expect("Cargo.toml carries the [workspace.package] section");
    let line = section
        .lines()
        .find(|l| l.trim_start().starts_with("version"))
        .expect("the [workspace.package] section carries the version row");
    line.split('"')
        .nth(1)
        .unwrap_or_else(|| panic!("unparsable version row: {line}"))
        .to_string()
}

fn adr_file_count() -> usize {
    let dir = repo_root().join("docs").join("adr");
    std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.is_file() && p.extension().map(|x| x == "md").unwrap_or(false)
        })
        .filter(|e| e.file_name() != "README.md")
        .count()
}

#[test]
fn reference_head_version_matches_cargo_workspace() {
    let reference = reference_text();
    let head = reference.lines().take(10).collect::<Vec<_>>().join("\n");
    let version = workspace_package_version();
    let expected = format!("> **Version:** {version}");
    assert!(
        head.contains(&expected),
        "the REFERENCE.md head Version does not match the machine: the head must carry \
         {expected:?} (from Cargo.toml [workspace.package]) — a stale head drifted past a \
         green CI once already (the R67 lesson); sync it with naryad №928"
    );
}

#[test]
fn reference_head_builtin_count_matches_registry() {
    let reference = reference_text();
    let head = reference.lines().take(10).collect::<Vec<_>>().join("\n");
    let count = registry_builtin_names().len();
    let synced_re = Regex::new(r"\*\*Synced with code:\*\*[^\n]*?·\s*(\d+)\s+builtins").unwrap();
    let got = synced_re
        .captures(&head)
        .unwrap_or_else(|| {
            panic!("the REFERENCE.md head carries no «Synced with code … N builtins» line")
        })
        .get(1)
        .unwrap()
        .as_str()
        .parse::<usize>()
        .unwrap();
    assert_eq!(
        got, count,
        "the REFERENCE.md head builtin count ({got}) does not match the registry named spec! \
         count ({count}) — the head drifted (the R67 lesson); re-sync with naryad №928"
    );
}

#[test]
fn reference_head_adr_count_matches_files() {
    let reference = reference_text();
    let head = reference.lines().take(10).collect::<Vec<_>>().join("\n");
    let count = adr_file_count();
    let synced_re = Regex::new(r"\*\*Synced with code:\*\*[^\n]*?·\s*(\d+)\s+ADR files").unwrap();
    let got = synced_re
        .captures(&head)
        .unwrap_or_else(|| panic!("the REFERENCE.md head carries no «… N ADR files» clause"))
        .get(1)
        .unwrap()
        .as_str()
        .parse::<usize>()
        .unwrap();
    assert_eq!(
        got, count,
        "the REFERENCE.md head ADR count ({got}) does not match docs/adr/*.md minus the \
         README index ({count}) — the head drifted (the R67 lesson); re-sync with naryad №928"
    );
}

// ── №809: the three-tier layout pin (the C3-a T3 owner decision) ────────
//
// The generated §6 index reads by MATURITY TIER: `stable` is the primary
// reference, `experimental` is the opt-in tier, `parked` is the appendix.
// The counts are machine facts: the headline carries the triple, each
// tier header carries its own count, the counts sum to the registry
// total, and every row sits in the section of its own registry tier
// (a row under the wrong tier moves a count and trips this pin).

fn registry_tiers() -> Vec<(String, String)> {
    let path = repo_root().join("src").join("builtins").join("registry.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    let name_re = Regex::new(r#"spec!\(\s*"([^"]+)""#).unwrap();
    let tier_re = Regex::new(r#""(stable|experimental|parked)""#).unwrap();
    let mut out = Vec::new();
    for line in content.lines() {
        let Some(nm) = name_re.captures(line) else {
            continue;
        };
        let Some(tm) = tier_re.captures_iter(line).last() else {
            continue;
        };
        out.push((
            nm.get(1).unwrap().as_str().to_string(),
            tm.get(1).unwrap().as_str().to_string(),
        ));
    }
    out
}

#[test]
fn reference_tier_sections_carry_the_machine_counts() {
    let tiers = registry_tiers();
    assert!(
        tiers.len() >= 392,
        "sanity: every spec! row carries the №796 tier literal (got {})",
        tiers.len()
    );

    let reference = reference_text();
    let begin = reference
        .find("BEGIN GENERATED BUILTIN INDEX")
        .expect("marker pinned by the block test");
    let end = reference
        .find("END GENERATED BUILTIN INDEX")
        .expect("marker pinned by the block test");
    let block = &reference[begin..end];

    // The headline triple.
    let mut counts = std::collections::BTreeMap::new();
    for (_, tier) in &tiers {
        *counts.entry(tier.as_str()).or_insert(0usize) += 1;
    }
    let headline = format!(
        "the three maturity tiers: stable {} / experimental {} / parked {}",
        counts.get("stable").copied().unwrap_or(0),
        counts.get("experimental").copied().unwrap_or(0),
        counts.get("parked").copied().unwrap_or(0),
    );
    assert!(
        block.contains(&headline),
        "the §6 headline does not carry the machine tier triple {headline:?} — regenerate"
    );

    // Per-tier rows: every row's trailing Tier cell matches the section
    // it sits in, and the section header carries the section's size.
    let tier_header_re =
        Regex::new(r"### Tier `(stable|experimental|parked)`[^\n]*\((\d+)\)").unwrap();
    let row_re = Regex::new(r"^\| `[a-z_][a-z0-9_]*\(").unwrap();
    let cell_tier_re = Regex::new(r"\|\s*(stable|experimental|parked)\s*\|\s*$").unwrap();

    let mut current = String::new();
    let mut section_rows: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut header_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    // The name extractor hoisted out of the loop (the clippy 1.99
    // regex_creation_in_loops lint — a Regex::new per row line is waste).
    let row_name_re = Regex::new(r"`([a-z_][a-z0-9_]*)\(").unwrap();
    for line in block.lines() {
        if let Some(h) = tier_header_re.captures(line) {
            current = h.get(1).unwrap().as_str().to_string();
            header_counts.insert(current.clone(), h.get(2).unwrap().as_str().parse().unwrap());
            continue;
        }
        if !current.is_empty() && row_re.is_match(line) {
            let name_cap = row_name_re
                .captures(line)
                .expect("a row line opens with the name");
            let name = name_cap.get(1).unwrap().as_str().to_string();
            let cell = cell_tier_re
                .captures(line)
                .unwrap_or_else(|| panic!("row for {name} carries no Tier cell: {line}"));
            assert_eq!(
                cell.get(1).unwrap().as_str(),
                current,
                "the row for {name} sits in the `{current}` section but its Tier cell says {}",
                cell.get(1).unwrap().as_str()
            );
            section_rows.entry(current.clone()).or_default().push(name);
        }
    }

    let mut registry_by_tier: std::collections::BTreeMap<String, Vec<&String>> =
        std::collections::BTreeMap::new();
    for (name, tier) in &tiers {
        registry_by_tier.entry(tier.clone()).or_default().push(name);
    }
    for tier in ["stable", "experimental", "parked"] {
        let want = registry_by_tier.get(tier).cloned().unwrap_or_default();
        let got = section_rows.get(tier).cloned().unwrap_or_default();
        assert_eq!(
            got.len(),
            want.len(),
            "the `{tier}` section carries {} row(s) but the registry has {} — \
             a row is misplaced or missing (regenerate the index)",
            got.len(),
            want.len()
        );
        assert_eq!(
            header_counts.get(tier).copied().unwrap_or(0),
            want.len(),
            "the `{tier}` header count does not match the registry tier size"
        );
    }
}
