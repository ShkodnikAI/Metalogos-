// ── README Consistency Test (Наряд №103) ───────────────────────────
// Catches stale numbers in README.md automatically.
// Start small (ADR count + version), expand as real divergences are found.
//
// Block 3 invariant: historical numbers (e.g. "142 builtins (0.9.1)" in
// the changelog table) are NOT checked — they are deliberately frozen
// snapshots of a past state, not claims about the current codebase.

use regex::Regex;
use std::fs;

/// The workspace root directory. Tests run from the crate root.
fn repo_root() -> std::path::PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo");
    std::path::PathBuf::from(manifest_dir)
}

/// Read the full README.md content.
fn read_readme() -> String {
    let path = repo_root().join("README.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e))
}

/// Count real ADR .md files (excluding README.md in the same directory).
fn real_adr_count() -> usize {
    let adr_dir = repo_root().join("docs").join("adr");
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(&adr_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                // Exclude the index/README in the ADR directory itself
                if path.file_name().is_some_and(|name| name == "README.md") {
                    continue;
                }
                count += 1;
            }
        }
    }
    count
}

/// Read the package version from Cargo.toml.
fn cargo_version() -> String {
    let path = repo_root().join("Cargo.toml");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    // Extract the first `version = "X.Y.Z"` line (the package version).
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") && trimmed.contains('"') {
            // Parse: version = "0.17.0"
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    return trimmed[start + 1..start + 1 + end].to_string();
                }
            }
        }
    }
    panic!("no version found in Cargo.toml");
}

/// Check whether a line is a historical entry in the changelog table.
/// Changelog lines look like: `| **0.9.1** | ... |`
/// Any number on such a line is a frozen historical value, not a current claim.
fn is_changelog_historical_line(line: &str) -> bool {
    let trimmed = line.trim();
    // Changelog table rows start with `|` and contain a bold version: **X.Y.Z**
    if !trimmed.starts_with('|') {
        return false;
    }
    trimmed.contains("**") && trimmed.contains("|")
}

/// Check whether a number on a given line is adjacent to an explicit version
/// reference in parentheses, e.g. "142 builtins (0.9.1)" — this marks it
/// as a historical snapshot, not a current claim.
fn is_historical_by_adjacent_version(line: &str) -> bool {
    let re = Regex::new(r"\(\d+\.\d+\.\d+\)").unwrap();
    re.is_match(line)
}

// ── Test: ADR count ────────────────────────────────────────────────

#[test]
fn readme_adr_count_matches_reality() {
    let readme = read_readme();
    let real = real_adr_count();

    // Find all "N ADR" / "N ADRs" / "N Architecture Decision Records"
    // on non-historical lines and extract the claimed counts.
    let re = Regex::new(r"(\d+)\s+ADR").unwrap();
    let mut claimed_counts: Vec<(usize, usize)> = Vec::new(); // (line_number, count)

    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;

        // Skip historical lines (changelog table rows or adjacent-version markers)
        if is_changelog_historical_line(line) || is_historical_by_adjacent_version(line) {
            continue;
        }

        let line_lower = line.to_lowercase();
        if !line_lower.contains("adr") {
            continue;
        }

        // Extract numbers preceding "ADR" / "ADRs" / "Architecture Decision Records"
        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    claimed_counts.push((line_num, n));
                }
            }
        }
    }

    assert!(
        !claimed_counts.is_empty(),
        "README should mention ADR count at least once on a non-historical line"
    );

    for (line_num, claimed) in &claimed_counts {
        assert_eq!(
            claimed, &real,
            "README line {} claims {} ADRs, but real count is {}",
            line_num, claimed, real
        );
    }
}

// ── Test: Version ──────────────────────────────────────────────────

