// ── Naryad №966 (issue #1954, the B7 line — the owner decision 13–14.11,
// P1, [security][compiler][adr]): the flow output's integrity half —
// the measured package, NOT a gate.
//
// The owner decision 13–14.11 split the B7 line in two: №965 landed the
// flow edge on the CONFIDENTIALITY axis, and the integrity half waits
// for a measured decision — a flow legitimately ending in model, remote
// or own-file data is the language's primary idiom (`p100_mcp_echo`,
// `n5_html_tender`, the self-hosted parser and lexer), and gating both
// axes refuses every such flow. This naryad measures the price (ADR-0154
// §13, PROPOSED: three sweeps on the tree `53f0fdd9` — 7 / 30 / 23
// carriers, 0 / 16 marginal refusals, the 16 idiom files named), records
// the boundary as threat-model row B13 with its own pin, and puts the
// three candidate idiom forms with their prices in front of the owner.
//
// WHAT THIS NARYAD DOES NOT DO: no gate is landed. The tree refuses the
// integrity half NOWHERE; the pin below asserts the CURRENT behavior and
// exists to catch a silent change of mind.
//
// MUTATION CONTRACT (the «сделано, когда» pin): gate the flow output
// edge on BOTH axes — replace the Flow arm's `edge.conf` check with
// `clearance_failure("print", &edge)` (`src/semantic.rs`) — and
// `n966_b13_idiom_pin_passes_today` and `n966_the_call_llm_flow_output_
// passes_inline` go green-to-red: the check refuses the idiom shape with
// `[HTML_INJECTION]` (the frozen print-channel integrity class —
// `sink_check_id`, no new check_id, §12.4). The mechanical form is
// `scripts/mutation_verify_966.sh`; the restore is a BACKUP-COPY
// restore, never `git checkout --` (the №953 lesson).
//
// The measurement numbers ride the DOCUMENTS (ADR-0154 §13.2, the B13
// row), not this file: a corpus change moves them, and the docs gate
// below forces that move to happen in the same PR as the corpus change —
// the №956 discipline.

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

/// THE PIN OF THE CURRENT BEHAVIOR (the mutation target): the B13
/// fixture — a flow ending in `call_llm` output, `public, untrusted` —
/// passes `mlog check` TODAY. The confidentiality axis does not refuse
/// an untrusted-PUBLIC flow output, and the integrity half is not gated
/// on the flow edge (the Flow arm reads `seen.conf`, never
/// `seen.integrity`). If this test goes red without a naryad of its own,
/// the boundary moved silently.
#[test]
fn n966_b13_idiom_pin_passes_today() {
    let src = fs::read_to_string(boundary_dir().join("b13_flow_output_integrity_idiom.mlog"))
        .expect("the b13 pin fixture is missing — restore it (№966)");
    let result =
        metalogos::check_program(&src).unwrap_or_else(|e| panic!("the b13 pin must parse: {e}"));
    assert!(
        result.errors.is_empty(),
        "the b13 idiom pin must pass check TODAY (the owner decision 13–14.11: \
         the integrity half is measured, not gated); got: {:?}",
        result
            .errors
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>()
    );
}

/// The same shape inline — the canonical idiom program, self-contained
/// so the mutation contract needs no fixture to be read.
#[test]
fn n966_the_call_llm_flow_output_passes_inline() {
    let src = "pattern P(x: String) -> String { return call_llm(\"summarize\", x) }\n\
               flow Main { input: String = \"doc\" -> P -> output }";
    let result = metalogos::check_program(src)
        .unwrap_or_else(|e| panic!("the idiom program must parse: {e}"));
    let rendered = result
        .errors
        .iter()
        .map(|e| format!("{e:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rendered.is_empty(),
        "an untrusted-PUBLIC flow output must pass (the owner decision 13–14.11; \
         the integrity half is №966's measured row B13); got: {rendered}"
    );
}

/// The CONTROL: the confidentiality gate the pin does NOT silence. A
/// flow ending in a SOURCE value (`env`) still refuses with the frozen
/// print-channel class — the pin is the integrity boundary, not a
/// general silencer.
#[test]
fn n966_private_flow_output_still_refuses() {
    let src = "pattern P(x: String) -> String { return env(\"SP\") }\n\
               flow Main { input: String = \"x\" -> P -> output }";
    let result = metalogos::check_program(src)
        .unwrap_or_else(|e| panic!("the control program must parse: {e}"));
    let rendered = result
        .errors
        .iter()
        .map(|e| format!("{e:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rendered.contains("PII_EGRESS_OUTPUT"),
        "the confidentiality gate must keep refusing a private flow output \
         (№965, landed); got: {}",
        if rendered.is_empty() {
            "<no errors>"
        } else {
            &rendered
        }
    );
}

/// THE DOCS GATE: the measured package lives in the documents, and the
/// boundary is a row of its own — not an edit to B7.
#[test]
fn n966_the_integrity_package_is_measured_in_the_docs() {
    let tm = fs::read_to_string(repo_root().join("docs").join("threat-model.md"))
        .expect("threat-model.md readable");
    assert!(
        tm.contains("| B13 | OPEN BY DECISION — №966"),
        "threat-model.md lost the B13 row — the measured boundary must be a row \
         of its own (№966), not an edit to B7"
    );
    for number in [
        "7 files carrying the confidentiality flow finding, 30 with both \
                   axes gated, 23 integrity-only",
        "16 on the integrity axis",
    ] {
        assert!(
            tm.contains(number),
            "the B13 row lost the measured numbers ({number}) — re-measure in the \
             SAME PR as any corpus change (the №956 discipline)"
        );
    }

    let adr = fs::read_to_string(
        repo_root()
            .join("docs")
            .join("adr")
            .join("0154-label-lattice.md"),
    )
    .expect("ADR-0154 readable");
    assert!(
        adr.contains("## 13. Appendix (№966)") && adr.contains("PROPOSED"),
        "ADR-0154 §13 (the measured package) is gone or its status was raised — \
         the promotion is the OWNER's act (канон §13.14 п.6), not the executor's"
    );
    for form in [
        "(а) The annotation on the output edge",
        "(б) The declaration on the flow head",
        "(в) The program-level profile key",
    ] {
        assert!(
            adr.contains(form),
            "ADR-0154 §13 lost one of the three candidate forms ({form}) — the \
             decision package must carry all three with their prices (№966)"
        );
    }
    assert!(
        adr.contains("marginal refusals"),
        "ADR-0154 §13 lost the marginal-refusal definition — the measurement \
         carries BOTH definitions (the carriers and the verdict flips)"
    );
}

/// The agree-test: the two documents tell one story. B7 says the
/// integrity half is open and points at B13; B13's pin carries its row
/// and its flip target; the .expected pin is wired.
#[test]
fn n966_the_b7_row_and_the_b13_pin_agree() {
    let tm = fs::read_to_string(repo_root().join("docs").join("threat-model.md"))
        .expect("threat-model.md readable");
    assert!(
        tm.contains("| B7 | FIXED"),
        "the B7 row is not FIXED — the confidentiality half landed with №965; \
         the rows must agree (№966)"
    );
    assert!(
        tm.contains("recorded as its own row B13 below"),
        "the B7 row lost its pointer to the B13 row — the two rows must agree (№966)"
    );

    let expected =
        fs::read_to_string(boundary_dir().join("b13_flow_output_integrity_idiom.expected"))
            .expect("the b13 .expected pin is missing");
    assert!(
        expected.contains("row: B13") && expected.contains("flip: №966"),
        "the b13 .expected pin must carry 'row: B13' and 'flip: №966' (the №948 \
         contract form)"
    );
}
