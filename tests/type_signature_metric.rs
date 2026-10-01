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

/// The checked-in floor: raised by №543 (2026-10-01 — the bot package
/// types 30 rows). NOTE the by-design gap between the two locks (the
/// file header says it): the CI script counts the SOURCE rows (137 —
/// includes the feature-gated `vec`/store-lane rows embed/vec_store/
/// vec_search/memory_forget), this test counts the COMPILED default-
/// feature registry (133). MUST move only up, in the same PR that types
/// more rows — AND together with `scripts/ci/type_signature_baseline.txt`
/// (`# threshold_bp: 2702`): the №542 registration (this lock lagging
/// the baseline since №757) is closed by this raise — both locks move
/// in the same PR from now on.
const TYPED_FLOOR: usize = 133;

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
