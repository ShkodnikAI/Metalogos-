// ── Naryad №967 (issue #1954 — the same-issue form, gh#1954; the B13
// line, ADR-0154 §13.8 — form (а), the owner decision 16.11, P1,
// [security][compiler]): the flow output's integrity half — GATED, with
// the §3 terminator annotation as the CHECKABLE idiom claim. ──
//
// The lineage: №965 landed the flow edge on the CONFIDENTIALITY axis
// (the owner's named choice); №966 measured the integrity half (ADR-0154
// §13: three sweeps, 7/30/23 carriers, 0/16 marginal refusals, the 16
// idiom files), recorded the boundary as threat-model row B13 with its
// own pin, and put the three candidate forms in front of the owner. The
// owner chose form (а) §13.4 — the `label_ann` on the pipeline
// terminator (`-> output<public, untrusted>`, the CHECKABLE claim) —
// and commissioned the implementing naryad (§13.7 = the decision, §13.8
// = the design draft). This naryad lands the design verbatim:
//
//   1. the gate: the Flow arm answers to `clearance_failure("print",
//      &edge)` when the terminator is NOT annotated — the FROZEN classes
//      (HTML_INJECTION for untrusted-public, PII_EGRESS_OUTPUT for
//      private; no new check_id, §12.4);
//   2. the claim: `-> output<public, untrusted>` declares the INTEGRITY
//      display intent (model, remote, own-file output printed on
//      purpose); the declared set must COVER the computed edge — a
//      weaker claim (or any conf word other than `public`) is the loud
//      §12.2 mismatch error (LABEL_ANNOTATION_MISMATCH): the annotation
//      cannot become a silencer, and `public, trusted` remains reachable
//      only by an edge that actually carries it;
//   3. the confidentiality half is NOT declareable away — a private
//      edge refuses with or without the annotation (№965 stays).
//
// MUTATION CONTRACT (the «сделано, когда» pins, `scripts/mutation_verify_966.sh`):
//   M3 — drop the integrity arm (the unannotated edge back to the conf
//        half only) -> the pin asserting the refusal FAILS (the boundary
//        moved back to silence — loudly);
//   M4 — make the annotation a silencer (skip the §12.2 claim check) ->
//        the weaker-claim negative FAILS (the silencer would pass).
//   M1/M2 (the №966 forms) are rewritten against the landed gate: M1 —
//   the both-axes clearance on the unannotated edge is the LANDED code,
//   so the mutation is its REMOVAL (M3's shape); M2 — the whole
//   output-edge gate dropped, the conf control reds.
// The restore is a BACKUP-COPY restore, never `git checkout --` (the
// №953 lesson).

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

