//! Наряд №374 (ADR-0142, candidate (б)): error-protocol — `try` returns the
//! structured result `Struct { ok: Bool, value: Value, error: Unit |
//! Struct { code, message } }` on BOTH backends (the shared
//! `try_result_struct` builder makes divergence impossible).
//!
//! Pre-№374 behavior (Наряд №91): `try` returned the bare inner value on
//! success and discarded the error as `Unit` — for agentic programs this
//! forced awkward workarounds (`type_of(r) == "Unit"` probes). The migration
//! idiom is `r.ok == false` (mlog has no unary `!` — REFERENCE §Migration).

use std::fs;
use std::path::{Path, PathBuf};

/// Execute via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

/// Execute via bytecode VM.
fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

/// Assert both backends agree on the trimmed Ok output.
fn assert_parity(name: &str, source: &str) {
    let base_dir = PathBuf::from("examples");
    let tw =
        run_tw(source, &base_dir).unwrap_or_else(|e| panic!("{}: TW must succeed: {}", name, e));
    let vm =
        run_vm(source, &base_dir).unwrap_or_else(|e| panic!("{}: VM must succeed: {}", name, e));
    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    assert_eq!(tw_out, vm_out, "{}: TW and VM outputs diverge", name);
}

fn program(body: &str) -> String {
    format!(
        "pattern T(_input: String) -> String {{\n{}\n}}\nflow Main {{ input: String = \"s\" -> T -> output }}",
        body
    )
}

/// Success path: ok = true, value carries the inner value, error is Unit.
#[test]
fn naryad_374_success_path_shape() {
    assert_parity(
        "success_shape",
        &program(
            "  let r = try to_string(42.0)\n  return to_string(r.ok) + \"/\" + r.value + \"/\" + to_string(type_of(r.error) == \"Unit\")",
        ),
    );
    // The inner value keeps its TYPE through the wrap (float passthrough).
    assert_parity(
        "success_value_type",
        &program(
            "  let r = try (2.0 + 3.0)\n  return to_string(r.ok) + \"/\" + to_string(r.value) + \"/\" + type_of(r.value)",
        ),
    );
}

/// Error path: ok = false, value = Unit, error carries code + message.
#[test]
fn naryad_374_error_path_shape() {
    assert_parity(
        "error_shape",
        &program(
            "  let r = try (1.0 / 0.0)\n  return to_string(r.ok) + \"/\" + to_string(type_of(r.value) == \"Unit\") + \"/\" + r.error.code + \"/\" + r.error.message",
        ),
    );
    // A builtin called with a WRONG TYPE errors at runtime — try catches it
    // (unknown functions are rejected at COMPILE time, outside try's reach).
    assert_parity(
        "error_wrong_arg_type",
        &program(
            "  let r = try str(1.0, 2.0, 3.0, 4.0)\n  return to_string(r.ok == false) + \"/\" + to_string(type_of(r.error) == \"Struct\")",
        ),
    );
    // The error message is carried verbatim in error.message.
    assert_parity(
        "error_message_content",
        &program("  let r = try (1.0 / 0.0)\n  return r.error.message"),
    );
}

/// Nested try: the inner try catches its own error; the outer try sees the
/// inner STRUCTURED result as a plain success value (no re-wrap confusion).
/// NOTE: the grammar binds `try` to a unary_expr — nesting goes through
/// `let` bindings, which is also the idiomatic agentic-program form.
#[test]
fn naryad_374_nested_try() {
    assert_parity(
        "nested_try_via_let",
        &program(
            "  let inner = try (1.0 / 0.0)\n  let outer = try to_string(inner.ok)\n  return to_string(outer.ok) + \"/\" + outer.value",
        ),
    );
    // The inner error does not leak into the outer try's error path.
    assert_parity(
        "nested_error_isolated",
        &program(
            "  let inner = try (1.0 / 0.0)\n  let outer = try (2.0 + 2.0)\n  return to_string(inner.ok == false) + \"/\" + to_string(outer.ok == true) + \"/\" + to_string(outer.value)",
        ),
    );
}