#[test]
fn readme_version_matches_cargo_toml() {
    let readme = read_readme();
    let real_version = cargo_version();

    // Find all version mentions on non-historical lines.
    // We look for both `v0.17.0` and `0.17.0` patterns.
    let re = Regex::new(r"v?(\d+\.\d+\.\d+)").unwrap();

    let mut checked = false;
    for (i, line) in readme.lines().enumerate() {
        let _line_num = i + 1;

        // Skip changelog historical rows
        if is_changelog_historical_line(line) {
            continue;
        }

        // Skip lines where the version is parenthesized as a historical marker
        // e.g. "(0.9.1)" — but NOT the badge line `v0.17.0`
        if is_historical_by_adjacent_version(line) && !line.contains("badge") {
            continue;
        }

        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                let found_version = m.as_str();
                // Only count matches for the current version
                if found_version == real_version {
                    checked = true;
                }
            }
        }
    }

    assert!(
        checked,
        "README should mention the current version {} at least once on a non-historical line",
        real_version
    );
}

// ── Test: Parser rules ─────────────────────────────────────────────

/// Count real Pest PEG grammar rules in grammar.pest.
/// A rule is a line starting with `identifier =` (or `identifier   =`).
fn real_parser_rule_count() -> usize {
    let path = repo_root().join("src").join("grammar.pest");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    let re = Regex::new(r"^[a-zA-Z_][a-zA-Z_0-9]*\s*=").unwrap();
    content.lines().filter(|line| re.is_match(line)).count()
}

#[test]
fn readme_parser_rules_match_reality() {
    let readme = read_readme();
    let real = real_parser_rule_count();

    // Find "N rules" mentions on non-historical lines in README.
    let re = Regex::new(r"(\d+)\s+rules").unwrap();
    let mut claimed_counts: Vec<(usize, usize)> = Vec::new();

    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;

        if is_changelog_historical_line(line) || is_historical_by_adjacent_version(line) {
            continue;
        }

        let line_lower = line.to_lowercase();
        if !line_lower.contains("rules") {
            continue;
        }

        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    claimed_counts.push((line_num, n));
                }
            }
        }
    }

    assert!(
        !claimed_counts.is_empty(),
        "README should mention parser rule count at least once on a non-historical line"
    );

    for (line_num, claimed) in &claimed_counts {
        assert_eq!(
            claimed, &real,
            "README line {} claims {} parser rules, but real count is {}",
            line_num, claimed, real
        );
    }
}

// ── Test: SVG builtins ─────────────────────────────────────────────

/// Count SVG/Graphics subsystem builtins from the registry.
/// The SVG subsystem comprises builtins in these categories:
///   "svg" (includes color_palette), "chart", "diagram",
///   "template" (template_render), "web" (html_render).
/// Note: diagram_style has category "tokens" but starts with "diagram_"
/// and is part of the SVG subsystem.
fn real_svg_builtins_count() -> usize {
    let path = repo_root().join("src").join("builtins").join("registry.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));

    // Count spec! entries whose name starts with svg_/chart_/diagram_/color_palette/
    // template_render/html_render — these constitute the SVG/Graphics subsystem.
    let re =
        Regex::new(r#"spec!\("(svg_|chart_|diagram_|color_palette|template_render|html_render)"#)
            .unwrap();
    content.lines().filter(|line| re.is_match(line)).count()
}

#[test]
fn readme_svg_builtins_match_reality() {
    let readme = read_readme();
    let real = real_svg_builtins_count();

    // Find the SVG subsystem line: "N builtins, hand-rolled in pure Rust"
    // This is the specific SVG/Graphics count, NOT the total builtins count.
    let re = Regex::new(r"(\d+)\s+builtins,\s+hand-rolled").unwrap();
    let mut claimed_counts: Vec<(usize, usize)> = Vec::new();

    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;

        if is_changelog_historical_line(line) || is_historical_by_adjacent_version(line) {
            continue;
        }

        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    claimed_counts.push((line_num, n));
                }
            }
        }
    }

    assert!(
        !claimed_counts.is_empty(),
        "README should mention SVG builtin count (\"N builtins, hand-rolled\") at least once"
    );

    for (line_num, claimed) in &claimed_counts {
        assert_eq!(
            claimed, &real,
            "README line {} claims {} SVG builtins, but real count is {}",
            line_num, claimed, real
        );
    }
}

// ── Test: Total builtin count (Наряд №149) ───────────────────────

