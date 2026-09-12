// ── Наряд №277: property — TW/VM parity on a generated grammar subset ──
//
// Contract: programs generated from a CONSERVATIVE subset of the grammar
// parse, compile, and execute to the IDENTICAL output on both backends
// (tree-walking interpreter and bytecode VM).
//
// The subset (explicitly, per the naryad): numeric/string literals,
// arithmetic (+ - * /), string concatenation (+), let bindings, builtin
// calls (upper/lower/len/reverse/str), pattern calls, flows.
//
// Constructs EXCLUDED because of the documented ADR-0105 boundary (VM is
// experimental scope — not full-language equivalent): `match` (not
// compilable in the VM — pinned by server tests), `Expr::BlockIfElse` as a
// value (№129), memory/conversation state, learnables, reflex, server
// routes, and every IO/LLM builtin. Parity on the excluded surface is NOT
// claimed — that honesty is the whole point of ADR-0105.

use proptest::prelude::*;

fn number_expr() -> impl Strategy<Value = String> {
    prop_oneof![
        (0..1000i64).prop_map(|n| format!("{}.0", n)),
        (-100.0f64..100.0).prop_map(|f| format!("{:?}", f)),
    ]
}

fn string_expr() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("\"hello\"".to_string()),
        "[^\"\\\\\n]{0,12}".prop_map(|s| format!("\"{}\"", s)),
    ]
}

/// Generated expression over the safe subset (one level of combination).
fn expr() -> impl Strategy<Value = String> {
    prop_oneof![
        number_expr(),
        string_expr(),
        (number_expr(), number_expr()).prop_map(|(a, b)| format!("{} + {}", a, b)),
        (number_expr(), number_expr()).prop_map(|(a, b)| format!("{} - {}", a, b)),
        (number_expr(), number_expr()).prop_map(|(a, b)| format!("{} * {}", a, b)),
        (number_expr(), number_expr()).prop_map(|(a, b)| format!("{} / {}", a, b)),
        string_expr().prop_map(|s| format!("upper({})", s)),
        string_expr().prop_map(|s| format!("lower({})", s)),
        string_expr().prop_map(|s| format!("len({})", s)),
        string_expr().prop_map(|s| format!("reverse({})", s)),
        (string_expr(), string_expr()).prop_map(|(a, b)| format!("{} + {}", a, b)),
        (string_expr(), 0..1000i64).prop_map(|(s, n)| format!("{} + {}", s, n)),
    ]
}

fn program_with_lets(lets: &[(String, String)], return_expr: &str) -> String {
    format!(
        "pattern Parity(x: String) -> String {{\n{}  return {}\n}}\nflow Main {{\n  input: String = \"go\"\n  -> Parity\n  -> output\n}}\n",
        lets.iter()
            .map(|(n, e)| format!("  let {} = {}\n", n, e))
            .collect::<String>(),
        return_expr
    )
}

/// A pattern that CALLS another pattern with two generated args — pattern
/// calls and cross-pattern let-binding flow are the load-bearing parity
/// surface (the exact class №№37/182 regression tests guard).
const HELPER_PROGRAM: &str = "pattern let_pair(a: String, b: String) -> String {\n  let t = str(a)\n  let u = str(b)\n  return t + \":\" + u\n}\nflow Main {\n  input: String = \"go\"\n  -> let_pair\n  -> output\n}\n";

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let program = metalogos::compiler::Compiler::new().compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_parity(source: &str) {
    let tw = metalogos::run_program(source);
    let vm = run_vm(source);
    match (tw, vm) {
        (Ok(a), Ok(b)) => assert_eq!(
            a.map(|s| s.trim().to_string()),
            b.map(|s| s.trim().to_string()),
            "TW and VM disagree on:\n{}",
            source
        ),
        (Err(tw_err), Err(vm_err)) => {
            // Both reject — parity of rejection (loud on both backends).
            assert!(!tw_err.is_empty() && !vm_err.is_empty());
        }
        (tw_res, vm_res) => panic!(
            "ONE backend rejected a subset program:\n{}\nTW={:?}\nVM={:?}",
            source, tw_res, vm_res
        ),
    }
}

#[test]
fn property_tw_vm_parity_on_generated_programs() {
    let cfg = ProptestConfig::with_cases(192);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let strat = (
        expr(),                                              // returned expression
        prop::collection::vec(("[a-z]{2,6}", expr()), 0..3), // let bindings
    );
    let res = runner.run(&strat, |(ret, raw_lets)| {
        // Dedupe let names (mlog may reject shadowing — that rejection must
        // be parity too, but generating duplicates tests rejection parity,
        // not execution parity, so dedupe on purpose).
        let mut lets: Vec<(String, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (name, e) in raw_lets {
            if seen.insert(name.clone()) {
                lets.push((name, e));
            }
        }
        let source = program_with_lets(&lets, &ret);
        assert_parity(&source);

        // Pattern-call parity with generated args.
        let a = lets.first().map(|(_, e)| e.clone()).unwrap_or_else(|| "1.5".to_string());
        let b = lets.get(1).map(|(_, e)| e.clone()).unwrap_or_else(|| "2.5".to_string());
        let call_program = format!(
            "pattern Caller(x: String) -> String {{\n  return let_pair({}, {})\n}}\nflow Main {{\n  input: String = \"go\"\n  -> Caller\n  -> output\n}}\n{}",
            a, b, HELPER_PROGRAM
        );
        assert_parity(&call_program);
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("TW/VM parity property failed: {}", e));
}