/// try inside route/pattern bodies — the VM's SECOND interpretation loop
/// (execute_code) and the TW's statement path must agree (pattern bodies use
/// the shared VM execute_code path; flow Main drives the pattern).
#[test]
fn naryad_374_try_in_pattern_and_route_bodies() {
    // Pattern body (route handler equivalent).
    assert_parity(
        "try_in_pattern_body",
        &program(
            "  let r = try (\"a\" / \"b\")\n  if r.ok == false { return \"caught:\" + r.error.message }\n  return \"uncaught\"",
        ),
    );
    // try whose SUCCESS value feeds further computation in the body.
    assert_parity(
        "try_success_feeds_body",
        &program("  let r = try (2.0 * 3.0)\n  return to_string(r.value * 2.0)"),
    );
    // Multiple try sites in one body.
    assert_parity(
        "try_multiple_sites",
        &program(
            "  let a = try (1.0 / 0.0)\n  let b = try (2.0 / 4.0)\n  let c = try (\"x\" / \"y\")\n  return to_string(a.ok == false) + \"/\" + to_string(b.ok == true) + \"/\" + to_string(c.ok == false)",
        ),
    );
}

/// Migration regression: the p91 golden contract (success-path type checks)
/// migrated to the `.ok` / `.value` idiom keeps its 4/4 golden output on both
/// backends.
#[test]
fn naryad_374_p91_migration_regression() {
    let base_dir = PathBuf::from("examples");
    let path = base_dir.join("p91_try_success_path.mlog");
    let source = fs::read_to_string(&path).expect("p91 example exists");
    let expected =
        fs::read_to_string(path.with_extension("expected")).expect("p91 expected exists");

    let tw = run_tw(&source, &base_dir).expect("p91 runs on TW");
    let vm = run_vm(&source, &base_dir).expect("p91 runs on VM");
    let exp = expected.trim_end();
    assert_eq!(tw.as_deref().map(str::trim_end).unwrap_or_default(), exp);
    assert_eq!(
        vm.as_deref().map(str::trim_end).unwrap_or_default(),
        exp,
        "p91: VM output must match .expected (№374 migration)"
    );
}

/// The old pre-№374 idiom is GONE from the golden examples: no
/// `type_of(x) == "Unit"` probe survives on try-results.
#[test]
fn naryad_374_no_stale_unit_probes_in_examples() {
    let examples = Path::new("examples");
    let mut offenders = Vec::new();
    for entry in fs::read_dir(examples).expect("examples dir").flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "mlog").unwrap_or(false) {
            let s = fs::read_to_string(&path).unwrap_or_default();
            let try_vars = find_try_vars(&s);
            for var in try_vars {
                let probe = format!("type_of({}) == \"Unit\"", var);
                if s.contains(&probe) {
                    offenders.push(format!("{}: {}", path.display(), probe));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "stale try-Unit probes must be migrated to `.ok == false` (№374): {:?}",
        offenders
    );
}

/// Find `let NAME = try ...` variable names in the source (identifier capture
/// without a regex dependency: scan line-wise).
fn find_try_vars(src: &str) -> Vec<String> {
    let mut vars = Vec::new();
    for line in src.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("let ") {
            if let Some(eq) = rest.find("= try") {
                let name = rest[..eq].trim();
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    vars.push(name.to_string());
                }
            }
        }
    }
    vars
}

/// No stubs (№16.0-D) — markers assembled from parts to avoid self-matching.
#[test]
fn naryad_374_no_stubs() {
    let markers = [
        concat!("todo", "!"),
        concat!("unimplemented", "!"),
        concat!("SKELE", "TON"),
    ];
    let src = fs::read_to_string(file!()).unwrap_or_default();
    for m in markers {
        assert!(
            !src.contains(m),
            "naryad_374 test contains stub marker {}",
            m
        );
    }
}