/// Count all spec!() entries in registry.rs — the single source of truth
/// for the total number of built-in functions.
fn real_total_builtins_count() -> usize {
    let path = repo_root().join("src").join("builtins").join("registry.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    Regex::new(r"spec!\(").unwrap().find_iter(&content).count()
}

#[test]
fn readme_total_builtins_match_reality() {
    let readme = read_readme();
    let real = real_total_builtins_count();

    // Match "N functions", "N builtins", "N built-in functions"
    // but NOT SVG-specific "N builtins, hand-rolled" (covered by readme_svg_builtins_match_reality).
    let re = Regex::new(r"(\d+)\s+(?:built-in\s+)?(?:functions?|builtins?)").unwrap();
    let mut claimed_counts: Vec<(usize, usize)> = Vec::new();

    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;

        // Skip historical lines (changelog table rows or adjacent-version markers)
        if is_changelog_historical_line(line) || is_historical_by_adjacent_version(line) {
            continue;
        }

        // Skip SVG-specific line ("N builtins, hand-rolled in pure Rust")
        if line.contains("hand-rolled") {
            continue;
        }

        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    claimed_counts.push((line_num, n));
                }
            }
        }
    }

    assert!(
        !claimed_counts.is_empty(),
        "README should mention total builtin count (\"N functions\" / \"N builtins\") at least once on a non-historical line"
    );

    for (line_num, claimed) in &claimed_counts {
        assert_eq!(
            claimed, &real,
            "README line {} claims {} total builtins, but real spec! count in registry.rs is {}",
            line_num, claimed, real
        );
    }
}

// ── Test: Builtin module (category) count (Наряд №149) ─────────────

/// Count unique builtin categories from registry.rs spec! entries.
/// Each spec! macro's category string (last string before optional => layer)
/// represents a logical "module".
/// Uses line-by-line parsing to avoid `)` in comments breaking `[^)]+`.
fn real_builtin_category_count() -> usize {
    let path = repo_root().join("src").join("builtins").join("registry.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));

    let string_re = Regex::new(r#""([^"]+)"#).unwrap();
    let layer_re = Regex::new(r#"=>\s*"[^"]+"#).unwrap();

    let mut categories: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in content.lines() {
        if !line.contains("spec!(") {
            continue;
        }
        // Strip comments to avoid `)` in comments breaking parsing
        let code_part = line.split("//").next().unwrap_or(line);
        // Strip => "layer" so it isn't mistaken for a category
        let clean = layer_re.replace_all(code_part, "");
        // Extract all string literals; category is the last one
        let strings: Vec<&str> = string_re
            .captures_iter(&clean)
            .map(|c| c.get(1).unwrap().as_str())
            .collect();
        if let Some(&cat) = strings.last() {
            categories.insert(cat.to_string());
        }
    }

    categories.len()
}

#[test]
fn readme_builtin_module_count_matches_reality() {
    let readme = read_readme();
    let real = real_builtin_category_count();

    // Match "N modules" only on lines that also mention "function" or "builtin"
    // (to avoid matching unrelated "12 modules" for the tree-walking interpreter).
    let re = Regex::new(r"(\d+)\s+modules").unwrap();
    let mut claimed_counts: Vec<(usize, usize)> = Vec::new();

    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;

        if is_changelog_historical_line(line) || is_historical_by_adjacent_version(line) {
            continue;
        }

        // Only check lines in builtin context
        let line_lower = line.to_lowercase();
        if !line_lower.contains("function")
            && !line_lower.contains("builtin")
            && !line_lower.contains("built-in")
        {
            continue;
        }

        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    claimed_counts.push((line_num, n));
                }
            }
        }
    }

    assert!(
        !claimed_counts.is_empty(),
        "README should mention builtin module count (\"N modules\" near \"functions\"/\"builtins\") at least once"
    );

    for (line_num, claimed) in &claimed_counts {
        assert_eq!(
            claimed, &real,
            "README line {} claims {} builtin modules, but real category count in registry.rs is {}",
            line_num, claimed, real
        );
    }
}

