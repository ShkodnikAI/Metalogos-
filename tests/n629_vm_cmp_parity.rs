// Naryad №629 (gh#1096): the VM comparison parity — eval_cmp mirrors the
// TW matrix 1:1. The pins run the SAME program through the production
// order of BOTH backends (the TW run path; the VM gate+compile+run path,
// the №617 shape) and demand the SAME outcome class: the heterogeneous
// / incomparable comparisons refuse on BOTH (the №479 '+' precedent),
// the Unit Eq semantics agree, the healthy numeric/string comparisons
// stay green (the non-regress arms).
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

fn run_tw(source: &str, base_dir: &PathBuf) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.clone())
}

fn run_vm(source: &str, base_dir: &PathBuf) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    // №523 parity: the production `mlog run --backend vm` applies the
    // semantic gate BEFORE the compile (the №617 shape — mirrored).
    let module_decls = metalogos::semantic::resolve_imports_statically(&declarations, base_dir)
        .map_err(|e| format!("Compilation error (Naryad #523): {}", e))?;
    let mut merged_decls = module_decls;
    merged_decls.extend(declarations.clone());
    let sem_result = metalogos::semantic::check_program(&merged_decls);
    let blocking: Vec<&metalogos::semantic::SpannedError> = sem_result
        .errors
        .iter()
        .filter(|err| !metalogos::semantic::is_exempt_from_blocking(err.kind))
        .collect();
    if !blocking.is_empty() {
        let code = blocking.iter().find_map(|err| err.kind.stable_code());
        let stamp = code.map(|c| format!("[{}] ", c)).unwrap_or_default();
        let lines: Vec<String> = blocking
            .iter()
            .map(|err| metalogos::semantic::format_blocking_line(err))
            .collect();
        return Err(format!(
            "Compilation error (Naryad #523): semantic findings block execution:\n{}{}",
            stamp,
            lines.join("\n")
        ));
    }
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.clone());
    let program = comp
        .compile(merged_decls)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn program_of(body: &str) -> String {
    format!(
        "pattern p(x: String) -> String {{\n  let a = {}\n  return to_string(a)\n}}\nflow Main {{\n  input: String = \"go\" -> p -> output\n}}\n",
        body
    )
}

fn assert_both_refuse(name: &str, body: &str) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = program_of(body);
    let tw = run_tw(&src, &repo);
    let vm = run_vm(&src, &repo);
    let tw_err = tw.expect_err(format!("{}: TW must refuse", name).as_str());
    let vm_err = vm.expect_err(format!("{}: VM must refuse", name).as_str());
    assert!(
        tw_err.starts_with("[TYPE_MISMATCH] ") && vm_err.contains("[TYPE_MISMATCH]"),
        "{}: both must stamp TYPE_MISMATCH, got tw: {} / vm: {}",
        name,
        tw_err,
        vm_err
    );
}

fn assert_both_ok(name: &str, body: &str, expected: &str) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = program_of(body);
    let tw = run_tw(&src, &repo).expect(format!("{}: TW must run", name).as_str());
    let vm = run_vm(&src, &repo).expect(format!("{}: VM must run", name).as_str());
    assert_eq!(
        tw.as_deref(),
        Some(expected),
        "{}: TW output mismatch",
        name
    );
    assert_eq!(
        vm.as_deref(),
        Some(expected),
        "{}: VM output mismatch (the parity target)",
        name
    );
}

#[test]
fn n629_heterogeneous_eq_refuses_on_both_backends() {
    // The №621-sweep finding: TW refused, the VM answered the silent
    // `false` — now BOTH refuse with the SAME stable code.
    assert_both_refuse("string_eq_float", "\"abc\" == 5.0");
    assert_both_refuse("numeric_string_eq_float", "\"5\" == 5.0");
    assert_both_refuse("bool_eq_float", "true == 5.0");
    assert_both_refuse("float_eq_bool", "5.0 == false");
}

#[test]
fn n629_ordering_ops_are_float_only_on_both_backends() {
    // The legacy VM compared Bool/Bool and String/String lexicographically
    // while the TW refused — now BOTH refuse.
    assert_both_refuse("bool_gt_bool", "true > false");
    assert_both_refuse("string_gt_string", "\"b\" > \"a\"");
    assert_both_refuse("string_lt_string", "\"a\" < \"b\"");
    assert_both_refuse("bool_le_bool", "true <= false");
}

#[test]
fn n629_unit_eq_semantics_agree() {
    // Unit == Unit -> true; Unit == x -> false (the TW matrix verbatim;
    // the legacy VM answered `false` for Unit == Unit).
    assert_both_ok("unit_eq_unit", "random_seed(1) == random_seed(1)", "true");
    assert_both_ok("unit_eq_float", "random_seed(1) == 5.0", "false");
    assert_both_ok("float_eq_unit", "5.0 == random_seed(1)", "false");
}

#[test]
fn n629_incomparable_struct_refuses_on_both_backends() {
    // The legacy VM answered the silent `false` for Struct/List/Html
    // comparisons — now BOTH refuse.
    assert_both_refuse("struct_eq_struct", "{ a: 1.0 } == { a: 1.0 }");
}

#[test]
fn n629_healthy_comparisons_stay_green_on_both_backends() {
    // The non-regress arms: the healthy Float/String/Bool comparisons
    // agree and keep their values.
    assert_both_ok("float_eq_true", "5.0 == 5.0", "true");
    assert_both_ok("float_eq_false", "5.0 == 4.0", "false");
    assert_both_ok("string_eq_true", "\"ab\" == \"ab\"", "true");
    assert_both_ok("string_eq_false", "\"ab\" == \"cd\"", "false");
    assert_both_ok("bool_eq_true", "true == true", "true");
    assert_both_ok("float_gt", "5.0 > 4.0", "true");
    assert_both_ok("float_le", "4.0 <= 4.0", "true");
    assert_both_ok("ne_true", "5.0 != 4.0", "true");
    assert_both_ok("ne_string", "\"a\" != \"b\"", "true");
}
