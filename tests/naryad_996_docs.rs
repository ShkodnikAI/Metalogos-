//! №996 (ADR-0200 W8, the dispatch wave В103 — W-AG) — the doc-sync
//! machine: the line's user surface (W1–W7) is covered by the public
//! docs and the values come from the code/the §11 registry, not from
//! memory. Pins: the REFERENCE CLI rows (the W4–W7 verbs + the global
//! trace flag), the §11.3 environment variables, the README honest
//! words (§6.1 not-quality, §6.3 tamper-evident), the CHANGELOG line
//! section + the METALOGOS_ATTEST_STRICT Migration note, the
//! PLAN-SUMMARY wave row, the ADR-0200 §12 record.
//!
//! The W7 census machine (`tests/naryad_995_audit.rs`) keeps answering
//! for the machine words — the two machines compose: the census holds
//! the src↔registry two-way, this one holds the docs↔registry surface.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[allow(clippy::disallowed_methods)]
fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The REFERENCE CLI rows cover the W4–W7 verbs the reference lacked
/// (the W1/W2′/W3 rows predate W8 and stay pinned by their own history).
#[test]
fn reference_covers_the_line_cli_surface() {
    let s = read("REFERENCE.md");
    for verb in [
        "mlog attest verify",
        "mlog curriculum build",
        "mlog distill freeze",
        "mlog distill unfreeze",
        "mlog distill status",
        "mlog distill enable-differential",
        "--trace-distill",
    ] {
        assert!(
            s.contains(verb),
            "REFERENCE.md lacks the line's CLI surface: {verb}"
        );
    }
    // The honest flags on the rows: the gate verdicts and the never-cut
    // words ride the row texts, not a separate promise.
    assert!(
        s.contains("[ATTESTATION_ENABLEMENT_UNFILTERED]")
            && s.contains("[ATTESTATION_ENABLEMENT_TEACHER_POLICY]"),
        "the enablement row names the machine codes (the §11 registry form)"
    );
    assert!(
        s.contains("Zero writes on refusal"),
        "the enablement row carries the zero-writes contract"
    );
    assert!(
        s.contains("provenance-consistent") || s.contains("provenance and consistency"),
        "the attest row keeps the §6.1 semantics (provenance, not quality)"
    );
}

/// The §11.3 environment variables live in the REFERENCE env table with
/// the recorded defaults (threshold 2, window 3600, cooldown 86400).
#[test]
fn reference_env_table_covers_the_line_variables() {
    let s = read("REFERENCE.md");
    for env in [
        "METALOGOS_ATTEST_STRICT",
        "METALOGOS_ATTEST_ALLOW_STALE",
        "METALOGOS_DISTILL_FREEZE_THRESHOLD",
        "METALOGOS_DISTILL_FREEZE_WINDOW_SECS",
        "METALOGOS_DISTILL_AUTO_UNFREEZE_COOLDOWN_SECS",
        "METALOGOS_TRACE_DISTILL",
    ] {
        assert!(
            s.contains(&format!("| `{env}` |")),
            "REFERENCE.md's env table lacks the line's variable: {env}"
        );
    }
    for (default, where_) in [
        ("default 2", "threshold"),
        ("default 3600", "window"),
        ("default 86400", "cooldown"),
    ] {
        assert!(
            s.contains(default),
            "the {where_} row carries the recorded default {default} (the values \
             come from the code, not from memory)"
        );
    }
}

/// The README names the line with the honest words: tamper-evident
/// (§6.3) and never quality (§6.1).
#[test]
fn readme_carries_the_honest_line_words() {
    let s = read("README.md");
    assert!(
        s.contains("tamper-evident"),
        "the README carries the §6.3 word (tamper-evident, not tamper-proof)"
    );
    assert!(
        s.contains("never quality"),
        "the README carries the §6.1 word (provenance and consistency, never \
         quality — ok: true is not a quality claim)"
    );
    assert!(
        s.contains("mlog attest verify"),
        "the README points at the line's verify surface"
    );
}

/// The CHANGELOG carries the line's surface section and the
/// METALOGOS_ATTEST_STRICT Migration note (the §6.6 transitional window
/// is part of the W1 contract).
#[test]
fn changelog_carries_line_section_and_migration_note() {
    let s = read("CHANGELOG.md");
    assert!(
        s.contains("### The provable-lowering line (ADR-0200) — the surface and the migration"),
        "the line's surface section is in the CHANGELOG"
    );
    assert!(
        s.contains("**Migration — `METALOGOS_ATTEST_STRICT`:**"),
        "the Migration note for METALOGOS_ATTEST_STRICT is present"
    );
    assert!(
        s.contains("TRANSITIONAL window")
            || s.contains("TRANSITIONAL window, not a resting state")
            || s.contains("transitional window"),
        "the note names the transitional window (decision 4, variant б)"
    );
    // The section recaps the line's slots without promising quality.
    for slot in ["W1 (№825", "W4 (№989", "W5 (№990", "W6 (№994", "W7 (№995"] {
        assert!(s.contains(slot), "the line section recaps {slot}");
    }
}

/// The PLAN-SUMMARY wave ledger carries the line's row (the
/// В100–В103 dispatches, the machine-record facts only).
#[test]
fn plan_summary_carries_the_line_wave_row() {
    let s = read("docs/PLAN-SUMMARY.md");
    assert!(
        s.contains("Wave 31 — the provable-lowering line"),
        "the wave ledger carries the line's row"
    );
    assert!(
        s.contains("В103 gh#2049") && s.contains("#2048"),
        "the row names the dispatches and the PRs (the machine-record facts)"
    );
    assert!(
        s.contains("W9") && s.contains("numbers measured only"),
        "the row states the remaining W9 with its own discipline word"
    );
}

/// The ADR-0200 §12 record exists and names the W8 deliverables.
#[test]
fn adr_carries_the_w8_record() {
    let s = read("docs/adr/0200-provable-lowering.md");
    assert!(
        s.contains("## §12. The W8 record — documentation and the indexes (№996, 2026-11-23)"),
        "the §12 W8 record is in the ADR"
    );
    for deliverable in [
        "REFERENCE.md",
        "Migration note",
        "doc-sync machine",
        "llms.txt №1-№995 →",
    ] {
        assert!(
            s.contains(deliverable),
            "the §12 record names the deliverable: {deliverable}"
        );
    }
    // §11 must be intact (the W8 pass touches nothing in it — the
    // census machine's subject).
    assert!(
        s.contains("### The check_id registry (§11.1)")
            && s.contains("### The threat-model classes (§11.5)"),
        "§11 stays intact under the W8 pass"
    );
}