// ── Наряд №166 Block 2: file-size + example-count consistency ────────
//
// The previous tests (Наряд №103) covered ADR count, version, parser
// rules, builtin counts. Наряд №166 extends coverage to:
//   - CHANGELOG.md size in KB (tolerance ±2 KB — files grow over time)
//   - REFERENCE.md size in KB (tolerance ±2 KB)
//   - Number of .mlog example programs (exact match)
//   - Number of ADR files (cross-checks readme_adr_count_matches_reality
//     via a different code path — direct file count, not regex on README)
//
// Commits count is intentionally NOT covered: it changes on every
// commit, so a static README number would always be stale. README now
// links to GitHub instead of stating a number.

/// File size in KB (1024 bytes per KB), rounded down.
fn file_size_kb(path_relative_to_repo_root: &str) -> u64 {
    let path = repo_root().join(path_relative_to_repo_root);
    let metadata = fs::metadata(&path).unwrap_or_else(|e| panic!("cannot stat {:?}: {}", path, e));
    metadata.len() / 1024
}

/// Count actual .mlog files in examples/ (excluding subdirectories —
/// the README claim is about top-level .mlog programs, not contracts).
fn real_example_count() -> usize {
    let examples_dir = repo_root().join("examples");
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(&examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "mlog") {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn readme_changelog_size_within_tolerance() {
    // README claims CHANGELOG.md is ~72 KB. Files grow, so allow ±2 KB.
    // The number in README is written as `(~72 KB)` — extract that.
    let readme = read_readme();
    let real_kb = file_size_kb("CHANGELOG.md");

    // Find the line in README that mentions CHANGELOG.md size.
    // Pattern: `CHANGELOG.md ... (NN KB)` or `(~NN KB)` or `(NN KB)`.
    let re = Regex::new(r"CHANGELOG\.md[^\n]*?\(~?(\d+)\s*KB\)").unwrap();
    let mut found = false;
    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;
        if is_changelog_historical_line(line) {
            continue;
        }
        if let Some(cap) = re.captures(line) {
            let claimed_kb: u64 = cap
                .get(1)
                .unwrap()
                .as_str()
                .parse()
                .expect("digits matched by regex");
            let diff = real_kb.abs_diff(claimed_kb);
            assert!(
                diff <= 2,
                "README line {}: CHANGELOG.md claims ~{} KB, real size {} KB (Δ{} KB > 2 KB tolerance). \
                 Update README.md to reflect the new size.",
                line_num,
                claimed_kb,
                real_kb,
                diff
            );
            found = true;
        }
    }
    assert!(
        found,
        "README should mention CHANGELOG.md size in KB (e.g. `CHANGELOG.md ... (~72 KB)`). \
         Real size: {} KB",
        real_kb
    );
}

#[test]
fn readme_reference_size_within_tolerance() {
    // README claims REFERENCE.md is ~72 KB. Same ±2 KB tolerance.
    let readme = read_readme();
    let real_kb = file_size_kb("REFERENCE.md");
    let re = Regex::new(r"REFERENCE\.md[^\n]*?\(~?(\d+)\s*KB\)").unwrap();
    let mut found = false;
    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;
        if is_changelog_historical_line(line) {
            continue;
        }
        if let Some(cap) = re.captures(line) {
            let claimed_kb: u64 = cap
                .get(1)
                .unwrap()
                .as_str()
                .parse()
                .expect("digits matched by regex");
            let diff = real_kb.abs_diff(claimed_kb);
            assert!(
                diff <= 2,
                "README line {}: REFERENCE.md claims ~{} KB, real size {} KB (Δ{} KB > 2 KB tolerance). \
                 Update README.md to reflect the new size.",
                line_num,
                claimed_kb,
                real_kb,
                diff
            );
            found = true;
        }
    }
    assert!(
        found,
        "README should mention REFERENCE.md size in KB (e.g. `REFERENCE.md ... (~72 KB)`). \
         Real size: {} KB",
        real_kb
    );
}

