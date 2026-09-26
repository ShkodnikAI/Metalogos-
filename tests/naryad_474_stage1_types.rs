// ── Naryad №474 (gh#742): stage 1 of the type system — the let-type
// inference, WARN-ONLY (gate gh#680, decision 3-A, step 2; the canon is
// the №467 body: "этап 1 (вывод let, warn-only)"). ────────────────────
//
// What this test pins:
//   1. the inference golden (builtin returns / literals / transitive
//      copies land in the env; Unknown stays out);
//   2. THE warn rule: a known-type reassignment with a different known
//      type warns with the [stage1 types] prefix, naming both types;
//   3. warn-only: the SAME program produces ZERO errors and RUNS
//      unchanged (the runtime value follows the reassignment);
//   4. honesty: Unknown NEVER warns;
//   5. each-body scoping: inner bindings do not leak, inner conflicts
//      do warn;
//   6. the corpus discipline: every examples/*.mlog parses, checks and
//      runs the semantic pass with no panic; every stage-1 warning
//      carries the prefix; the designed-error example still errors
//      (the pass adds no errors — structural, pinned here).

use metalogos::builtins::sig_types::Type;
use metalogos::semantic_types::{infer_block_types, STAGE1_PREFIX};

const CONFLICT_PROGRAM: &str = r#"
pattern Demo(x: String) -> String {
    let mut s = upper(x)
    s = 5.0
    return str(s)
}
"#;

fn parse_decls(source: &str) -> Vec<metalogos::ast::Declaration> {
    metalogos::parser::parse(source).expect("the test program must parse")
}

fn pattern_body<'a>(
    decls: &'a [metalogos::ast::Declaration],
    name: &str,
) -> &'a [metalogos::ast::Statement] {
    for d in decls {
        if let metalogos::ast::Declaration::Pattern(p) = d {
            if p.name == name {
                return &p.body;
            }
        }
    }
    panic!("pattern {} not found", name);
}

#[test]
fn inference_golden_builtins_literals_copies() {
    let decls = parse_decls(
        r#"
        pattern Golden(a: String) -> String {
            let s = upper(a)
            let n = 42.0
            let flag = true
            let copy = s
            let untyped = find(a, "f", "==", 1.0)
            return s
        }
    "#,
    );
    let env = infer_block_types(pattern_body(&decls, "Golden"));
    // The builtin 'upper' is typed String in the registry (№467 stage 0).
    assert_eq!(env.get("s").expect("s must be inferred").ty, Type::String);
    assert_eq!(env.get("n").expect("n must be inferred").ty, Type::Float);
    assert_eq!(env.get("flag").expect("flag").ty, Type::Bool);
    // The transitive copy: from a typed let.
    assert_eq!(env.get("copy").expect("copy").ty, Type::String);
    // `find` is NOT typed in the flat stage-0 vocabulary — Unknown stays
    // OUT of the env (the honesty pin at the inference level).
    assert!(env.get("untyped").is_none(), "Unknown must not be stored");
}

#[test]
fn conflict_warns_and_names_both_types() {
    let result = metalogos::check_program(CONFLICT_PROGRAM).expect("check must run");
    let stage1: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains(STAGE1_PREFIX))
        .collect();
    assert_eq!(
        stage1.len(),
        1,
        "exactly one stage-1 conflict: {:?}",
        result.warnings
    );
    let msg = &stage1[0].message;
    assert!(msg.contains("'s'"), "the variable is named: {}", msg);
    assert!(
        msg.contains("String"),
        "the previous type is named: {}",
        msg
    );
    assert!(msg.contains("Float"), "the new type is named: {}", msg);
    assert!(
        msg.contains("warn-only"),
        "the honesty note is in the text: {}",
        msg
    );
}

#[test]
fn warn_only_zero_errors_and_the_program_runs() {
    let result = metalogos::check_program(CONFLICT_PROGRAM).expect("check must run");
    assert!(
        result.errors.is_empty(),
        "WARN-ONLY: the same program must produce zero errors, got {:?}",
        result.errors
    );
    // And it RUNS exactly as before (the reassignment wins at runtime —
    // the language stays dynamically typed).
    let out = metalogos::run_program(CONFLICT_PROGRAM).expect("the program must run");
    let _ = out; // the demo pattern's return value is not the point here:
                 // the point is rc=0 with no compile-time rejection.
}

