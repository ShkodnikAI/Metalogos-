// ── gh#1430 (fosved-acc АСС-1.9, naryad №772, wave 60): the VM try×pattern-call
// unwind wiped the caller's frame locals ──
//
// The reported shape (v0.33.2, sha256 48b98167…, reproduced by the executor
// before the fix): when `try` catches an error raised by a PATTERN CALL, every
// local already bound in the current frame reads back as Unit after the catch
// on the VM; TW keeps the values. A plain literal-bound local dies the same
// way (`Unit Sub Float`); the consumer-shaped variant dies with
// `Float Sub Unit` on the serve VM lane (fosved-acc site monitor — the
// duty route went silent during an outage, the exact blocking class the
// №476 rule names).
//
// Root cause (this fix): the CallPattern arms push a CallFrame and propagate
// the error with `?` — the callee's frame (and its stack region) stayed
// installed after the catch. Every later LoadLocal/StoreLocal resolved
// against the STALE base_bp → the pre-try locals read Unit. The fix restores
// BOTH depths (stack + call_stack) at the catch boundary of BOTH TryEval arms
// (the run loop and the inner loop — форма №762), the same discipline
// run_deny_handler (№392) already applies to its own body on every path.
//
// The matrix pins the whole class on both backends: (1) the issue's p3 shape
// (VM == TW 1-в-1, the pre-try locals live), (2) the consumer ms shape
// (`Float Sub Unit` before the fix), (3) the nested-frame shape (the stale
// frame of a SUCCESSFUL outer call after an inner catch — the success-path
// pop misalignment the same leak produced), (4) the inline-builtin boundary
// (locals survived before — must stay), (5) the try success path (must stay).

use metalogos::interpreter::Value;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), std::path::PathBuf::from("."))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn norm(v: &Result<Option<String>, String>) -> String {
    match v {
        Ok(Some(s)) => s.trim().to_string(),
        Ok(None) => "<none>".to_string(),
        Err(e) => format!("ERR: {}", e),
    }
}

// 1. THE issue shape (gh#1430 p3.mlog verbatim): pre-try locals `x` (a plain
//    literal binding — the report's "not about now()" fact) and `t0` must
//    survive the catch on BOTH engines. The `b=` token is time-dependent, so
//    the pin asserts it is a real float (the arithmetic RAN) instead of
//    byte-equality; the `r=`/`a=` tokens are deterministic and compared
//    1-в-1 (the pre-fix VM died with `Unit Sub Float` before reaching the
//    return).
const P3: &str = r#"
pattern Boom() -> Any {
  return to_float("abc")
}

pattern Main1(_: String) -> String {
  let x = 1.0
  let t0 = now()
  let r = try Boom()
  let a = x - 0.5
  let b = now() - t0
  return "r=" + to_string(r) + " a=" + to_string(a) + " b=" + to_string(b)
}

flow Main {
  input: String = "" -> Main1 -> output
}
"#;

fn assert_p3_answer(answer: &str, lane: &str) {
    // The `b=` token is time-dependent (the two engines run at different
    // moments) — the deterministic tokens are compared exactly, the b
    // arithmetic is asserted to have RUN (a computed float, not an error).
    let tokens: Vec<&str> = answer.split_whitespace().collect();
    assert_eq!(
        tokens.first().copied(),
        Some("r=()"),
        "{lane} r token: {answer:?}"
    );
    assert_eq!(
        tokens.get(1).copied(),
        Some("a=0.5"),
        "{lane} a token (the pre-try literal local): {answer:?}"
    );
    let b = tokens.get(2).copied().unwrap_or("");
    assert!(
        b.starts_with("b=0.0") && b[2..].parse::<f64>().is_ok(),
        "{lane} b token must be a computed float (the pre-try t0 survived): {answer:?}"
    );
}

#[test]
fn n1430_p3_pretry_locals_survive_the_catch() {
    let tw = norm(&run_tw(P3));
    let vm = norm(&run_vm(P3));
    assert_p3_answer(&tw, "TW");
    assert_p3_answer(&vm, "VM");
}

// 2. The consumer shape (p2.mlog / the fosved-acc duty monitor): the
//    pre-fix VM died with `Float Sub Unit` — the monitor stayed silent
//    during an outage. The ms arithmetic must RUN on both engines.
const P2: &str = r#"
pattern Boom() -> Any {
  return to_float("abc")
}

pattern Main1(_: String) -> String {
  let t0 = now()
  let r = try Boom()
  let ms = (now() - t0) * 1000.0
  return "ms=" + to_string(ms)
}

flow Main {
  input: String = "" -> Main1 -> output
}
"#;

#[test]
fn n1430_consumer_ms_shape_arithmetic_runs() {
    // Same time-dependent shape: each engine's answer is asserted on its
    // own (the ms arithmetic ran; the pre-fix VM died with
    // `Float Sub Unit` before answering anything).
    for (lane, answer) in [("TW", norm(&run_tw(P2))), ("VM", norm(&run_vm(P2)))] {
        let ms = answer.strip_prefix("ms=").unwrap_or("");
        assert!(
            ms.parse::<f64>().map(|v| v > 0.0).unwrap_or(false),
            "{lane} ms must be a computed positive float (t0 survived): {answer:?}"
        );
    }
}