#[test]
fn readme_example_count_matches_reality() {
    // README claims "N .mlog programs" in two places:
    //   - Tree comment: `├── examples/ # 189 .mlog programs`
    //   - Metrics table: `| Example Programs | 189 |`
    // Both must match the real count of top-level .mlog files in examples/.
    let readme = read_readme();
    let real = real_example_count();

    let re = Regex::new(r"(\d+)\s*\.mlog\s+programs").unwrap();
    let mut claimed_counts: Vec<(usize, usize)> = Vec::new();

    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;
        if is_changelog_historical_line(line) || is_historical_by_adjacent_version(line) {
            continue;
        }
        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(n) = m.as_str().parse::<usize>() {
                    claimed_counts.push((line_num, n));
                }
            }
        }
    }

    // Also check the Metrics table row: `| Example Programs | N |`
    let metrics_re = Regex::new(r"\|\s*Example Programs\s*\|\s*(\d+)\s*\|").unwrap();
    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;
        if is_changelog_historical_line(line) {
            continue;
        }
        if let Some(cap) = metrics_re.captures(line) {
            if let Ok(n) = cap.get(1).unwrap().as_str().parse::<usize>() {
                claimed_counts.push((line_num, n));
            }
        }
    }

    assert!(
        !claimed_counts.is_empty(),
        "README should mention example count (\"N .mlog programs\" or Metrics table) at least once"
    );

    for (line_num, claimed) in &claimed_counts {
        assert_eq!(
            claimed, &real,
            "README line {} claims {} example programs, but real count in examples/ is {}",
            line_num, claimed, real
        );
    }
}

#[test]
fn readme_does_not_claim_static_commit_count() {
    // Наряд №166 Block 2: commits is a live number. Static README
    // numbers drift on every commit. The README should link to GitHub
    // instead of stating a number — this test enforces that contract.
    //
    // We allow either:
    //   - No mention of a specific commit count on non-historical lines
    //   - A line that links to GitHub commits (no specific number)
    //
    // We DO allow historical commit numbers in the changelog table
    // (those are frozen snapshots of past releases).
    let readme = read_readme();
    // Look for `| Commits | NNN |` in the Metrics table.
    let metrics_re = Regex::new(r"\|\s*Commits\s*\|\s*(\d+)\s*\|").unwrap();
    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;
        if is_changelog_historical_line(line) {
            continue;
        }
        if let Some(cap) = metrics_re.captures(line) {
            panic!(
                "README line {}: Metrics table has static commit count `{}`. \
                 Commits is a live number — replace with a link to \
                 https://github.com/ShkodnikAI/Metalogos-/commits/main \
                 (see Наряд №166 Block 2).",
                line_num,
                cap.get(1).unwrap().as_str()
            );
        }
    }
    // Also check the prose line "... NNN commits." — same rule.
    let prose_re = Regex::new(r"\b(\d{3,})\s+commits\b").unwrap();
    for (i, line) in readme.lines().enumerate() {
        let line_num = i + 1;
        if is_changelog_historical_line(line) {
            continue;
        }
        if let Some(cap) = prose_re.captures(line) {
            panic!(
                "README line {}: prose mentions static commit count `{}`. \
                 Commits is a live number — replace with a link to \
                 https://github.com/ShkodnikAI/Metalogos-/commits/main \
                 (see Наряд №166 Block 2).",
                line_num,
                cap.get(1).unwrap().as_str()
            );
        }
    }
}

// ── Наряд №167: REFERENCE.md builtin coverage baseline ───────────────
//
// Block 2: connect scripts/gen_reference_check.py logic to CI as a
// blocking test. The test does NOT require 100% coverage — that would
// be unrealistic for a 357-builtin codebase with new builtins added
// frequently. Instead it freezes a BASELINE: the number of missing
// builtins must not GROW.
//
// When a new builtin is added to registry.rs without REFERENCE.md
// documentation, this test fails and the contributor must either:
//   1. Document the builtin in REFERENCE.md (preferred — see Block 3
//      of naryad 167 for the batched documentation effort)
//   2. Bump the BASELINE constant with a comment explaining why this
//      specific builtin is undocumented (e.g. experimental, behind a
//      feature gate, internal-only)
//
// The baseline is the count of missing builtins at the time this test
// was added. As naryad 167 (and follow-up batches) close the gap, the
// baseline should DECREASE — never increase.