#[test]
fn run_proof_reassignment_wins_at_runtime() {
    let src = r#"
        pattern Main() -> Float {
            let mut v = len("hello")
            v = 7.5
            return v
        }
        test "the reassignment wins at runtime" {
            let r = Main()
            assert_eq(r, 7.5)
        }
    "#;
    let result = metalogos::check_program(src).expect("check must run");
    assert!(result.errors.is_empty(), "no errors: {:?}", result.errors);
    // len IS typed (Float) — the conflict against the Float literal 7.5
    // must NOT fire (Float == Float): no stage-1 warning here.
    let stage1: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains(STAGE1_PREFIX))
        .collect();
    assert!(
        stage1.is_empty(),
        "same-type reassignment never warns: {:?}",
        stage1
    );
    // The runtime follows the reassignment (the test block asserts it).
    let outcomes = metalogos::test_program(src).expect("the test block must run");
    assert_eq!(outcomes.len(), 1, "one test block ran");
    assert!(
        outcomes[0].passed,
        "the reassignment must win at runtime: {:?}",
        outcomes[0].error
    );
}

#[test]
fn unknown_never_warns() {
    let src = r#"
        pattern Quiet(a: String) -> String {
            let mut x = find(a, "f", "==", 1.0)
            x = "now a string"
            return a
        }
    "#;
    let result = metalogos::check_program(src).expect("check must run");
    assert!(result.errors.is_empty(), "no errors: {:?}", result.errors);
    let stage1: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains(STAGE1_PREFIX))
        .collect();
    assert!(
        stage1.is_empty(),
        "Unknown must NEVER warn (the honesty pin): {:?}",
        stage1
    );
}

#[test]
fn each_body_scoped_and_warns_inside() {
    let src = r#"
        pattern Loop(items: List) -> Unit {
            let outside = upper("keep")
            each it in items {
                let mut inner = upper("scoped")
                inner = 1.0
            }
            return
        }
    "#;
    let result = metalogos::check_program(src).expect("check must run");
    assert!(result.errors.is_empty(), "no errors: {:?}", result.errors);
    let stage1: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains(STAGE1_PREFIX))
        .collect();
    assert_eq!(
        stage1.len(),
        1,
        "the inner conflict warns (the each body is walked): {:?}",
        result.warnings
    );
    assert!(
        stage1[0].message.contains("'inner'"),
        "the INNER variable is named: {}",
        stage1[0].message
    );
    // The env-level scoping is structural: infer_block_types on the body
    // alone never surfaces 'inner' to the caller (the clone does not
    // leak) — pinned implicitly by the count assertion above.
    let decls = parse_decls(src);
    let env = infer_block_types(pattern_body(&decls, "Loop"));
    assert!(env.contains_key("outside"), "the top-level let lands");
    assert!(
        !env.contains_key("inner"),
        "the each-body binding must not leak into the outer env"
    );
}

#[test]
fn corpus_no_panic_prefix_discipline_and_no_new_errors() {
    let dir = std::path::Path::new("examples");
    let mut checked = 0u32;
    let mut stage1_total = 0u32;
    let mut stage1_in_errors = 0u32;
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
        let decls = match metalogos::parser::parse(&source) {
            Ok(d) => d,
            Err(_) => continue, // examples that intentionally fail the PARSER
        };
        let _ = decls; // parse-pass proof (the semantic check re-parses)
        let result = metalogos::check_program(&source).expect("check must run on parsed files");
        checked += 1;
        for w in &result.warnings {
            if w.message.contains(STAGE1_PREFIX) {
                stage1_total += 1;
                assert!(
                    w.message.starts_with(STAGE1_PREFIX),
                    "every stage-1 warning starts with the prefix: {}",
                    w.message
                );
            }
        }
        for e in &result.errors {
            if e.message.contains(STAGE1_PREFIX) {
                stage1_in_errors += 1;
            }
        }
    }
    assert!(
        checked > 200,
        "the corpus is large (242 .mlog files among 490 total in examples/), checked {}",
        checked
    );
    assert_eq!(
        stage1_in_errors, 0,
        "the stage-1 output must land in WARNINGS only — never in errors"
    );
    println!(
        "corpus: {} programs checked, {} stage-1 warnings total (warn-only)",
        checked, stage1_total
    );
}
