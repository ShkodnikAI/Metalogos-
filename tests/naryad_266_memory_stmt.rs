// ── Наряд №266 (issue #290): memorize/relate/forget inside pattern/route
//    bodies — supported, token soup excluded FOREVER ──────────────────────
//
// Fact (verified by the controller on main @ 3c4ab45 / 29f0592): inside a
// pattern body the statement grammar had NO memory arms, so
//   memorize fact with priority=0.8
// silently split into 4 garbage statements:
//   Stmt 0: ExprStmt { expr: Ident("memorize") }
//   Stmt 1: ExprStmt { expr: Ident("fact") }
//   Stmt 2: ExprStmt { expr: Ident("with") }
//   Stmt 3: Assign { name: "priority", value: FloatLit(0.8) }
// TW then failed at the first pattern invocation with
// "undefined variable: memorize" (exit 1); the VM skipped the equivalent
// opcodes SILENTLY in execute_code (`_ => ip += 1`). Contract 3 of
// examples/p8_route_patterns.mlog ("pattern with memory from route") never
// worked end-to-end — it only ever compiled.
//
// Chosen variant (fixed loudly in PR §8.4): VARIANT 1 — SUPPORT, not a loud
// ban. Rationale: TW already executed top-level memorize/forget/relate as
// runtime actions (execution.rs run()), the VM already had the
// Memorize/Forget/Relate opcodes (execute_main_code), and no extra session
// context is required — so the statement form was pure plumbing, and a ban
// would have permanently hidden working semantics behind a parse error.
//
// Done-when (наряд): the probe-fact is INVERTED — a pattern with
// `memorize ... with priority=` inside the body parses as ONE memory
// statement, TW executes it, VM executes it honestly; the third outcome
// ("silent token soup") is excluded by the AST assertions below; top-level
// memorize (p7/m4/parser test_parse_memorize) stays green.

use metalogos::ast::{Declaration, Expr, Statement};

/// The exact probe from the наряд fact (p8 Contract 3 shape).
const PROBE: &str = r#"
pattern Remember(fact: String) -> String {
  memorize fact with priority=0.8
  return "ok"
}
pattern RecallIt(x: String) -> String {
  return recall("hello")
}
flow Main { input: String = "hello" -> Remember -> RecallIt -> output }
"#;

/// Part 1 of the fact, inverted at the AST level: the pattern body parses
/// `memorize fact with priority=0.8` as ONE Statement::Memorize — not as the
/// historical 4-statement token soup. This is the test that pins the third
/// outcome ("silent token soup") out FOREVER.
#[test]
fn n266_token_soup_excluded_memorize_stmt_ast() {
    let decls = metalogos::parser::parse(PROBE).expect("probe must parse");
    let pattern = decls
        .iter()
        .find_map(|d| match d {
            Declaration::Pattern(p) if p.name == "Remember" => Some(p),
            _ => None,
        })
        .expect("Remember pattern must be present");

    assert_eq!(
        pattern.body.len(),
        2,
        "body must be exactly [memorize, return] — 4 statements would be the token soup"
    );
    match &pattern.body[0] {
        Statement::Memorize(m) => {
            assert!(
                matches!(&m.value, Expr::Ident { name, .. } if name == "fact"),
                "memorize value must be the param expression `fact`, got {:?}",
                m.value
            );
            assert!(
                (m.priority - 0.8).abs() < 1e-9,
                "priority must be 0.8, got {}",
                m.priority
            );
        }
        other => panic!(
            "body[0] must be Statement::Memorize, got {:?} (token soup is back)",
            other
        ),
    }
    assert!(
        matches!(&pattern.body[1], Statement::Return { .. }),
        "body[1] must be the return statement, got {:?}",
        pattern.body[1]
    );
}