/// Names declared in `src/builtins/registry.rs` via `spec!("name", ...)`.
/// Mirrors `scripts/gen_reference_check.py::collect_builtin_names`.
fn collect_registry_builtin_names() -> Vec<String> {
    let path = repo_root().join("src").join("builtins").join("registry.rs");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    let re = Regex::new(r#"spec!\("(\w+)""#).unwrap();
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

/// Names documented in REFERENCE.md (matched as `` `name(` `` — requires
/// an actual signature with arguments, not a bare prose mention).
/// Mirrors `scripts/gen_reference_check.py::collect_documented_names`.
fn collect_documented_builtin_names() -> std::collections::HashSet<String> {
    let path = repo_root().join("REFERENCE.md");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    let re = Regex::new(r"`(\w+)\(").unwrap();
    re.captures_iter(&content)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .collect()
}

#[test]
fn reference_md_builtin_coverage_does_not_regression() {
    // Наряд №167 Block 2: freeze the number of missing builtins.
    //
    // At the time this test was committed, 191 of 357 builtins were
    // missing from REFERENCE.md (46.5% covered, 53.5% missing). The
    // baseline was 191. After naryad 167 Block 3 (crypto/encoding/
    // test/system/fluid/mtree/graph/cron/time/json-dict documentation
    // batches), coverage rose to 212/357 (59.4%), 145 missing.
    // Baseline lowered from 191 to 145 to lock in the gain.
    //
    // As follow-up batches land (string/*, ext/*, pdf/*, etc.), the
    // baseline should DECREASE further — never increase.
    //
    // If you added a new builtin to registry.rs and this test fails,
    // you have two options (in order of preference):
    //
    //   1. Document the builtin in REFERENCE.md (see section 4.x for
    //      the appropriate category table).
    //
    //   2. If the builtin is experimental / behind a feature gate /
    //      internal-only and intentionally undocumented, bump
    //      `BASELINE_MISSING_COUNT` and add a comment explaining why.
    //      Do NOT bump it just to make the test pass — every bump
    //      is a regression in user-facing documentation.
    const BASELINE_MISSING_COUNT: usize = 145;

    let all_names = collect_registry_builtin_names();
    let documented = collect_documented_builtin_names();
    let missing: Vec<&String> = all_names
        .iter()
        .filter(|n| !documented.contains(*n))
        .collect();

    let total = all_names.len();
    let missing_count = missing.len();
    let covered = total - missing_count;
    let pct = (covered as f64 / total as f64) * 100.0;

    assert!(
        missing_count <= BASELINE_MISSING_COUNT,
        "REFERENCE.md builtin coverage regression: {} missing (baseline {}), {} total builtins, \
         {:.1}% covered. Missing builtins:\n{}\n\
         To fix: document these builtins in REFERENCE.md, OR if some are intentionally \
         undocumented (experimental/feature-gated), bump BASELINE_MISSING_COUNT in \
         tests/readme_consistency.rs with a comment explaining why. See Наряд №167.",
        missing_count,
        BASELINE_MISSING_COUNT,
        total,
        pct,
        missing
            .iter()
            .take(20)
            .map(|n| format!("  - {}", n))
            .collect::<Vec<_>>()
            .join("\n")
            + if missing_count > 20 {
                "\n  ... (truncated)"
            } else {
                ""
            }
    );

    // Sanity: ensure the baseline is still meaningful. If documentation
    // efforts have closed the gap significantly, the baseline should be
    // lowered — print a reminder.
    if missing_count < BASELINE_MISSING_COUNT.saturating_sub(20) {
        eprintln!(
            "[INFO] REFERENCE.md coverage improved significantly: {} missing (baseline {}). \
             Consider lowering BASELINE_MISSING_COUNT in tests/readme_consistency.rs to lock in the gain.",
            missing_count, BASELINE_MISSING_COUNT
        );
    }
}
