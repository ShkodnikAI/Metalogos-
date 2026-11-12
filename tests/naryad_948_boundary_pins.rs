// ── №948 (issue #1915): the honest-boundary pins (the audit 12.11) ─────
//
// The threat-model §The inter-pattern boundary tabulates every flow the
// analyzer/executable surface does NOT stop on v0.39.0. Each row carries
// a pin fixture in examples/boundaries/ whose expected behavior is the
// CURRENT one (`mlog check` passes — the gap is real). The FIXING naryad
// flips its row's pin into a blocking leak-suite negative:
//   №950 flips B1–B4, №951 flips B5–B8, №953 flips B10,
//   №954 flips B11, №952 lands the B12 machinery, B9 stays pass-at-check
//   (the №955 fix is runtime: the 500 body and the error text).
//
// This file is also the DOCS GATE: the pre-№948 overclaim
// ("interprocedural taint IS tracked") must never return to
// docs/threat-model.md or docs/limitations.md without the table. The
// №948 mutation removes the table + restores the phrase → this file red.
//
// The fixture I/O is the leak-suite precedent (tests/run_leak_suite.rs):
// real filesystem reads of repo fixtures by design.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    Path::new(&manifest).to_path_buf()
}

fn boundary_dir() -> PathBuf {
    repo_root().join("examples").join("boundaries")
}

/// The single-file pins: (fixture, table row, fixing naryad).
const SINGLE_FILE_PINS: &[(&str, &str, u32)] = &[
    ("b01_source_in_pattern_return.mlog", "B1", 950),
    ("b02_param_to_sink.mlog", "B2", 950),
    ("b03_html_via_pattern.mlog", "B3", 950),
    ("b04_exec_via_pattern.mlog", "B4", 950),
    ("b05_each_accumulator.mlog", "B5", 951),
    ("b06_secret_in_get_url.mlog", "B6", 951),
    ("b07_flow_output.mlog", "B7", 951),
    ("b08_secret_entity_to_string.mlog", "B8", 951),
    ("b09_error_text_value.mlog", "B9", 955),
];

/// THE DOCS GATE (the №948 mutation target): the boundary table stands,
/// the overclaim is gone, every document points at it.
#[test]
fn n948_threat_model_carries_the_boundary_table_and_no_overclaim() {
    let tm = fs::read_to_string(repo_root().join("docs").join("threat-model.md"))
        .expect("threat-model.md readable");
    assert!(
        tm.contains("### The inter-pattern boundary"),
        "the boundary table section is gone from threat-model.md — restore it (№948)"
    );
    assert!(
        !tm.to_lowercase().contains("interprocedural taint is tracked"),
        "the pre-№948 overclaim 'interprocedural taint IS tracked' returned to threat-model.md — the honest-boundary mutation gate (№948)"
    );
    for row in [
        "B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12",
    ] {
        assert!(
            tm.contains(&format!("| {} |", row)),
            "threat-model boundary table lost row {}",
            row
        );
    }

    let lm = fs::read_to_string(repo_root().join("docs").join("limitations.md"))
        .expect("limitations.md readable");
    assert!(
        lm.contains("ONE NARROW LANE"),
        "limitations.md lost the honest interprocedural row (№948)"
    );
    assert!(
        !lm.contains("Interprocedural taint — bounded depth, now CONFIGURABLE"),
        "limitations.md restored the pre-№948 overstated row — the honest-boundary mutation gate (№948)"
    );

    let rm = fs::read_to_string(repo_root().join("README.md")).expect("README.md readable");
    assert!(
        rm.contains("within a single pattern"),
        "README lost the honest first-line boundary (№948)"
    );

    assert!(
        repo_root()
            .join("docs")
            .join("security")
            .join("advisory-2026-11-12.md")
            .exists(),
        "the security advisory file is gone (№948)"
    );
    let sec = fs::read_to_string(repo_root().join("SECURITY.md")).expect("SECURITY.md readable");
    assert!(
        sec.contains("advisory-2026-11-12.md"),
        "SECURITY.md lost the advisory link (№948)"
    );
}

/// Every table row has its pin fixture on disk (the bijection).
#[test]
fn n948_every_boundary_row_has_its_pin_fixture() {
    for (fixture, row, naryad) in SINGLE_FILE_PINS {
        let path = boundary_dir().join(fixture);
        assert!(
            path.exists(),
            "row {} lost its pin fixture {}",
            row,
            fixture
        );
        let expected_path = path.with_extension("expected");
        assert!(
            expected_path.exists(),
            "row {} lost its .expected pin ({})",
            row,
            expected_path.display()
        );
        let expected = fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", expected_path.display(), e));
        assert!(
            expected.contains(&format!("row: {}", row))
                && expected.contains(&format!("flip: №{}", naryad)),
            "{}: the .expected pin must carry 'row: {}' and 'flip: №{}' (№948 contract)",
            expected_path.display(),
            row,
            naryad
        );
    }
    for (dir, files) in [
        ("b10_check_root", vec!["leaf.mlog", "root.mlog"]),
        (
            "b11_capability_import",
            vec!["entry.mlog", "root.mlog", "dep/evil.mlog"],
        ),
    ] {
        for f in files {
            assert!(
                boundary_dir().join(dir).join(f).exists(),
                "boundary pin {}/{} is missing",
                dir,
                f
            );
        }
    }
}

