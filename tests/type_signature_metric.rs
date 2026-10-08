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
/// default-feature registry (312). MUST move only up, in the same PR
/// that types more rows — AND together with
/// `scripts/ci/type_signature_baseline.txt` (`# threshold_bp: 6124`).
/// The general share 6124 bp, the precise share 4302 bp — the gate
/// records stay exceeded (3500/3000, ADR-0181 §3/§3.1). №661 (2026-10-09):
/// canary_check typed → 313/516 compiled (317/516 source = 6143 bp).
const TYPED_FLOOR: usize = 313;

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

// ── №623 (gh#1086) + №631 (gh#1099): the third metric — the
// parameterized share ────────────────────────────────────────────────
//
// The in-tree twin of `scripts/ci/type_signature_parameterized_baseline.txt`
// (`# threshold_bp: 6043` since №637): the share of the PARAMETERIZED rows
// (`List<T>` / `Struct<Name>`) among the typed List/Struct rows. The
// same two-locks discipline: the script counts the SOURCE rows, this
// test counts the COMPILED specs — the locks cannot drift. Only-up, in
// the same PR that parameterizes more rows (the №757 procedure). The
// №631 second honest package: 27 verified rows of the registry-order
// contour (the handler-read table in gh#1099) — the floor 7 → 31. The
// №637 third honest package: the PDF contour — 20 verified Struct rows
// (with field_meta, the anti-dilution rule) + pdf_extract_images
// List<String> (the handler-read table in gh#1120) — 52/87 = 5977 bp
// compiled, the floor 31 → 52 — the 0.30 gate goal ≥ 5000 bp reached.
// №650 fourth honest package: the sensitive-surface imap rows —
// imap_list/imap_search List<ImapMessage> + imap_read Struct<ImapEmail>
// (with field_meta) — 55/90 compiled = 6111 bp;
// the floor 52 → 55.
const PARAM_FLOOR: usize = 56;
/// The compiled denominator: 91 SOURCE List/Struct rows − the 4 gated
/// vec/store rows (the same by-design source/compiled gap the TYPED_FLOOR
/// comment documents — the gated rows are BARE List/Struct, so the
/// compiled share reads HIGHER than the source share: 7/87 = 804 bp at
/// №623, 31/87 = 3563 bp at №631, 52/87 = 5977 bp at №637 — the baseline
/// file locks the source number, this test the compiled one).
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

// ── №627 (gh#1110): the FOURTH metric — the field-label share ───────
//
// The in-tree twin of `scripts/ci/type_signature_fieldmeta_baseline.txt`
// (`# threshold_bp: 9500` since №638): the share of the rows carrying the
// field-meta section (`Struct<Name>{field:label,...}`) among the
// parameterized Struct rows. The same two-locks discipline: the script
// counts the SOURCE rows (40 Struct-param rows — the llm_usage cfg(llm)
// row rides only there when the feature is on), this test counts the
// COMPILED default-feature specs. Only-up, in the same PR that labels
// more fields (the №757 procedure). The first honest package:
// GeoLocation/Weather/LlmUsage — 28 verified field-label rows (the
// handler-read table in the naryad gh#1110); the floor 0 → 3 compiled
// rows. The №637 package: the 20 PDF Struct rows arrive WITH their
// field_meta sections in the same PR (the anti-dilution rule — no
// parameterized Struct row lands bare), the floor 3 → 23. The №638
// second honest package: the 15 EXISTING parameterized Struct rows of
// the memory/goal/todo/vec/human contours (52 field-label entries; the
// classification per the №627 table read off the handlers — the table
// in gh#1121), the floor 23 → 37 (38/40 = 9500 bp source; the cfg(vec)
// vec_store row rides only the source metric — 37/39 compiled).
//
// №650 (wave 38): imap_read Struct<ImapEmail> ships WITH the 9-entry
// field_meta (the №627 classification: every field the IMAP server
// payload or the caller uid echo → untrusted) — 38/40 compiled.
//
// №661 (wave 39; the К-Б stage-2 verdict gh#1144): canary_check ships
// WITH the 3-entry field_meta — the FIRST PRIVATE label (id:private — the
// canary-credential reference; leaked/position — the check outcome,
// internal) — 39/41 compiled (40/42 source = 9523 bp). The BLOCKING
// enforcement lives in the type-signatures job
// (--enforce-fieldmeta) + the in-tree twin below.
//
// BuiltinSpec.field_meta is the registry SIDE-TABLE (the enum is NOT
// extended — stage-2 minimality); parse_field_meta is the consumer API.
const FIELDMETA_FLOOR: usize = 39;