// 3. The nested-frame shape: the inner catch leaves the callee frame stale;
//    the OUTER pattern then returns successfully, and the success-path
//    `call_stack.pop()` of the outer CallPattern arm popped the STALE frame
//    instead of its own — the caller's base_bp shifted again. Pre-fix the
//    reported death was the same `Unit Sub Float`; the honest record (the
//    wave report) carries the 1-в-1 reproduction. Both frames must be
//    balanced after the inner catch.
const P3C: &str = r#"
pattern Q() -> Any {
  return to_float("abc")
}
pattern P() -> Any {
  let z = 7.0
  let r = try Q()
  return z
}
pattern Main1(_: String) -> String {
  let x = 1.0
  let r = try P()
  let a = x - 0.5
  return "a=" + to_string(a) + " r=" + to_string(r)
}
flow Main { input: String = "" -> Main1 -> output }
"#;

#[test]
fn n1430_nested_frames_balanced_after_inner_catch() {
    let tw = norm(&run_tw(P3C));
    let vm = norm(&run_vm(P3C));
    assert_eq!(
        tw, vm,
        "gh#1430 nested: TW/VM divergence — tw={tw:?} vm={vm:?}"
    );
    assert_eq!(
        vm, "a=0.5 r=7",
        "nested: the outer pre-try local AND the inner caught value: {vm:?}"
    );
}

// 4. The boundary (the report's "same binary" fact): try over an INLINE
//    builtin call — no pattern in between — kept the locals on both engines
//    before the fix. The fix must not move this boundary (the truncate is a
//    no-op there: nothing is left on the stack, no frame is installed).
const INLINE: &str = r#"
pattern Main1(_: String) -> String {
  let x = 1.0
  let r = try to_float("abc")
  let a = x - 0.5
  return "r=" + to_string(r) + " a=" + to_string(a)
}
flow Main { input: String = "" -> Main1 -> output }
"#;

#[test]
fn n1430_inline_builtin_boundary_unchanged() {
    let tw = norm(&run_tw(INLINE));
    let vm = norm(&run_vm(INLINE));
    assert_eq!(
        tw, vm,
        "inline-builtin boundary moved — the gh#1430 fix over-reached: tw={tw:?} vm={vm:?}"
    );
    assert_eq!(vm, "r=() a=0.5", "inline-builtin boundary: {vm:?}");
}

// 5. The try success path (no error raised): the result rides the Ok arm —
//    untouched by the fix (граница: the semantics outside the unwind path
//    must not change).
const SUCCESS: &str = r#"
pattern Add(a: Float, b: Float) -> Float {
  return a + b
}
pattern Main1(_: String) -> String {
  let x = 1.0
  let r = try Add(2.0, 3.0)
  let a = x - 0.5
  return "r=" + to_string(r) + " a=" + to_string(a)
}
flow Main { input: String = "" -> Main1 -> output }
"#;

#[test]
fn n1430_try_success_path_untouched() {
    let tw = norm(&run_tw(SUCCESS));
    let vm = norm(&run_vm(SUCCESS));
    assert_eq!(tw, vm, "try success path diverged: tw={tw:?} vm={vm:?}");
    assert!(vm.starts_with("r=5"), "try success value: {vm:?}");
    assert!(vm.ends_with("a=0.5"), "try success path locals: {vm:?}");
}

// The serve-lane executor (the consumer's actual path — the ADR-0171 default
// VM backend): the route body runs through execute_route_code, the inner-loop
// TryEval arm. The p2 shape must answer 200-shaped there too (pre-fix this
// lane died with `Float Sub Unit` inside the route — the fosved-acc monitor
// stayed silent; the e2e HTTP harness in the consumer repo pinned the class,
// this unit pins the bytecode lane itself).
#[test]
fn n1430_route_body_lane_survives_the_catch() {
    let source = r#"
mlogserver {
  port: 8091
  route "/duty" method=GET {
    let t0 = now()
    let r = try Boom()
    let ms = (now() - t0) * 1000.0
    respond("ms=" + to_string(ms))
  }
}

pattern Boom() -> Any {
  return to_float("abc")
}
"#;
    let declarations = metalogos::parser::parse(source.trim()).expect("parse");
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    // The production boot order (server.rs build path): the full program
    // compiles FIRST (the pattern table fills), THEN compile_routes —
    // the route-body compiler resolves pattern calls from that table.
    let program = comp.compile(declarations.clone()).expect("compile");
    let config = declarations
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .expect("mlogserver block");
    let compiled_routes = comp.compile_routes(&config.routes).expect("compile_routes");
    let mut vm = metalogos::vm::Vm::new();
    vm.load_program(&program).expect("load_program");
    let out = vm
        .execute_route_code(&compiled_routes[0], &program)
        .expect("the route must survive the caught pattern-call error (gh#1430)");
    match out {
        Value::HttpResponse { body, status, .. } => {
            assert_eq!(status, 200, "the duty route answers 200 after the catch");
            let ms = body.strip_prefix("ms=").unwrap_or("");
            assert!(
                ms.parse::<f64>().map(|v| v > 0.0).unwrap_or(false),
                "route ms must be a computed positive float: {body:?}"
            );
        }
        other => panic!("expected HttpResponse, got {other:?}"),
    }
}