fn rendered_errors(src: &str, what: &str) -> String {
    let result =
        metalogos::check_program(src).unwrap_or_else(|e| panic!("{}: parse failed: {}", what, e));
    result
        .errors
        .iter()
        .map(|e| format!("{e:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// THE GATED NEGATIVE (the flip of the №966 current-behavior pin): the
/// B13 fixture — a flow ending in `call_llm` output, `public, untrusted`,
/// UNANNOTATED — refuses with the frozen print-channel class, and the
/// refusal names the idiom form (the §13.4 criterion: the refusal stays
/// readable to an author who did not declare the idiom).
#[test]
fn n966_b13_unannotated_refuses() {
    let src = fs::read_to_string(boundary_dir().join("b13_flow_output_integrity_idiom.mlog"))
        .expect("the b13 pin fixture is missing — restore it (№966)");
    let rendered = rendered_errors(&src, "the b13 pin fixture");
    assert!(
        rendered.contains("HTML_INJECTION"),
        "the UNANNOTATED flow output must refuse with the frozen print-channel class \
         (№967 — the integrity half is gated); got: {}",
        if rendered.is_empty() {
            "<no errors — the B13 gate is back to silence>"
        } else {
            rendered.as_str()
        }
    );
    assert!(
        rendered.contains("declare `-> output<public, untrusted>`"),
        "the refusal must name the idiom form (ADR-0154 §13.8 п.3 — the §13.4 \
         criterion); got: {}",
        rendered
    );
}

/// THE ANNOTATED POSITIVE: the same shape WITH the true idiom claim —
/// the display of `public, untrusted` model output is INTENDED and
/// declared — passes.
#[test]
fn n966_b13_annotated_passes() {
    let src = "pattern Summarize(x: String) -> String { return call_llm(\"summarize\", x) }\n\
               flow Main { input: String = \"doc\" -> Summarize -> output<public, untrusted> }";
    let rendered = rendered_errors(src, "the annotated idiom");
    assert!(
        rendered.is_empty(),
        "the flow with the TRUE terminator claim must pass (№967 — form (а): the \
         annotation declares the integrity display intent); got: {rendered}"
    );
}

/// The same negative, self-contained inline — the canonical idiom
/// program without the annotation.
#[test]
fn n966_the_call_llm_flow_output_unannotated_refuses() {
    let src = "pattern P(x: String) -> String { return call_llm(\"summarize\", x) }\n\
               flow Main { input: String = \"doc\" -> P -> output }";
    let rendered = rendered_errors(src, "the unannotated idiom");
    assert!(
        rendered.contains("HTML_INJECTION"),
        "an unannotated untrusted-PUBLIC flow output must refuse (№967 — the B13 \
         gate); got: {rendered}"
    );
}

/// THE §12.2 UNSATISFIED-CLAIM NEGATIVE: an annotation WEAKER than the
/// computed edge — `public, trusted` claimed over an untrusted edge —
/// is the silencer attempt, refused loudly with the annotation-discipline
/// class (the mutation target M4: skip the claim check and this pin
/// reds).
#[test]
fn n966_the_weaker_claim_refuses() {
    let src = "pattern P(x: String) -> String { return call_llm(\"summarize\", x) }\n\
               flow Main { input: String = \"doc\" -> P -> output<public, trusted> }";
    let rendered = rendered_errors(src, "the weaker claim");
    assert!(
        rendered.contains("LABEL_ANNOTATION_MISMATCH"),
        "a declared set weaker than the computed edge must refuse with the loud \
         §12.2 error (№967 — the annotation cannot become a silencer); got: {}",
        if rendered.is_empty() {
            "<no errors — the annotation became a silencer>"
        } else {
            rendered.as_str()
        }
    );
}

/// THE §12.2 CLAIM DISCIPLINE on the confidentiality axis: the
/// terminator declaration may not move the conf half — a conf word other
/// than `public` is the loud mismatch error (the annotation is the
/// INTEGRITY display intent, not a confidentiality waiver).
#[test]
fn n966_the_conf_word_refuses() {
    let src = "pattern P(x: String) -> String { return call_llm(\"summarize\", x) }\n\
               flow Main { input: String = \"doc\" -> P -> output<private, untrusted> }";
    let rendered = rendered_errors(src, "the conf-word claim");
    assert!(
        rendered.contains("LABEL_ANNOTATION_MISMATCH"),
        "a non-public conf word on the output terminator must refuse (№967 — the \
         confidentiality half is not declareable); got: {}",
        if rendered.is_empty() {
            "<no errors>"
        } else {
            rendered.as_str()
        }
    );
}

/// The CONTROL (the №965 confidentiality gate, unchanged by №967): a
/// flow ending in a SOURCE value (`env`) refuses with the frozen class,
/// and the refusal does NOT advise a terminator declaration (the p08
/// red-team dead-end — the private edge is not declareable).
#[test]
fn n966_private_flow_output_still_refuses() {
    let src = "pattern P(x: String) -> String { return env(\"SP\") }\n\
               flow Main { input: String = \"x\" -> P -> output }";
    let rendered = rendered_errors(src, "the control program");
    assert!(
        rendered.contains("PII_EGRESS_OUTPUT"),
        "the confidentiality gate must keep refusing a private flow output \
         (№965, landed); got: {}",
        if rendered.is_empty() {
            "<no errors>"
        } else {
            rendered.as_str()
        }
    );
    assert!(
        !rendered.contains("declare `-> output<"),
        "the private-edge refusal must NOT advise a terminator declaration (the \
         p08 red-team dead-end); got: {rendered}"
    );
}

/// THE ANNOTATION CANNOT WAIVE THE CONFIDENTIALITY HALF: the same
/// private-edge program WITH the terminator declaration still refuses —
/// the declared `public` over a private edge fires the mismatch
/// discipline first, and the private egress refuses either way (layer
/// (2) of the landed Flow arm).
///
/// The p08 red-team finding (docs/audits/red-team-2026-11-17/): the
/// refusal must NOT advise `-> output<private, ...>` — a declaration
/// layer (1) refuses by design, the advice would be an undecareable
/// dead end. The private-edge refusal carries the redact() downward
/// move instead (№326).
#[test]
fn n966_private_flow_output_annotated_still_refuses() {
    let src = "pattern P(x: String) -> String { return env(\"SP\") }\n\
               flow Main { input: String = \"x\" -> P -> output<public, untrusted> }";
    let rendered = rendered_errors(src, "the annotated control program");
    assert!(
        rendered.contains("PII_EGRESS_OUTPUT"),
        "a private flow output must refuse EVEN WITH the terminator declaration \
         (№967 — the confidentiality half is not declareable away); got: {}",
        if rendered.is_empty() {
            "<no errors — the annotation waived the confidentiality half>"
        } else {
            rendered.as_str()
        }
    );
    assert!(
        !rendered.contains("declare `-> output<"),
        "the private-edge refusal must NOT advise a terminator declaration — the \
         advice would be the undecareable dead end the red-team pass p08 caught; \
         got: {rendered}"
    );
    assert!(
        rendered.contains("redact()"),
        "the private-edge refusal must carry the sanctioned downward move (№326); \
         got: {rendered}"
    );
}

/// THE OVER-DECLARATION IS HONEST: a trusted edge (`literal` source —
/// public, trusted) with the dirtier-but-true declaration passes — the
/// claim COVERS the edge; `public, trusted` on a TRULY trusted edge is
/// equally fine (it stays reachable only by an edge that carries it —
/// the mismatch check enforces that direction, not this one).
#[test]
fn n966_the_over_declaration_passes() {
    let src = "pattern Lit(x: String) -> String { return \"hello\" }\n\
               flow Main { input: String = \"x\" -> Lit -> output<public, untrusted> }";
    let rendered = rendered_errors(src, "the over-declaration");
    assert!(
        rendered.is_empty(),
        "a declaration dirtier than the computed edge COVERS it (№967 — honest \
         conservatism passes); got: {rendered}"
    );
}

/// THE BOGUS WORD IS A PARSE-LEVEL ERROR: the terminator annotation is a
/// grammar position the label system sees — the №322 validation fires on
/// an unknown word before any clearance question.
#[test]
fn n966_the_bogus_word_refuses() {
    let src = "pattern Lit(x: String) -> String { return \"hello\" }\n\
               flow Main { input: String = \"x\" -> Lit -> output<bogus> }";
    let rendered = rendered_errors(src, "the bogus annotation");
    assert!(
        rendered.contains("label annotation"),
        "an unparseable terminator annotation must be the loud №322 label error; \
         got: {}",
        if rendered.is_empty() {
            "<no errors>"
        } else {
            rendered.as_str()
        }
    );
}

/// THE DOCS GATE (the №956 discipline): the boundary is LANDED — the B13
/// row moved to the FIXED state with the measured numbers kept, the ADR
/// carries the decision (§13.7), the design (§13.8) and the landing
/// record (§13.9); limitations.md no longer lists the B13 half as an
/// open gap.
#[test]
fn n966_the_integrity_package_is_measured_in_the_docs() {
    let tm = fs::read_to_string(repo_root().join("docs").join("threat-model.md"))
        .expect("threat-model.md readable");
    assert!(
        tm.contains("| B13 | FIXED by №967"),
        "threat-model.md B13 row did not move to the landed state — the fixing \
         naryad flips its row in the same PR (the №948 contract)"
    );
    for number in [
        "7 files carrying the confidentiality flow finding, 30 with both \
                   axes gated, 23 integrity-only",
        "16 on the integrity axis",
    ] {
        assert!(
            tm.contains(number),
            "the B13 row lost the measured numbers ({number}) — the corpus change \
             and the doc move ride the SAME PR (the №956 discipline)"
        );
    }
    assert!(
        !tm.contains("| B13 | OPEN BY DECISION"),
        "the B13 row still claims OPEN BY DECISION — the owner decision 16.11 \
         chose form (а) §13.4 and the gate landed (№967)"
    );

    let adr = fs::read_to_string(
        repo_root()
            .join("docs")
            .join("adr")
            .join("0154-label-lattice.md"),
    )
    .expect("ADR-0154 readable");
    assert!(
        adr.contains("### 13.7") && adr.contains("the §13.4 form (а)"),
        "ADR-0154 §13.7 (the owner decision 16.11) is gone — the decision record \
         must survive the landing"
    );
    assert!(
        adr.contains("### 13.8") && adr.contains("the design draft"),
        "ADR-0154 §13.8 (the design draft) is gone — the naryad implements it \
         verbatim, the record stays"
    );
    assert!(
        adr.contains("### 13.9"),
        "ADR-0154 §13.9 (the landing record) is missing — the executor records \
         the implementation in the same PR (the №966 §13 form)"
    );

    let lm = fs::read_to_string(repo_root().join("docs").join("limitations.md"))
        .expect("limitations.md readable");
    assert!(
        !lm.contains("the FLOW OUTPUT's INTEGRITY half (row B13"),
        "limitations.md still lists the B13 integrity half as an open gap — the \
         boundary became a rule (№967); the row leaves the not-caught list"
    );
}

/// The agree-test: the two documents tell one story. B7 says the
/// integrity half is its own row; B13 is LANDED with its flip target;
/// the .expected pin carries the landed status and the annotated form.
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
        expected.contains("row: B13") && expected.contains("status: LANDED"),
        "the b13 .expected pin must carry 'row: B13' and the LANDED status (the \
         №948 contract form, flipped by №967)"
    );
    assert!(
        expected.contains("-> output<public, untrusted>"),
        "the b13 .expected pin must carry the idiom form the migration uses \
         (№967 — form (а))"
    );
}
