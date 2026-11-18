// ── Naryad №970 (issue #1974 — the B14 line, the red-team finding F2,
// p38_exec_step_stdout_display; form (ii), the owner decision 18.11,
// P1, [security][compiler]): the exec-stdout channel on the flow
// edge — GATED through the №967 machinery. ──
//
// The lineage: the independent red-team audit 17.11 found (F2) that a
// flow whose pipeline step is the builtin `exec` carrier passes
// `mlog check` with no annotation and displays the command's stdout on
// the flow output — command output is the textbook untrusted channel,
// but `exec` is not among P1's enumerated sources, so no promise text
// was broken (the P1/P10-adjacent gap, INCONCLUSIVE F2). The owner
// chose the display-rule form (ii) — the exec step joins
// `Integrity::Untrusted` into the flow edge (the conf axis untouched)
// and the №967 gate answers:
//
//   1. the unannotated terminator refuses with the FROZEN class
//      (HTML_INJECTION for untrusted-public; §12.4 — no new check_id)
//      and names the idiom form (the §13.4 criterion: the refusal
//      stays readable to an author who did not declare it);
//   2. `-> output<public, untrusted>` is the honest display-intent
//      claim — a true claim passes;
//   3. a weaker claim (`<public, trusted>`) is the loud §12.2
//      mismatch — the annotation cannot become a silencer.
//
// The gate is STATIC: no capability grant and no env var are needed
// for the check refusal (the runtime gates stay fail-closed and
// untouched). Zero corpus migration: 0 exec-step carriers measured
// (the naryad's record) — the no-exec control below pins the zero.
//
// MUTATION CONTRACT (`scripts/mutation_verify_970.sh`):
//   M1 — the exec-stdout join is DROPPED -> n970_b14_unannotated_refuses,
//        n970_the_exec_step_flow_unannotated_refuses and
//        n948_b14_exec_stdout_display_gated MUST FAIL (the boundary
//        moved back to silence — loudly, exactly what the flipped pins
//        exist to catch);
//   M2 — the join is moved to the SOURCE edge (every flow gets it) ->
//        n970_flow_without_exec_stays_clean MUST FAIL (the zero-migration
//        control is the named mutation target: a join that fires
//        regardless of the exec step is the over-refusal bug).
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

/// THE GATED NEGATIVE (the flip of the red-team F2 current-behavior
/// finding): the B14 fixture — a flow whose pipeline step is the
/// builtin `exec` carrier, the command's stdout displayed UNANNOTATED —
/// refuses with the frozen print-channel class, and the refusal names
/// the idiom form.
#[test]
fn n970_b14_unannotated_refuses() {
    let src = fs::read_to_string(boundary_dir().join("b14_exec_stdout_display.mlog"))
        .expect("the b14 pin fixture is missing — restore it (№970)");
    let rendered = rendered_errors(&src, "the b14 pin fixture");
    assert!(
        rendered.contains("HTML_INJECTION"),
        "the UNANNOTATED exec-step flow output must refuse with the frozen \
         print-channel class (№970 — the exec stdout is the untrusted channel on \
         the display edge); got: {}",
        if rendered.is_empty() {
            "<no errors — the B14 gate is back to silence>"
        } else {
            rendered.as_str()
        }
    );
    assert!(
        rendered.contains("declare `-> output<public, untrusted>`"),
        "the refusal must name the idiom form (the №967 machinery reused — the \
         §13.4 criterion); got: {}",
        rendered
    );
}

/// THE ANNOTATED POSITIVE: the same shape WITH the true display-intent
/// claim — the display of command output is INTENDED and declared —
/// passes.
#[test]
fn n970_b14_annotated_passes() {
    let src = "flow Main { input: String = \"echo SECRET_MARKER_XYZ\" -> exec -> \
               output<public, untrusted> }";
    let rendered = rendered_errors(src, "the annotated exec display");
    assert!(
        rendered.is_empty(),
        "the flow with the TRUE terminator claim must pass (№970 — the display of \
         command output declared as the idiom); got: {rendered}"
    );
}

/// The same negative, self-contained inline — the canonical exec-step
/// program without the annotation.
#[test]
fn n970_the_exec_step_flow_unannotated_refuses() {
    let src = "flow Main { input: String = \"echo SECRET_MARKER_XYZ\" -> exec -> output }";
    let rendered = rendered_errors(src, "the unannotated exec display");
    assert!(
        rendered.contains("HTML_INJECTION"),
        "an unannotated exec-step flow output must refuse (№970 — the B14 gate); \
         got: {rendered}"
    );
}

/// THE §12.2 UNSATISFIED-CLAIM NEGATIVE: an annotation WEAKER than the
/// computed edge — `public, trusted` claimed over the exec-stdout edge —
/// is the silencer attempt, refused loudly with the annotation-discipline
/// class.
#[test]
fn n970_the_weaker_claim_refuses() {
    let src = "flow Main { input: String = \"echo SECRET_MARKER_XYZ\" -> exec -> \
               output<public, trusted> }";
    let rendered = rendered_errors(src, "the weaker claim over exec stdout");
    assert!(
        rendered.contains("LABEL_ANNOTATION_MISMATCH"),
        "a declared set weaker than the computed edge must refuse with the loud \
         §12.2 error (№970 — the annotation cannot become a silencer); got: {}",
        if rendered.is_empty() {
            "<no errors — the annotation became a silencer>"
        } else {
            rendered.as_str()
        }
    );
}

/// THE ZERO-MIGRATION CONTROL (the M2 mutation target): a flow WITHOUT
/// the exec step stays unannotated-clean — the join must ride the exec
/// STEP, not the flow shape. This is what makes the №970 landing a
/// zero-migration change (0 exec-step carriers measured).
#[test]
fn n970_flow_without_exec_stays_clean() {
    let src = "pattern P(x: String) -> String { return \"ok\" }\n\
               flow Main { input: String = \"doc\" -> P -> output }";
    let rendered = rendered_errors(src, "the no-exec control");
    assert!(
        rendered.is_empty(),
        "a flow without the exec step must stay unannotated-clean (№970 — zero \
         migration; the join rides the exec step only); got: {rendered}"
    );
}

/// THE STATIC-GATE CHECK: the refusal is the check-time display rule —
/// it needs NO capability profile and NO env var (this program carries
/// no `profile capability` at all and the check is a pure static walk;
/// the runtime gates stay fail-closed and untouched).
#[test]
fn n970_the_gate_is_static() {
    let src = "flow Main { input: String = \"echo X\" -> exec -> output }";
    let rendered = rendered_errors(src, "the static gate check");
    assert!(
        rendered.contains("HTML_INJECTION"),
        "the check refusal must fire statically (№970 — no grant, no env; the \
         runtime gates are a separate fail-closed lane); got: {}",
        if rendered.is_empty() {
            "<no errors — the gate became runtime-dependent>"
        } else {
            rendered.as_str()
        }
    );
}
