// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. The corpus test below reads examples/ for
// fixtures by design — the allow is scoped to this file (the same posture
// as tests/naryad_474_stage1_types.rs).
#![allow(clippy::disallowed_methods)]

// ── Naryad №486 (issue #734): the declared-type conflict surface —
// stage 1 warn-only extension. ───────────────────────────────────────
//
// What this test pins:
//   1. R1 — a pattern declaring `-> Int` that returns a typed String
//      builtin warns, WITH POSITION (the return statement's real span
//      — the parser now carries it for Assign/Return);
//   2. R2 — a declared parameter type seeds the inference env: the
//      EXISTING №474 assign rule flags `param = <Float builtin>`, the
//      origin naming the DECLARATION, and the follow-up return
//      conflicts under R1;
//   3. R3 — a pattern call types its `let` from the DECLARED return
//      type (plain patterns AND learnable patterns), so a later
//      builtin reassignment of a different known type conflicts;
//   4. warn-only: the SAME programs produce ZERO errors and run;
//   5. honesty: an unparseable declared spelling (`-> Message`) never
//      warns; a labeled declaration compares by BASE (`String<private>`
//      vs a String builtin is silent); a same-type return is silent;
//   6. the corpus discipline: every examples/*.mlog checks with no
//      panic; every №486 warning carries the `[n486 types]` prefix and
//      lands in WARNINGS only; the total count is PRINTED as the
//      informational CI metric (NOT a ratchet — the issue says so).

use metalogos::semantic_types::N486_PREFIX;
use metalogos::semantic_types::STAGE1_PREFIX;

const RETURN_CONFLICT: &str = "
pattern P(x: String) -> Int {
    return upper(x)
}
";

fn n486_of(source: &str) -> Vec<metalogos::semantic::SpannedError> {
    let result = metalogos::check_program(source).expect("check must run");
    result
        .warnings
        .iter()
        .filter(|w| w.message.contains(N486_PREFIX))
        .cloned()
        .collect()
}

fn stage1_of(source: &str) -> Vec<metalogos::semantic::SpannedError> {
    let result = metalogos::check_program(source).expect("check must run");
    result
        .warnings
        .iter()
        .filter(|w| w.message.contains(STAGE1_PREFIX))
        .cloned()
        .collect()
}

#[test]
fn r1_return_conflict_warns_with_position() {
    // Line 1 is empty; the pattern is line 2; the `return` is line 3.
    let warns = n486_of(RETURN_CONFLICT);
    assert_eq!(
        warns.len(),
        1,
        "exactly one declared-return conflict: {:?}",
        warns
    );
    let msg = &warns[0].message;
    assert!(msg.contains("'P'"), "the pattern is named: {}", msg);
    assert!(msg.contains("pattern"), "the kind is named: {}", msg);
    assert!(msg.contains("Int"), "the declared type is named: {}", msg);
    assert!(
        msg.contains("String"),
        "the produced type is named: {}",
        msg
    );
    assert!(
        msg.contains("'upper'"),
        "the builtin origin is named: {}",
        msg
    );
    assert!(
        msg.contains("warn-only"),
        "the honesty note is in the text: {}",
        msg
    );
    // THE POSITION: the warning points at the `return` statement's line.
    assert_eq!(
        warns[0].span.start_line, 3,
        "the warning carries the return statement's real position (span {:?})",
        warns[0].span
    );
}

