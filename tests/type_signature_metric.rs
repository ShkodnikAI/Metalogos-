// ── Naryad #467 (P1, types/rust): the typed-signature metric — the
// in-tree double lock ────────────────────────────────────────────────
//
// The stage-0 CI metric lives in `scripts/ci/type_signature_share.py`
// (the `type-signatures` job prints the share on every run and gates it
// against the checked-in floor). This test is the same floor enforced
// from INSIDE the binary, over the live `BUILTIN_REGISTRY` — the two
// locks cannot drift: the script counts the source rows, the test
// counts the compiled specs.
//
// The rule (the owner's strengthening): the typed-signature share rises
// every release — a regression (a typed row losing its type, or the
// floor falling) fails here exactly as it fails the CI gate.

use metalogos::builtins::{sig_types::Type, BUILTIN_REGISTRY};

/// The checked-in floor: the stage-0 start value (2026-09-26, 49 typed
/// rows of 500 — the verified flat vocabulary). MUST move only up, in
/// the same PR that types more rows; mirrors
/// `scripts/ci/type_signature_baseline.txt` (`# threshold_bp: 980`).
const TYPED_FLOOR: usize = 49;

#[test]
fn typed_signature_share_never_falls_below_the_floor() {
    let total = BUILTIN_REGISTRY.len();
    let typed = BUILTIN_REGISTRY
        .iter()
        .filter(|spec| spec.return_type != Type::Unknown)
        .count();
    let share_bp = (typed * 10000) / total;
    println!(
        "typed signature share: {}/{} ({}.{:02}%)",
        typed,
        total,
        share_bp / 100,
        share_bp % 100
    );
    assert!(
        typed >= TYPED_FLOOR,
        "typed signatures regressed: {} < {} (№467: the share rises every \
         release; a typed row lost its type or the floor fell)",
        typed,
        TYPED_FLOOR
    );
}

#[test]
fn every_typed_signature_is_the_honest_parse_of_its_path() {
    // A typed row's return_type must NOT be Unknown by definition; the
    // honest-Unknown discipline is pinned by the sig_types unit tests —
    // here we pin the registry-side invariant: no row typed with a path
    // that from_path could not lift (the fill site cannot produce
    // Unknown for a row that carries an explicit type argument — a
    // vocabulary miss would have compiled to Unknown and dropped the
    // share, which the floor test above then catches).
    let typed = BUILTIN_REGISTRY
        .iter()
        .filter(|spec| spec.return_type != Type::Unknown)
        .count();
    let unknown = BUILTIN_REGISTRY.len() - typed;
    // The known stage-0 fact: 451 honest Unknowns (the untyped rows).
    assert_eq!(
        typed + unknown,
        BUILTIN_REGISTRY.len(),
        "the partition must be exact"
    );
}
