// ── №948 (issue #1915): the honest-boundary pins (the audit 12.11) ─────
//
// The threat-model §The inter-pattern boundary tabulates every flow the
// analyzer/executable surface does NOT stop on v0.39.0. Each row carries
// a pin fixture in examples/boundaries/ whose expected behavior is the
// CURRENT one (`mlog check` passes — the gap is real). The FIXING naryad
// flips its row's pin into a blocking leak-suite negative:
//   №950 flips B1–B4, №951 flips B5–B8, №953 flips B10 (DONE —
//   n948_b10_check_root_is_static_zero_execution below), №954 flips B11
//   (DONE — n948_b11_imported_capability_profile_is_the_refusal),
//   №952 lands the B12 machinery, B9 stays pass-at-check (the №955 fix
//   is runtime: the 500 body and the error text).
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

/// The single-file pins: (fixture, table row, fixing naryad). The
/// bijection list — every row keeps its fixture and its `.expected` pin
/// for the whole life of the table, flipped or not.
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

/// The rows whose gap is STILL OPEN — the pins that must keep passing
/// `mlog check` until their own naryad lands. B1–B4 left this list when
/// №950 flipped them (see `n948_b01_b04_interprocedural_boundary_...`
/// below); B5–B8 leave it with №951, B9 with №955. A row removed from
/// here without a flipped negative taking its place is a silently
/// abandoned pin — the table would then claim a gap nothing measures.
const STILL_OPEN_PINS: &[(&str, &str, u32)] = &[
    // B7 stays open BY DECISION, not by omission: closing the flow output
    // exposed three pre-existing lattice questions (the axis the output is
    // gated on — 8 corpus files on confidentiality alone against 29 on
    // both; a sink's RETURN value inheriting its arguments' labels, so
    // `quarantine_write`'s receipt comes out poisoned; and three of the
    // project's own examples that print private data from a flow, each
    // pinned as clean by an existing test). Each is an owner decision, and
    // the fix is held as a patch until it is made.
    ("b07_flow_output.mlog", "B7", 951),
    ("b09_error_text_value.mlog", "B9", 955),
];

/// The rows FLIPPED by №951 — the in-pattern losses, each now a blocking
/// refusal pinned by the class that actually fires.
const FLIPPED_951_PINS: &[(&str, &str, &str)] = &[
    ("b05_each_accumulator.mlog", "B5", "PII_EGRESS_OUTPUT"),
    ("b06_secret_in_get_url.mlog", "B6", "PII_EGRESS_NETWORK"),
    (
        "b08_secret_entity_to_string.mlog",
        "B8",
        "PII_EGRESS_OUTPUT",
    ),
];

/// The rows FLIPPED by №950 — each now a blocking refusal, pinned by the
/// diagnostic class that actually fires (the ADR-0154 §12.4 contract: the
/// existing frozen vocabulary, no new `check_id`).
const FLIPPED_950_PINS: &[(&str, &str, &str)] = &[
    (
        "b01_source_in_pattern_return.mlog",
        "B1",
        "PII_EGRESS_OUTPUT",
    ),
    ("b02_param_to_sink.mlog", "B2", "PII_EGRESS_OUTPUT"),
    ("b03_html_via_pattern.mlog", "B3", "HTML_INJECTION"),
    ("b04_exec_via_pattern.mlog", "B4", "UNTRUSTED_EXEC_DECISION"),
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

/// The gap is REAL today: every STILL-OPEN pin passes `mlog check` with
/// zero errors (the CURRENT behavior the table documents). The fixing
/// naryad flips its row out of this list and into a negative below.
#[test]
fn n948_boundary_pins_pass_check_today_the_gap_is_real() {
    for (fixture, row, _) in STILL_OPEN_PINS {
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

/// B1–B4 pins — FLIPPED TO LEAK-SUITE NEGATIVES by №950 (the №948
/// contract): the interprocedural summaries (ADR-0154 §12) carry the
/// callee's internal sources into the call site's label (а) and check the
/// argument against the clearance the callee's parameter requires (б).
/// Each of these four fixtures passed `mlog check` on the PUBLISHED
/// v0.39.0 while leaking at runtime — the audit of 12.11 proved all four
/// on the release binary. The regression gate: if the open world ever
/// returns, the refusals disappear and this pin goes RED.
#[test]
fn n948_b01_b04_interprocedural_boundary_is_the_refusal() {
    for (fixture, row, check_id) in FLIPPED_950_PINS {
        let src = fs::read_to_string(boundary_dir().join(fixture))
            .unwrap_or_else(|e| panic!("cannot read {}: {}", fixture, e));
        let result = metalogos::check_program(&src)
            .unwrap_or_else(|e| panic!("row {}: parse failed — the pin is broken: {}", row, e));
        let rendered = result
            .errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains(check_id),
            "row {} ({}): the interprocedural boundary must refuse with {} — the open world is back (№950, ADR-0154 §12); got: {}",
            row,
            fixture,
            check_id,
            if rendered.is_empty() {
                "<no errors — check passes again>"
            } else {
                &rendered
            }
        );
    }
}

/// B5, B6, B8 pins — FLIPPED TO LEAK-SUITE NEGATIVES by №951 (the №948
/// contract): the in-pattern label losses.
///
/// B5 — the `each` body's raises now fold back into the loop-exit
/// environment, exactly as the `while` arm already did (the asymmetry WAS
/// the gap). B6 — the address/query family of request builtins gates its
/// arguments on the confidentiality axis, so a secret in a URL refuses.
/// B8 — an entity seeds from its explicit annotation, its declared type
/// AND its initializer's label, not from string literals alone; the
/// sharpest half of B8 was that `entity k: String<private>` was ignored
/// outright.
#[test]
fn n948_b05_b06_b08_in_pattern_losses_are_the_refusal() {
    for (fixture, row, check_id) in FLIPPED_951_PINS {
        let src = fs::read_to_string(boundary_dir().join(fixture))
            .unwrap_or_else(|e| panic!("cannot read {}: {}", fixture, e));
        let result = metalogos::check_program(&src)
            .unwrap_or_else(|e| panic!("row {}: parse failed — the pin is broken: {}", row, e));
        let rendered = result
            .errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains(check_id),
            "row {} ({}): the in-pattern loss must refuse with {} (№951); got: {}",
            row,
            fixture,
            check_id,
            if rendered.is_empty() {
                "<no errors — the loss is back>"
            } else {
                &rendered
            }
        );
    }
}

/// The row FLIPPED by №952 — the value-label second line: the fixture is
/// a blocking leak-suite negative (the static line refuses with the B1
/// class), and its .expected documents the RUNTIME twin — the same
/// program refuses with [SINK_CLEARANCE_RUNTIME] on both backends under
/// the test-only static-off flag (tests/naryad_952_value_labels.rs).
const FLIPPED_952_PINS: &[(&str, &str, &str)] =
    &[("b12_runtime_value_label.mlog", "B12", "PII_EGRESS_OUTPUT")];

/// B12 pin — FLIPPED TO THE LEAK-SUITE NEGATIVE by №952 (the №948
/// contract): the runtime value-label second line's fixture refuses at
/// check with the B1 class; the RUNTIME proof (static off, both
/// backends) lives in tests/naryad_952_value_labels.rs.
#[test]
fn n948_b12_value_labels_second_line_is_the_refusal() {
    for (fixture, row, check_id) in FLIPPED_952_PINS {
        let src = fs::read_to_string(boundary_dir().join(fixture))
            .unwrap_or_else(|e| panic!("cannot read {}: {}", fixture, e));
        let result = metalogos::check_program(&src)
            .unwrap_or_else(|e| panic!("row {}: parse failed — the pin is broken: {}", row, e));
        let rendered = result
            .errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains(check_id),
            "row {} ({}): the value-label fixture must refuse with {} (№952 static line); got: {}",
            row,
            fixture,
            check_id,
            if rendered.is_empty() {
                "<no errors — the second line is back to silence>"
            } else {
                &rendered
            }
        );
    }
}