#[test]
fn field_label_share_never_falls_below_the_floor() {
    let labeled = BUILTIN_REGISTRY
        .iter()
        .filter(|spec| !spec.field_meta.is_empty())
        .count();
    let structs = BUILTIN_REGISTRY
        .iter()
        .filter(|spec| spec.parameterized && spec.return_type == Type::Struct)
        .count();
    println!(
        "field-label signatures: {}/{} ({}.{:02}%)",
        labeled,
        structs,
        (labeled * 10000) / structs.max(1) / 100,
        (labeled * 10000) / structs.max(1) % 100
    );
    assert!(
        labeled >= FIELDMETA_FLOOR,
        "field-label signatures regressed: {} < {} (№627: the share rises \
         every release; a field-meta row lost its section or the floor \
         fell)",
        labeled,
        FIELDMETA_FLOOR
    );
    // The denominator lock: the parameterized-Struct base must not shrink
    // (the same №623 discipline — a coarse row typed away is a fact
    // change; record it in the naryad, then move this const). The №637
    // fact: 39 compiled Struct-param rows (40 source − llm_usage cfg(llm));
    // №661: + canary_check → 40 compiled (42 source).
    assert!(
        structs >= 40,
        "the parameterized-Struct denominator shrank: {} < 40 (№627: the \
         base the field-label share is computed over must not shrink)",
        structs
    );
}

#[test]
fn n661_every_parameterized_struct_row_carries_field_meta() {
    // The IN-TREE twin of the blocking `--enforce-fieldmeta` step (the
    // type-signatures job, №661): every parameterized Struct row of the
    // registry carries a well-formed field_meta, EXCEPT the named
    // dynamic-form carve-outs. The carve-out list is DUPLICATED on both
    // sides DELIBERATELY (the Rust test + the Python
    // FIELD_META_ENFORCE_EXCEPTIONS) — growing either side is an explicit,
    // reviewed PR decision (the anti-Goodhart posture: never a silent
    // default). A NEW parameterized Struct row that lands bare reddens
    // THIS test and the CI step at once — the red-proof the naryad pins.
    const DYNAMIC_CARVE_OUTS: &[&str] = &["form_data", "json_body"];
    let mut violations: Vec<String> = Vec::new();
    for spec in BUILTIN_REGISTRY.iter() {
        if !(spec.parameterized && spec.return_type == Type::Struct) {
            continue;
        }
        // The non-empty sections are already grammar-proven by
        // every_field_meta_is_the_well_formed_grammar — the enforcement
        // asks only: is the section THERE (or the row named as dynamic).
        if spec.field_meta.is_empty() && !DYNAMIC_CARVE_OUTS.contains(&spec.name) {
            violations.push(spec.name.to_string());
        }
    }
    assert!(
        violations.is_empty(),
        "№661: parameterized Struct rows without field_meta (and not in \
         the named carve-outs): {:?} — label the fields honestly (the №650 \
         mirror rule: the signature mirrors the real handler form) or name \
         the dynamic form explicitly on BOTH sides (this test's \
         DYNAMIC_CARVE_OUTS + the Python FIELD_META_ENFORCE_EXCEPTIONS)",
        violations
    );
}

#[test]
fn every_field_meta_is_the_well_formed_grammar() {
    // The fail-closed armor (the in-tree twin of from_path's Unknown
    // erasure): a non-empty field_meta MUST parse under the №627 grammar
    // (parse_field_meta is fail-closed — None on malformed), the row must
    // be Struct-typed, and every (field, label) entry must carry a
    // non-empty name and a stage-0 Label. A malformed section attached at
    // the fill site fails HERE and drops the row to Unknown (the general
    // typed floor catches it independently).
    for spec in BUILTIN_REGISTRY.iter() {
        if spec.field_meta.is_empty() {
            continue;
        }
        let parsed = metalogos::builtins::sig_types::parse_field_meta(spec.field_meta)
            .unwrap_or_else(|| {
                panic!(
                    "the malformed field_meta on '{}' — a typo must not ride the registry side-table (№627 fail-closed)",
                    spec.name
                )
            });
        assert!(
            !parsed.is_empty(),
            "the empty field-meta section on '{}' (№627: {} = absent)",
            spec.name,
            "\"\""
        );
        assert_eq!(
            spec.return_type,
            Type::Struct,
            "the field-meta row '{}' must erase to Struct (the section \
             belongs to structs — a List head with a brace tail is honest \
             Unknown)",
            spec.name
        );
        for (field, _label) in &parsed {
            assert!(
                !field.is_empty(),
                "the empty field name in the '{}' table",
                spec.name
            );
        }
    }
}
