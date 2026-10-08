// №645 (issue #1132; the unified audit of 48301708, §6.2 R1): the
// conformance-suite runner — every pair tests/conformance/<id>.mlog +
// <id>.expected runs through the PRODUCTION path of BOTH backends (the
// TW run path; the VM gate+compile+run path, the №617 shape) and must
// match the expected record:
//
//   STATUS: ok     → both backends answer Ok with the EXACT OUT line;
//   STATUS: error  → both backends refuse with the EXPECTED stable code
//                    (the [CODE] stamp, №479/№385 class — the wording is
//                    deliberately NOT pinned here; the code is).
//
// The cross-backend agreement is the oracle: a pair whose backends
// diverge is a RED regardless of the .expected content — that is the
// «спецификация — эталон» machine (the audit §6.2: the fuzzer gets an
// oracle stronger than «two backends matched»). A divergence found by a
// NEW case lands as a loud no-progress + a separate naryad — never a
// silent .expected edit to fit a backend (М3).
//
// The blocking wiring: the `conformance (blocking)` CI job runs this
// suite; the №535 table row (scripts/ci/blocking_checks.tsv, the
// conformance surface) and fact_blocking_check_cells pin it (№645: 32).
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};

struct Expected {
    status: String,       // "ok" | "error"
    code: Option<String>, // the stable code, error pairs only
    out: Option<String>,  // the exact stdout, ok pairs only
}

fn parse_expected(path: &Path) -> Result<Expected, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let mut status = None;
    let mut code = None;
    let mut out = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("STATUS:") {
            status = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("CODE:") {
            code = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("OUT:") {
            out = Some(rest.trim().to_string());
        }
    }
    let status = status.ok_or_else(|| format!("{}: missing STATUS line", path.display()))?;
    if status != "ok" && status != "error" {
        return Err(format!(
            "{}: STATUS must be ok|error, got {}",
            path.display(),
            status
        ));
    }
    if status == "error" && code.is_none() {
        return Err(format!("{}: an error pair MUST pin CODE:", path.display()));
    }
    if status == "ok" && out.is_none() {
        return Err(format!("{}: an ok pair MUST pin OUT:", path.display()));
    }
    Ok(Expected { status, code, out })
}

fn stable_code(err: &str) -> Option<String> {
    // The №479/№385 stamp: the FIRST bracketed stable code of the message.
    let start = err.find('[')?;
    let end_rel = err[start + 1..].find(']')?;
    let id = &err[start + 1..start + 1 + end_rel];
    if id.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !id.is_empty() {
        Some(id.to_string())
    } else {
        None
    }
}

fn run_tw(source: &str, base: &Path) -> Result<String, String> {
    match metalogos::run_program_with_dir(source, base.to_path_buf()) {
        Ok(Some(out)) => Ok(out),
        Ok(None) => Ok(String::new()),
        Err(e) => Err(e),
    }
}

fn run_vm(source: &str, base: &Path) -> Result<String, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let module_decls = metalogos::semantic::resolve_imports_statically(&declarations, base)
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
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp
        .compile(merged_decls)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    match vm.run(program) {
        Ok(Some(out)) => Ok(out),
        Ok(None) => Ok(String::new()),
        Err(e) => Err(e),
    }
}

fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
}

#[test]
fn n645_conformance_pairs_both_backends() {
    let dir = conformance_dir();
    let mut pairs: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read the conformance dir {}: {}", dir.display(), e))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "mlog").unwrap_or(false))
        .collect();
    pairs.sort();
    assert!(
        pairs.len() >= 12,
        "the №645 topic-1 corpus carries at least 12 pairs, got {}",
        pairs.len()
    );
    let base = conformance_dir();
    let mut failures: Vec<String> = Vec::new();
    for mlog in &pairs {
        let id = mlog
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let expected_path = mlog.with_extension("expected");
        if !expected_path.exists() {
            failures.push(format!("{}: no .expected pair", id));
            continue;
        }
        let expected = match parse_expected(&expected_path) {
            Ok(e) => e,
            Err(e) => {
                failures.push(e);
                continue;
            }
        };
        let source = fs::read_to_string(mlog).unwrap_or_else(|e| panic!("{}: {}", id, e));
        let tw = run_tw(&source, &base);
        let vm = run_vm(&source, &base);
        // The cross-backend agreement — the oracle. A divergence is red
        // EVEN IF both sides would match some other .expected.
        let class = |r: &Result<String, String>| match r {
            Ok(_) => "ok".to_string(),
            Err(e) => stable_code(e).unwrap_or_else(|| "UNSTABLE-ERROR".to_string()),
        };
        if class(&tw) != class(&vm) {
            failures.push(format!(
                "{}: the BACKENDS DIVERGE — TW {:?} vs VM {:?} (a loud \
                 finding: a separate naryad, never a silent .expected fit)",
                id,
                tw.as_ref()
                    .map(|s| s.trim())
                    .map(|s| s.chars().take(80).collect::<String>()),
                vm.as_ref()
                    .map(|s| s.trim())
                    .map(|s| s.chars().take(80).collect::<String>())
            ));
            continue;
        }
        match expected.status.as_str() {
            "ok" => {
                let want = expected.out.as_deref().unwrap_or("");
                for (name, r) in [("TW", &tw), ("VM", &vm)] {
                    match r {
                        Ok(out) => {
                            if out.trim() != want {
                                failures.push(format!(
                                    "{}: {} output drift — want {:?}, got {:?}",
                                    id,
                                    name,
                                    want,
                                    out.trim().chars().take(120).collect::<String>()
                                ));
                            }
                        }
                        Err(e) => failures.push(format!(
                            "{}: {} must run, refused: {}",
                            id,
                            name,
                            e.chars().take(120).collect::<String>()
                        )),
                    }
                }
            }
            "error" => {
                let want_code = expected.code.as_deref().unwrap_or("");
                for (name, r) in [("TW", &tw), ("VM", &vm)] {
                    match r {
                        Ok(out) => failures.push(format!(
                            "{}: {} must REFUSE, answered Ok({:?})",
                            id,
                            name,
                            out.trim().chars().take(60).collect::<String>()
                        )),
                        Err(e) => {
                            let got = stable_code(e).unwrap_or_default();
                            if got != want_code {
                                failures.push(format!(
                                    "{}: {} refusal code drift — want [{}], \
                                     got {:?} ({})",
                                    id,
                                    name,
                                    want_code,
                                    got,
                                    e.chars().take(100).collect::<String>()
                                ));
                            }
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }
    assert!(
        failures.is_empty(),
        "the conformance suite is RED:\n  - {}",
        failures.join("\n  - ")
    );
}
