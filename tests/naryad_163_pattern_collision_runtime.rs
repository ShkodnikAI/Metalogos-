// ── НАРЯД #163 — pattern-name collisions on `run` / `serve` ──────────
//
// `mlog check` already reports `duplicate pattern`. The tree-walking
// loader (`modules.rs` HashMap::insert) used to overwrite silently.
// These contracts lock the new warning / --strict behaviour.

use metalogos::interpreter::Interpreter;
use std::fs;
use std::path::PathBuf;

fn parse_ok(src: &str) -> Vec<metalogos::ast::Declaration> {
    metalogos::parser::parse(src).unwrap_or_else(|e| panic!("parse {src:?}: {e}"))
}

fn scratch_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "mlog-n163-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&p).expect("temp dir");
    p
}

fn write_mod(dir: &std::path::Path, name: &str, body: &str) {
    fs::write(dir.join(format!("{name}.mlog")), body).expect("write module");
}

fn load(dir: PathBuf, source: &str, strict: bool) -> Result<Interpreter, String> {
    let mut interp = Interpreter::new();
    interp.set_base_dir(dir);
    interp.set_strict_pattern_names(strict);
    interp.run(parse_ok(source))?;
    Ok(interp)
}

#[test]
fn format_matches_check_prefix() {
    let msg = "duplicate pattern: HandleDev";
    assert!(msg.starts_with("duplicate pattern:"));
    let msg =
        "duplicate pattern: HandleDev (already defined in dept/dev, redefined in dept/chain)";
    assert!(msg.starts_with("duplicate pattern: HandleDev"));
    assert!(msg.contains("dept/dev"));
    assert!(msg.contains("dept/chain"));
}

#[test]
fn check_still_reports_duplicate_pattern() {
    let source = r#"
        pattern Foo(x: String) -> String { return x }
        pattern Foo(y: String) -> String { return y }
    "#;
    let result = metalogos::check_program(source).expect("check");
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("duplicate pattern: Foo")));
}

#[test]
fn two_imports_same_name_warns_and_runs() {
    let dir = scratch_dir();
    write_mod(
        &dir,
        "mod_a",
        "pattern Shared() -> String { return \"a\" }",
    );
    write_mod(
        &dir,
        "mod_b",
        "pattern Shared() -> String { return \"b\" }",
    );
    let source = concat!(
        "import mod_a as a\n",
        "import mod_b as b\n",
        "pattern Main() -> String { return \"ok\" }\n",
    );
    let interp = load(dir, source, false).expect("run must not fail on a collision");
    let warnings = interp.name_collision_warnings();
    assert_eq!(
        warnings.len(),
        1,
        "exactly one collision warning, got: {warnings:?}"
    );
    assert!(
        warnings[0].contains("duplicate pattern: Shared"),
        "shared wording with check: {}",
        warnings[0]
    );
    assert!(
        warnings[0].contains("mod_a") && warnings[0].contains("mod_b"),
        "warning names both origins: {}",
        warnings[0]
    );
}

#[test]
fn serve_bootstrap_warns_but_loads() {
    let dir = scratch_dir();
    write_mod(&dir, "mod_a", "pattern Shared() -> String { return \"a\" }");
    write_mod(&dir, "mod_b", "pattern Shared() -> String { return \"b\" }");
    let source = concat!(
        "import mod_a as a\n",
        "import mod_b as b\n",
        "mlogserver {\n",
        "  port: 0\n",
        "  route \"/\" method=GET { return \"ok\" }\n",
        "}\n",
    );
    let interp = load(dir, source, false).expect("serve bootstrap must not refuse to load");
    assert_eq!(interp.name_collision_warnings().len(), 1);
    assert!(interp.get_server_config().is_some());
}

#[test]
fn unique_names_are_silent() {
    let dir = scratch_dir();
    write_mod(&dir, "mod_a", "pattern Alpha() -> String { return \"a\" }");
    write_mod(&dir, "mod_b", "pattern Beta() -> String { return \"b\" }");
    let source = concat!(
        "import mod_a as a\n",
        "import mod_b as b\n",
        "pattern Gamma() -> String { return \"c\" }\n",
    );
    let interp = load(dir, source, false).expect("clean program runs");
    assert!(
        interp.name_collision_warnings().is_empty(),
        "no false-positive warnings: {:?}",
        interp.name_collision_warnings()
    );
}

#[test]
fn reimport_same_module_is_silent() {
    let dir = scratch_dir();
    write_mod(
        &dir,
        "mod_a",
        "pattern Shared() -> String { return \"a\" }",
    );
    write_mod(&dir, "mid", "import mod_a as a\n");
    let source = concat!(
        "import mid as mid\n",
        "import mod_a as a\n",
        "pattern Main() -> String { return \"ok\" }\n",
    );
    let interp = load(dir, source, false).expect("re-import is not an error");
    assert!(
        interp.name_collision_warnings().is_empty(),
        "idempotent re-import must not warn: {:?}",
        interp.name_collision_warnings()
    );
}

#[test]
fn strict_mode_fails_the_load() {
    let dir = scratch_dir();
    write_mod(&dir, "mod_a", "pattern Shared() -> String { return \"a\" }");
    write_mod(&dir, "mod_b", "pattern Shared() -> String { return \"b\" }");
    let source = "import mod_a as a\nimport mod_b as b\n";
    let err = match load(dir, source, true) {
        Err(e) => e,
        Ok(_) => panic!("strict mode must error"),
    };
    assert!(
        err.contains("duplicate pattern: Shared"),
        "strict error reuses check wording: {err}"
    );
}