/// The gap is REAL today: every single-file pin passes `mlog check`
/// with zero errors (the CURRENT behavior the table documents).
/// The fixing naryad (№950/№951) FLIPS its row here into the negative.
#[test]
fn n948_boundary_pins_pass_check_today_the_gap_is_real() {
    for (fixture, row, _) in SINGLE_FILE_PINS {
        let src = fs::read_to_string(boundary_dir().join(fixture))
            .unwrap_or_else(|e| panic!("cannot read {}: {}", fixture, e));
        let result = metalogos::check_program(&src).unwrap_or_else(|e| {
            panic!(
                "row {}: parse failed — the pin fixture itself is broken: {}",
                row, e
            )
        });
        assert!(
            result.errors.is_empty(),
            "row {} ({}): expected the CURRENT behavior — check passes (the documented gap); got {:?}",
            row,
            fixture,
            result.errors.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>()
        );
    }
}

/// B10 pin: `check_program_with_root` EXECUTES the root today — the
/// root's `write_file` side effect lands BEFORE the analysis, without
/// the Category-A gates (the audit's C-2). №953 flips this pin: the
/// file must NOT appear, zero execution.
#[test]
fn n948_b10_check_root_executes_the_root_today() {
    let tmp = std::env::temp_dir().join(format!("n948_b10_{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("tmp dir");
    let root_src = fs::read_to_string(boundary_dir().join("b10_check_root").join("root.mlog"))
        .expect("root fixture");
    let root_path = tmp.join("root.mlog");
    fs::write(&root_path, &root_src).expect("write root fixture");

    let leaf_src = fs::read_to_string(boundary_dir().join("b10_check_root").join("leaf.mlog"))
        .expect("leaf fixture");
    // The executed root writes relative to the CHECKER'S CWD (the audit
    // observed the same on the release binary) — clean around the probe.
    let cwd_side_effect = std::env::current_dir()
        .expect("cwd")
        .join("EXECUTED_BY_CHECK.txt");
    let _ = fs::remove_file(&cwd_side_effect);
    let result = metalogos::check_program_with_root(&leaf_src, Some(root_path.as_path()));
    let executed = cwd_side_effect.exists();
    let _ = fs::remove_file(&cwd_side_effect);
    let _ = fs::remove_dir_all(&tmp);
    assert!(
        result.is_ok(),
        "the pin fixture pair itself fails check — the fixture is broken"
    );
    assert!(
        executed,
        "B10 pin flipped early: check --root no longer executes the root — the №953 fix is landing; flip this pin to the negative in №953"
    );
}

/// B11 pin: an imported module declaring its own `profile capability`
/// passes check today (the audit's H-2 — the permit roster unions the
/// import). №954 flips this pin: the declaration in the import is the
/// refusal.
#[test]
fn n948_b11_capability_import_passes_check_today() {
    let tmp = std::env::temp_dir().join(format!("n948_b11_{}", std::process::id()));
    let dep_dir = tmp.join("dep");
    fs::create_dir_all(&dep_dir).expect("tmp dep dir");
    let b11 = boundary_dir().join("b11_capability_import");
    fs::copy(b11.join("entry.mlog"), tmp.join("entry.mlog")).expect("copy entry");
    fs::copy(b11.join("root.mlog"), tmp.join("root.mlog")).expect("copy root stub");
    fs::copy(b11.join("dep").join("evil.mlog"), dep_dir.join("evil.mlog")).expect("copy dep");

    let entry_src = fs::read_to_string(tmp.join("entry.mlog")).expect("entry fixture");
    // The checked file + a stub root in the SAME directory (the imports
    // resolve against the root's directory — the office CI form shape,
    // without the self-root duplicate-merge artifact).
    let result =
        metalogos::check_program_with_root(&entry_src, Some(tmp.join("root.mlog").as_path()));
    let _ = fs::remove_dir_all(&tmp);
    let result = result.expect("parse ok");
    assert!(
        result.errors.is_empty(),
        "B11 pin flipped early: the imported capability profile is already refused — the №954 fix is landing; flip this pin to the negative in №954: {:?}",
        result.errors.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>()
    );
}
