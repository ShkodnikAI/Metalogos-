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

/// The checked-in floor: raised by №613 (2026-10-06 — 47 rows PRECISE,
/// the 0.29 criterion 1b), by №616 (2026-10-06 — 61 rows PRECISE, the
/// 0.30 movement №1: the verified single-shape handler facts, the
/// verification table in the naryad gh#1077), and by №623 (2026-10-07 —
/// the 7 PARAMETERIZED rows List<T>/Struct<Name>: the first honest
/// package of the third metric, the handler-read verification table in
/// the naryad gh#1086). The by-design gap continues (see
/// part 1's comment): the CI script counts SOURCE rows (309 — the same 4
/// gated vec/store rows ride only there), this test counts the COMPILED
/// default-feature registry (305). MUST move only up, in the same PR
/// that types more rows — AND together with
/// `scripts/ci/type_signature_baseline.txt` (`# threshold_bp: 5988`).
/// The general share 5988 bp, the precise share 4224 bp — the 0.29 gate
/// goals stay exceeded (3500/3000, ADR-0181 §3/§3.1).
const TYPED_FLOOR: usize = 305;

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

// ── №623 (gh#1086): the third metric — the parameterized share ──────
//
// The in-tree twin of `scripts/ci/type_signature_parameterized_baseline.txt`
// (`# threshold_bp: 769`): the share of the PARAMETERIZED rows
// (`List<T>` / `Struct<Name>`) among the typed List/Struct rows. The
// same two-locks discipline: the script counts the SOURCE rows, this
// test counts the COMPILED specs — the locks cannot drift. Only-up, in
// the same PR that parameterizes more rows (the №757 procedure).
const PARAM_FLOOR: usize = 7;
/// The compiled denominator: 91 SOURCE List/Struct rows − the 4 gated
/// vec/store rows (the same by-design source/compiled gap the TYPED_FLOOR
/// comment documents — the gated rows are BARE List/Struct, so the
/// compiled share (7/87 = 804 bp) reads HIGHER than the source share
/// (7/91 = 769 bp); the baseline file locks the source number, this test
/// the compiled one.
const PARAM_LS_DENOM: usize = 87;

#[test]
fn parameterized_signature_share_never_falls_below_the_floor() {
    let param = BUILTIN_REGISTRY
        .iter()
        .filter(|spec| spec.parameterized)
        .count();
    let ls = BUILTIN_REGISTRY
        .iter()
        .filter(|spec| {
            let t = format!("{}", spec.return_type);
            t == "List" || t == "Struct" || spec.parameterized
        })
        .count();
    println!(
        "parameterized signatures: {}/{} ({}.{:02}%)",
        param,
        ls,
        (param * 10000) / ls.max(1) / 100,
        (param * 10000) / ls.max(1) % 100
    );
    assert!(
        param >= PARAM_FLOOR,
        "parameterized signatures regressed: {} < {} (№623: the share rises \
         every release; a parameterized row lost its spelling or the floor \
         fell)",
        param,
        PARAM_FLOOR
    );
    assert!(
        ls >= PARAM_LS_DENOM,
        "the List/Struct denominator shrank: {} < {} (№623: the base the \
         share is computed over must not shrink — a coarse row typed away \
         is a fact change, record it in the naryad)",
        ls,
        PARAM_LS_DENOM
    );
}
