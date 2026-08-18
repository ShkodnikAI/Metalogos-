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