/// B10 pin — FLIPPED TO THE LEAK-SUITE NEGATIVE by №953 (PR #1931, the
/// №948 contract): `check_program_with_root` is STATIC now — zero
/// execution, no `Interpreter` on the path. The regression gate: if the
/// C-2 execution bug ever returns, the root's `write_file` side effect
/// lands BEFORE the analysis and this pin goes RED. The root's own
/// leaks are REPORTED instead of executed (the honest bonus pinned by
/// `tests/naryad_953_check_root_static.rs` with the SECRET_LEAK probe).
#[test]
fn n948_b10_check_root_is_static_zero_execution() {
    let tmp = std::env::temp_dir().join(format!("n948_b10_{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("tmp dir");
    let root_src = fs::read_to_string(boundary_dir().join("b10_check_root").join("root.mlog"))
        .expect("root fixture");
    let root_path = tmp.join("root.mlog");
    fs::write(&root_path, &root_src).expect("write root fixture");

    let leaf_src = fs::read_to_string(boundary_dir().join("b10_check_root").join("leaf.mlog"))
        .expect("leaf fixture");
    // The v0.39.0 bug executed the root relative to the CHECKER'S CWD
    // (the audit observed the same on the release binary) — clean around
    // the probe, then prove the file does NOT appear.
    let cwd_side_effect = std::env::current_dir()
        .expect("cwd")
        .join("EXECUTED_BY_CHECK.txt");
    let _ = fs::remove_file(&cwd_side_effect);
    let result = metalogos::check_program_with_root(&leaf_src, Some(root_path.as_path()));
    let executed = cwd_side_effect.exists();
    let _ = fs::remove_file(&cwd_side_effect);
    let _ = fs::remove_dir_all(&tmp);
    let result = result.expect("parse ok — the pin fixture pair itself must stay well-formed");
    assert!(
        !executed,
        "check --root EXECUTED the root (the side-effect file EXECUTED_BY_CHECK.txt appeared) — the C-2 bug is back; the B10 leak-suite negative (№953) is violated"
    );
    // The office `check --root` form stays CLEAN on harmless literals —
    // the static resolution must not invent false positives. (The
    // REPORTING of a genuinely leaking root — SECRET_LEAK — is pinned by
    // tests/naryad_953_check_root_static.rs.)
    assert!(
        result.errors.is_empty(),
        "the b10 pair is harmless literals — the static check --root must complete cleanly, got: {:?}",
        result.errors.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>()
    );
}

/// B11 pin — FLIPPED TO THE LEAK-SUITE NEGATIVE by №954 (the №948
/// contract): an imported module declaring its own `profile capability`
/// is the loud `[CAPABILITY_PROFILE_IN_IMPORT]` refusal — the permit
/// roster NEVER unions the import. The regression gate: if the union
/// ever returns, the refusal disappears and this pin goes RED.
#[test]
fn n948_b11_imported_capability_profile_is_the_refusal() {
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
    let rendered = result
        .errors
        .iter()
        .map(|e| format!("{:?}", e))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rendered.contains("CAPABILITY_PROFILE_IN_IMPORT"),
        "the imported capability profile must be the loud CAPABILITY_PROFILE_IN_IMPORT refusal — the permit roster must never union the import (№954), got: {}",
        if rendered.is_empty() { "<no errors — the union is back>" } else { &rendered }
    );
}