/// relate/forget share the same grammar class — both must parse as single
/// memory statements inside a body too (наряд: «relate/forget внутри тел —
/// тот же класс по грамматике»).
#[test]
fn n266_relate_and_forget_stmt_ast() {
    let src = r#"
pattern Link(topic: String) -> String {
  relate "metalogos" to topic as "mentions"
  forget "stale topic" after 30.days
  return "ok"
}
"#;
    let decls = metalogos::parser::parse(src).expect("relate/forget probe must parse");
    let pattern = decls
        .iter()
        .find_map(|d| match d {
            Declaration::Pattern(p) if p.name == "Link" => Some(p),
            _ => None,
        })
        .expect("Link pattern must be present");

    assert_eq!(pattern.body.len(), 3, "exactly [relate, forget, return]");
    match &pattern.body[0] {
        Statement::Relate(r) => {
            assert!(
                matches!(&r.to, Expr::Ident { name, .. } if name == "topic"),
                "relate `to` must be the param expression, got {:?}",
                r.to
            );
            assert_eq!(r.relation, "mentions");
        }
        other => panic!("body[0] must be Statement::Relate, got {:?}", other),
    }
    match &pattern.body[1] {
        Statement::Forget(f) => {
            assert_eq!(f.days, 30, "forget horizon must be 30 days");
        }
        other => panic!("body[1] must be Statement::Forget, got {:?}", other),
    }
}

/// Route/hook/test bodies are `statement*` too — the grammar must accept
/// memory statements there identically (they all funnel through the same
/// parse_single_statement).
#[test]
fn n266_memory_stmt_in_route_and_hook_bodies_parse() {
    let src = r#"
pattern Ping(x: String) -> String { return x }
hook before_pattern {
  memorize "hook-note" with priority=0.5
}
server {
  port: 8080
  route "/note" method=POST {
    memorize "route-note" with priority=0.6
    respond("ok")
  }
}
"#;
    let decls = metalogos::parser::parse(src).expect("route/hook probe must parse");

    let route_has_memorize = decls.iter().any(|d| match d {
        Declaration::MlogServer(srv) => srv.routes.iter().any(|r| {
            r.body.first().map(|s| matches!(s, Statement::Memorize(_))) == Some(true)
        }),
        _ => false,
    });
    assert!(route_has_memorize, "route body must start with memorize");

    let hook_has_memorize = decls.iter().any(|d| match d {
        Declaration::Hook(h) => {
            h.body.first().map(|s| matches!(s, Statement::Memorize(_))) == Some(true)
        }
        _ => false,
    });
    assert!(hook_has_memorize, "hook body must start with memorize");
}

/// PEG ordered choice must keep the pre-#266 behavior for lines that only
/// LOOK like memory keywords: `memorize = 5` is still an Assign, `forget(x)`
/// is still a call — the memory arms must not over-reach.
#[test]
fn n266_keyword_lookalikes_still_parse_as_before() {
    let src = r#"
pattern Overload(memorize: String) -> String {
  let mut relate = 5.0
  relate = 7.0
  return memorize + to_string(relate)
}
flow Main { input: String = "ok" -> Overload -> output }
"#;
    let decls = metalogos::parser::parse(src).expect("lookalike program must parse");
    let pattern = decls
        .iter()
        .find_map(|d| match d {
            Declaration::Pattern(p) if p.name == "Overload" => Some(p),
            _ => None,
        })
        .expect("Overload pattern must be present");
    assert!(
        matches!(&pattern.body[0], Statement::LetBinding { .. }),
        "let mut relate = 5.0 must stay a let binding, got {:?}",
        pattern.body[0]
    );
    assert!(
        matches!(&pattern.body[1], Statement::Assign { name, .. } if name == "relate"),
        "relate = 7.0 must stay an Assign, got {:?}",
        pattern.body[1]
    );

    // End-to-end: params named `memorize` keep working on TW.
    let out = metalogos::run_program(src).expect("lookalike program must run");
    assert_eq!(out.as_deref(), Some("ok7"), "TW output must be unchanged");
}

/// Part 2 of the fact, inverted: TW EXECUTES the statement — the flow output
/// proves the memory round-trip (memorize inside Remember, recall inside
/// RecallIt, same interpreter/session).
#[test]
fn n266_tw_executes_pattern_memory() {
    let out = metalogos::run_program(PROBE).expect("probe must run on TW");
    assert_eq!(
        out.as_deref(),
        Some("hello"),
        "TW must memorize `fact` inside the pattern and recall it — got {:?} \
         (the pre-#266 answer was error: undefined variable: memorize)",
        out
    );
}

/// Static pass must stay green for the legal statement form (no false
/// positives from the #264 mutability pass or the call/arity checks).
#[test]
fn n266_check_passes_for_memory_statements() {
    let result = metalogos::check_program(PROBE).expect("parse must succeed");
    assert!(
        result.is_ok(),
        "check must accept memory statements in bodies, got: {:?}",
        result.errors
    );
}

