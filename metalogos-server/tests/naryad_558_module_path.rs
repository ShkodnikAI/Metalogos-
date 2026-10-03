// ── tests/naryad_558_module_path.rs ──────────────────────────────────
// №558 (Wave 25 P1; the audit 02.10 M-8; dispatch gh#925): the ONE
// module-search rule — `src/module_path.rs::resolve_module_file` is the
// single implementation, and all THREE call sites (the interpreter's
// runtime loader, the compiler's `resolve_import`, the semantic
// `resolve_imports_statically`) call it.
//
// The behavior is byte-identical: the previous forms
// (`format!("{}.mlog")` on the runtime loader and the semantic pass,
// `with_extension("mlog")` on the compiler) are equivalent on every
// reachable input — the import-path grammar admits no dots
// (`import_path_segments = { IDENT ~ (SLASH ~ IDENT)* }`), so the
// extension-replace could never fire. The fixtures pin the three sites
// × three shapes (found / not found / nested path) BEFORE-and-AFTER the
// collapse: the same programs resolved the same files and refuse with
// the same messages.
#![allow(clippy::disallowed_methods)]

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mlog")
}

static FIXTURE_SEQ: AtomicUsize = AtomicUsize::new(0);

/// The fixture tree: main.mlog imports a flat module, a nested module and
/// (in the missing variant) a nonexistent one.
struct Fixture {
    dir: std::path::PathBuf,
}

impl Fixture {
    fn build(with_missing: bool) -> Self {
        // a UNIQUE directory per fixture instance — the tests run in
        // parallel, and a shared path would race one Fixture's Drop
        // against another test's child process
        let n = FIXTURE_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "n558_{}_{}_{}",
            std::process::id(),
            n,
            if with_missing { "miss" } else { "ok" }
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        // the flat module (found shape)
        std::fs::write(
            dir.join("my_utils.mlog"),
            "pattern shout(s: String) -> String {\n  return s\n}\n",
        )
        .unwrap();
        // the nested module (nested shape)
        std::fs::write(
            dir.join("lib/extra.mlog"),
            "pattern extra(s: String) -> String {\n  return s\n}\n",
        )
        .unwrap();
        let import_line = if with_missing {
            "import ghost_mod as g\nimport my_utils as u\nimport lib/extra as e"
        } else {
            "import my_utils as u\nimport lib/extra as e"
        };
        std::fs::write(
            dir.join("main.mlog"),
            format!(
                "{import_line}\npattern main_go() -> String {{\n  return u.shout(\"hi\") + e.extra(\"!\")\n}}\n"
            ),
        )
        .unwrap();
        Self { dir }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// ── site 1: the runtime loader (mlog run) ────────────────────────────

#[test]
fn n558_runtime_loader_found_and_nested_resolve() {
    let fx = Fixture::build(false);
    let out = Command::new(bin())
        .current_dir(&fx.dir)
        .args(["run", "main.mlog"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the flat and the nested imports resolve through the ONE rule; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn n558_runtime_loader_missing_refuses() {
    let fx = Fixture::build(true);
    let out = Command::new(bin())
        .current_dir(&fx.dir)
        .args(["run", "main.mlog"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a missing module refuses");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("ghost_mod"),
        "the refusal names the module: {}",
        text
    );
}

// ── site 2: the compiler (mlog compile) ──────────────────────────────

#[test]
fn n558_compiler_found_and_nested_resolve() {
    let fx = Fixture::build(false);
    let out = Command::new(bin())
        .current_dir(&fx.dir)
        .args(["compile", "main.mlog"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the compiler resolves both shapes through the ONE rule; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn n558_compiler_missing_refuses() {
    let fx = Fixture::build(true);
    let out = Command::new(bin())
        .current_dir(&fx.dir)
        .args(["compile", "main.mlog"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a missing module refuses the compile"
    );
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("ghost_mod"),
        "the refusal names the module: {}",
        text
    );
}

// ── site 3: the semantic static resolution (the direct contract) ─────

#[test]
fn n558_semantic_resolution_found_nested_and_missing() {
    let fx = Fixture::build(false);
    let source = std::fs::read_to_string(fx.dir.join("main.mlog")).unwrap();
    let declarations = metalogos::parser::parse(&source).unwrap();
    let merged = metalogos::semantic::resolve_imports_statically(&declarations, &fx.dir)
        .expect("both import shapes resolve statically through the ONE rule");
    // the modules' patterns are IN the merged set (the flat and the nested)
    let names: Vec<String> = merged
        .iter()
        .filter_map(|d| match d {
            metalogos::ast::Declaration::Pattern(p) => Some(p.name.clone()),
            _ => None,
        })
        .collect();
    assert!(
        names.contains(&"shout".to_string()) && names.contains(&"extra".to_string()),
        "the merged declarations carry both modules' patterns: {:?}",
        names
    );

    // the missing shape refuses loudly (the same rule, the same base)
    let fx_miss = Fixture::build(true);
    let source = std::fs::read_to_string(fx_miss.dir.join("main.mlog")).unwrap();
    let declarations = metalogos::parser::parse(&source).unwrap();
    let err = metalogos::semantic::resolve_imports_statically(&declarations, &fx_miss.dir)
        .expect_err("a missing module refuses the static resolution");
    assert!(
        err.contains("ghost_mod"),
        "the refusal names the module: {}",
        err
    );
}

// ── the SSOT structural pin ──────────────────────────────────────────

#[test]
fn n558_the_three_sites_call_the_one_rule() {
    // The source-level pin: each site delegates to resolve_module_file —
    // no site composes the path inline anymore (the №551-sync posture:
    // the source is the contract).
    let loader = include_str!("../../src/interpreter/modules.rs");
    let compiler = include_str!("../../src/compiler.rs");
    let semantic = include_str!("../../src/semantic.rs");
    for (site, src) in [
        ("the runtime loader", loader),
        ("the compiler", compiler),
        ("the semantic pass", semantic),
    ] {
        assert!(
            src.contains("module_path::resolve_module_file"),
            "{site} must call the ONE rule (src/module_path.rs)"
        );
    }
    // the inline compositions are GONE
    assert!(
        !loader.contains("join(format!(\"{}.mlog\""),
        "the runtime loader must not compose the path inline"
    );
    assert!(
        !compiler.contains("with_extension(\"mlog\")"),
        "the compiler must not compose the path inline"
    );
    assert!(
        !semantic.contains("join(format!(\"{}.mlog\""),
        "the semantic pass must not compose the path inline"
    );
    // and the SSOT itself carries the rule
    let ssot = include_str!("../../src/module_path.rs");
    assert!(ssot.contains("fn resolve_module_file"));
}
