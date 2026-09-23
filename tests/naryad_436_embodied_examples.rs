// ── Naryad №436 (issue #630, Wave 11) — the embodied red/green example
//    contract ──────────────────────────────────────────────────────────
//
// The wave-11 example line convention (real binary run against the
// .expected golden): examples/w11_embodied_bounds.mlog proves BOTH
// embodied monitors of №354/№355 are load-bearing at the language
// surface:
//
//   RED 1  chunk_make on an unbounded device refuses typed
//          EMBODIED_UNBOUNDED (ADR-0159 §2.4.2: no unmonitored action).
//   RED 2  WorldState refuses materialization on every surface
//          (print / to_string / json_encode) typed WORLD_STATE_PRIVATE
//          (private-by-default, the Phase-1 lattice / №349).
//   GREEN  with a bounds formula attached the pipeline goes through and
//          the sealed proof verifies with the honest PENDING verdict
//          (the monitor lands with №356 — no program-forged satisfied).
//
// The mutation harness scripts/mutation_verify_436.sh runs THIS file
// against two live mutants (M1 the bounds refusal, M2 the print-surface
// guard) and requires the matching test to FAIL under each mutation —
// the tests below are that matching pair (one anchor test per mutant).

use std::fs;
use std::path::Path;

/// Execute the wave-11 embodied example in-process (the golden.rs seam:
/// `metalogos::run_program`) and return its stdout.
fn run_example() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let path = Path::new(&manifest_dir).join("examples/w11_embodied_bounds.mlog");
    let source = fs::read_to_string(&path).expect("examples/w11_embodied_bounds.mlog exists");
    metalogos::run_program(&source)
        .expect("w11_embodied_bounds must execute cleanly (no uncaught refusals)")
        .unwrap_or_default()
}

/// M1 anchor: the unbounded chunk_make refusal is typed and reachable.
/// Under mutation M1 (the bounds check neutered to pass-through) this
/// test MUST go red — that is the load-bearing proof the harness checks.
#[test]
fn n436_red_chunk_is_typed_unbounded() {
    let out = run_example();
    let field = out
        .split('|')
        .find(|f| f.starts_with("red_chunk="))
        .expect("the report carries the red_chunk field");
    assert_eq!(
        field, "red_chunk=EMBODIED_UNBOUNDED",
        "chunk_make on an unbounded device must refuse typed EMBODIED_UNBOUNDED; \
         got {field:?} — the bounds refusal is NOT load-bearing"
    );
}

/// M2 anchor: the print surface refuses the private WorldState with the
/// typed WORLD_STATE_PRIVATE stamp (the try-branchable №413 convention).
/// Under mutation M2 (the print-surface guard neutered to pass-through)
/// this test MUST go red.
#[test]
fn n436_print_surface_refuses_private() {
    let out = run_example();
    let field = out
        .split('|')
        .find(|f| f.starts_with("print="))
        .expect("the report carries the print field");
    assert_eq!(
        field, "print=WORLD_STATE_PRIVATE",
        "print(WorldState) must refuse typed WORLD_STATE_PRIVATE; got {field:?} — \
         the print-surface materialization guard is NOT load-bearing"
    );
}

/// The remaining materialization surfaces (to_string, json_encode) carry
/// the same typed refusal — the private-by-default contract is uniform
/// across the projection surfaces, not a print-only guard.
#[test]
fn n436_all_materialization_surfaces_refuse() {
    let out = run_example();
    for field in [
        "to_string=WORLD_STATE_PRIVATE",
        "json_encode=WORLD_STATE_PRIVATE",
    ] {
        assert!(
            out.split('|').any(|f| f == field),
            "expected the report to carry {field:?} — the surface guard is missing"
        );
    }
}

/// The green leg: with a bounds formula attached the same pipeline goes
/// through — chunk, sealed proof, and the honest PENDING verdict (the
/// stage-A monitor is №356; no program-forged satisfied).
#[test]
fn n436_green_leg_proof_pending() {
    let out = run_example();
    for field in [
        "chunk=ActionChunk",
        "proof=Proof",
        "valid=true",
        "verdict=pending",
    ] {
        assert!(
            out.split('|').any(|f| f == field),
            "expected the green leg to carry {field:?} — the bounded pipeline failed"
        );
    }
    assert_eq!(
        out.trim_end(),
        fs::read_to_string(
            Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()))
                .join("examples/w11_embodied_bounds.expected")
        )
        .expect("examples/w11_embodied_bounds.expected exists")
        .trim_end(),
        "the example output must stay in lockstep with its .expected golden"
    );
}