/// Part 3 of the fact, inverted: the VM EXECUTES the statement honestly.
/// Before #266 the compiler emitted `Instruction::Memorize` into the pattern
/// code but the VM's execute_code SILENTLY SKIPPED it (`_ => ip += 1`), so
/// recall returned "" — the silent half of the token-soup contract. The
/// parity contract here: same source, same answer as TW.
#[test]
fn n266_vm_executes_pattern_memory_same_as_tw() {
    let declarations = metalogos::parser::parse(PROBE).expect("probe must parse");
    let program = metalogos::compiler::Compiler::new()
        .compile(declarations)
        .expect("probe must compile");
    let mut vm = metalogos::vm::Vm::new();
    let out = vm.run(program).expect("probe must run on VM");
    assert_eq!(
        out.as_deref(),
        Some("hello"),
        "VM must memorize `fact` inside the pattern and recall it exactly like TW — got {:?}",
        out
    );
}

/// forget as a statement executes without breaking the store (a fresh entry
/// is younger than the 30-day horizon, so it survives; the pre-#266 VM
/// behavior was a silent opcode skip).
#[test]
fn n266_forget_stmt_runs_on_both_backends() {
    let src = r#"
pattern Cycle(x: String) -> String {
  forget "nothing-matches-this" after 30.days
  memorize x with priority=0.9
  return recall(x)
}
flow Main { input: String = "durable" -> Cycle -> output }
"#;
    let tw = metalogos::run_program(src).expect("TW must run forget+memorize statements");
    assert_eq!(tw.as_deref(), Some("durable"));

    let declarations = metalogos::parser::parse(src).expect("probe must parse");
    let program = metalogos::compiler::Compiler::new()
        .compile(declarations)
        .expect("probe must compile");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(program).expect("VM must run forget+memorize statements");
    assert_eq!(vm_out.as_deref(), Some("durable"), "VM parity for forget stmt");
}

/// Regression: top-level memorize (the p7/m4 contract) is untouched —
/// same program shape as tests/memory_persist_e2e.rs, in-memory store.
#[test]
fn n266_top_level_memorize_regression() {
    let src = r#"
memorize "ephemeral data" with priority=0.5
entity r: String = recall("ephemeral")
flow Main { input: String = r -> output }
"#;
    let out = metalogos::run_program(src).expect("top-level memorize must keep working");
    assert_eq!(out.as_deref(), Some("ephemeral data"));

    let decls = metalogos::parser::parse("memorize \"x\" with priority=0.9").unwrap();
    assert!(
        matches!(decls[0], Declaration::Memorize(_)),
        "top-level memorize must remain a Declaration::Memorize"
    );
}

/// The restored example: Contract 3 of examples/p8_route_patterns.mlog must
/// parse and check green (the on-disk example is the end-to-end artifact of
/// this naryad; its server runtime is covered by the route contract tests).
#[test]
fn n266_p8_example_contract3_restored() {
    let source = include_str!("../examples/p8_route_patterns.mlog");
    let decls = metalogos::parser::parse(source).expect("p8 must parse");
    let remember = decls
        .iter()
        .find_map(|d| match d {
            Declaration::Pattern(p) if p.name == "Remember" => Some(p),
            _ => None,
        })
        .expect("Remember pattern must be present");
    assert!(
        remember
            .body
            .iter()
            .any(|s| matches!(s, Statement::Memorize(m) if (m.priority - 0.8).abs() < 1e-9)),
        "p8 Remember must contain `memorize fact with priority=0.8` again"
    );

    let result = metalogos::check_program(source).expect("p8 must parse for check");
    assert!(
        result.errors.is_empty(),
        "p8 must be check-clean, got: {:?}",
        result.errors
    );

    // And the VM must execute the restored pattern body honestly: invoke
    // Remember's compiled code through a flow and recall what it stored.
    let vm_src = r#"
pattern Remember(fact: String) -> String {
  memorize fact with priority=0.8
  return "ok"
}
pattern RecallIt(x: String) -> String {
  return recall("hello")
}
flow Main { input: String = "hello" -> Remember -> RecallIt -> output }
"#;
    let program = metalogos::compiler::Compiler::new()
        .compile(metalogos::parser::parse(vm_src).unwrap())
        .unwrap();
    let mut vm = metalogos::vm::Vm::new();
    assert_eq!(
        vm.run(program).unwrap().as_deref(),
        Some("hello"),
        "the restored p8 Contract 3 shape must work end-to-end on the VM"
    );
}