#[test]
fn r2_declared_param_conflicts_through_the_assign_rule() {
    let src = "
pattern Q(a: String) -> String {
    a = len(\"hello\")
    return a
}
";
    // The №474 assign rule: 'a' holds the DECLARED String, is reassigned
    // with the Float builtin 'len' — the origin names the declaration.
    let stage1 = stage1_of(src);
    assert_eq!(stage1.len(), 1, "exactly one assign conflict: {:?}", stage1);
    let msg = &stage1[0].message;
    assert!(msg.contains("'a'"), "the variable is named: {}", msg);
    assert!(
        msg.contains("the declared parameter type 'String'"),
        "the origin names the DECLARATION: {}",
        msg
    );
    assert!(msg.contains("Float"), "the new type is named: {}", msg);
    // R1 follows: after the reassignment 'a' is known Float, the pattern
    // declares -> String — the `return a` conflicts with position.
    let warns = n486_of(src);
    assert_eq!(
        warns.len(),
        1,
        "exactly one declared-return conflict: {:?}",
        warns
    );
    assert!(
        warns[0].message.contains("'Q'"),
        "the pattern is named: {}",
        warns[0].message
    );
    // Both diagnostics point at their statements (lines 3 and 4).
    assert_eq!(
        stage1[0].span.start_line, 3,
        "the assign conflict is positioned"
    );
    assert_eq!(
        warns[0].span.start_line, 4,
        "the return conflict is positioned"
    );
}

#[test]
fn r3_pattern_call_types_from_the_declaration() {
    let src = "
pattern H() -> Float {
    return 1.0
}
pattern M() -> String {
    let v = H()
    v = upper(\"z\")
    return v
}
";
    // `let v = H()` types v as the DECLARED Float of H; the builtin
    // reassignment (upper → String) conflicts via the №474 rule, the
    // origin naming the declaration. H itself is consistent (Float vs
    // -> Float, silent); M's `return v` is String vs -> String, silent.
    let stage1 = stage1_of(src);
    assert_eq!(stage1.len(), 1, "exactly one conflict: {:?}", stage1);
    let msg = &stage1[0].message;
    assert!(msg.contains("'v'"), "the variable is named: {}", msg);
    assert!(
        msg.contains("the declared return type of the pattern 'H'"),
        "the origin names the DECLARED return: {}",
        msg
    );
    assert!(msg.contains("Float"), "the declared type is named: {}", msg);
    assert!(msg.contains("String"), "the builtin type is named: {}", msg);
    let warns = n486_of(src);
    assert!(
        warns.is_empty(),
        "no declared-return conflict here: {:?}",
        warns
    );
}

#[test]
fn r3_learnable_call_types_from_the_declaration() {
    let src = "
learnable pattern Score() -> Float {
  prompt: \"rate the text from 0 to 1\"
}
pattern M() -> String {
    let v = Score()
    v = upper(\"a\")
    return v
}
";
    let stage1 = stage1_of(src);
    assert_eq!(stage1.len(), 1, "exactly one conflict: {:?}", stage1);
    assert!(
        stage1[0]
            .message
            .contains("the declared return type of the learnable pattern 'Score'"),
        "the origin names the learnable declaration: {}",
        stage1[0].message
    );
}

#[test]
fn warn_only_zero_errors_and_the_program_runs() {
    let result = metalogos::check_program(RETURN_CONFLICT).expect("check must run");
    assert!(
        result.errors.is_empty(),
        "WARN-ONLY: zero errors, got {:?}",
        result.errors
    );
    let out = metalogos::run_program(RETURN_CONFLICT).expect("the program must run");
    let _ = out; // rc=0 with no compile-time rejection is the point.
}

#[test]
fn honesty_unparseable_declared_type_never_warns() {
    // `Message` is an entity name — outside the stage-0 vocabulary.
    // The declared side is Unknown → the check stays silent.
    let src = "
pattern U() -> Message {
    return upper(\"a\")
}
";
    let result = metalogos::check_program(src).expect("check must run");
    assert!(result.errors.is_empty(), "no errors: {:?}", result.errors);
    let warns = n486_of(src);
    assert!(
        warns.is_empty(),
        "an unparseable declared type NEVER warns: {:?}",
        warns
    );
}

#[test]
fn honesty_labeled_declaration_compares_by_base() {
    // `x: String<private>` compares as String — labels are annotations,
    // not runtime types. A String builtin reassigned into a labeled
    // parameter is NOT a conflict; a genuinely different type still is
    // (and the origin names the ERASED base — the honest runtime fact).
    let src = "
pattern Lb(x: String<private>) -> String {
    x = upper(\"a\")
    return x
}
";
    assert!(
        n486_of(src).is_empty(),
        "the label is erased before comparing: {:?}",
        n486_of(src)
    );
    assert!(
        stage1_of(src).is_empty(),
        "String builtin into a labeled String param is silent: {:?}",
        stage1_of(src)
    );
    // The flip side: a Float builtin into the labeled String param DOES
    // conflict, and the origin carries the base type (the label is not
    // part of the runtime fact).
    let src2 = "
pattern Lb2(x: String<private>) -> String {
    x = len(\"hi\")
    return x
}
";
    let stage1 = stage1_of(src2);
    assert_eq!(
        stage1.len(),
        1,
        "the base-type conflict still fires: {:?}",
        stage1
    );
    assert!(
        stage1[0]
            .message
            .contains("the declared parameter type 'String'"),
        "the origin names the ERASED base: {}",
        stage1[0].message
    );
}

#[test]
fn honesty_same_type_return_is_silent() {
    let src = "
pattern S(x: String) -> String {
    return upper(x)
}
";
    assert!(n486_of(src).is_empty(), "String into -> String never warns");
    assert!(
        stage1_of(src).is_empty(),
        "no assign rule involvement either"
    );
}

#[test]
fn tool_method_declared_return_conflict_is_positioned() {
    let src = "
tool math_api {
    get_mean() -> Int {
        return upper(\"a\")
    }
}
";
    let warns = n486_of(src);
    assert_eq!(warns.len(), 1, "exactly one conflict: {:?}", warns);
    let msg = &warns[0].message;
    assert!(
        msg.contains("tool method") && msg.contains("'math_api.get_mean'"),
        "the qualified owner is named: {}",
        msg
    );
    assert_eq!(warns[0].span.start_line, 4, "the return line is named");
}

#[test]
fn corpus_declared_type_warn_count() {
    let dir = std::path::Path::new("examples");
    let mut checked = 0u32;
    let mut n486_total = 0u32;
    let mut n486_in_errors = 0u32;
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("examples/ exists")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "mlog").unwrap_or(false))
        .collect();
    entries.sort();
    for path in entries {
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let result = match metalogos::check_program(&source) {
            Ok(r) => r,
            Err(_) => continue, // examples that intentionally fail the PARSER
        };
        checked += 1;
        for w in &result.warnings {
            if w.message.contains(N486_PREFIX) {
                n486_total += 1;
                assert!(
                    w.message.starts_with(N486_PREFIX),
                    "every №486 warning starts with the prefix: {}",
                    w.message
                );
            }
        }
        for e in &result.errors {
            if e.message.contains(N486_PREFIX) {
                n486_in_errors += 1;
            }
        }
    }
    assert!(checked > 200, "the corpus is large, checked {}", checked);
    assert_eq!(
        n486_in_errors, 0,
        "the №486 output must land in WARNINGS only — never in errors"
    );
    // The INFORMATIONAL metric (NOT a ratchet — the issue says so): the
    // count of real declared-type conflicts the corpus carries today.
    // The CI artifact step prints this line into the job log.
    println!(
        "corpus: {} programs checked, {} declared-type (n486) warnings total (warn-only, informational)",
        checked, n486_total
    );
}
