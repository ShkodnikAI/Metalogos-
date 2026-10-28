//! Наряд №829 (issue #1622, wave 69, P1, класс [correctness]): the VM
//! dispatch resolves the callee BY NAME — independent of `profile legacy`
//! blocks, declaration order, unit size, and import shadows (fix #1616 +
//! #1618).
//!
//! Root cause (the report's mechanics): pass1 allocated the positional
//! CallPattern index as `pattern_indices.len()` — the size of a NAME-keyed
//! map. A redeclared name (the office stub convention: a local stub
//! shadows an imported pattern) kept the map size flat, so the next FRESH
//! pattern received the shadow's index and every later index shifted
//! against the pass2 `pattern_bodies` table — a call executed the pattern
//! defined immediately before the target; silent when arities coincide
//! (the dangerous #1616 mode), a wrong-name arity error otherwise (#1618's
//! «pattern X expects N args, got 0»). The `profile legacy` line was a
//! CORRELATED switch, not the cause: units without the profile never
//! reached execution (the №523 SECRET_EGRESS gate refused first), so the
//! isolation table in #1616 read the gate's refusal as «bug gone». The
//! fix: the index comes from a BODY-COUNT allocator that mirrors the
//! collector 1:1, redeclaration keeps last-wins semantics (the TW
//! registry shape), and a loud compile-time gate refuses any name→slot
//! desync before bytecode exists.
//!
//! RED-first (форма волны): every behavioral test below fails on the
//! UNPATCHED main — the silent-shadow unit returned «DECOY» from the
//! preceding body, the args-lost unit refused with the WRONG pattern's
//! name («pattern Mark expects 2 args, got 0»), and the flow-step lane
//! executed the SHADOWED (imported) body. After the fix all lanes go
//! green and VM == TW on every unit.
//!
//! The two negative controls (стандарт «2 негативных контроля»):
//! (а) removing `profile legacy` from the unit does not change dispatch
//!     results — the profile is an egress POLICY, not call semantics;
//! (b) the TW lane on the same files behaves exactly as before — golden
//!     oracle untouched (the diff never enters the interpreter).

#![allow(clippy::disallowed_methods)] // the №475 fixture-writer lane: tests write their OWN temp modules

use std::fs;
use std::path::{Path, PathBuf};

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// Write `module_src` under `<dir>/fixtures829/mod.mlog` and return the
/// dir. The slashed import path (`fixtures829/mod`) dodges the pre-existing
/// import-path whitespace quirk (a comment directly after a slash-free
/// import pollutes the path — a separate finding, NOT this naryad's class).
fn write_fixture(tag: &str, module_src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("n829_{}_{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("fixtures829")).expect("create fixture dir");
    fs::write(dir.join("fixtures829/mod.mlog"), module_src).expect("write module fixture");
    dir
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

/// Copy the repo's `std/collections.mlog` into the fixture's std/ root —
/// the module rule resolves `std/*` against the base dir (№558), and the
/// map()-lane unit needs the import for `collections_loaded`.
fn copy_std_collections(dir: &Path) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("std/collections.mlog");
    let std_dir = dir.join("std");
    fs::create_dir_all(&std_dir).expect("create std dir");
    fs::copy(&src, std_dir.join("collections.mlog")).expect("copy std/collections.mlog");
}

// ── The shared module fixture: the import whose name the entry SHADOWS ──
const SHADOW_MODULE: &str = r#"
pattern Shim(a: String) -> String {
  return "imported"
}
"#;

// The silent-corruption unit (#1616 form): `profile legacy` + a shadowed
// import + a same-arity pair (Decoy/Target) + a 0-arg expression call.
// Pre-fix: `Target()` resolved to Decoy's body and returned «DECOY» with
// no error — the dangerous silent mode.
const SILENT_UNIT: &str = r#"
import fixtures829/mod
profile legacy { egress: permissive_with_audit }

pattern Shim(a: String) -> String {
  return "local"
}
pattern Decoy() -> String {
  return "DECOY"
}
pattern Target() -> String {
  return "TARGET"
}
pattern Runner(input: String) -> String {
  let direct = Target()
  return direct
}
flow Main {
  input: String = "" -> Runner -> output
}
"#;

// The args-lost unit (#1618 form): the accumulator construction in a frame
// with an import + a stub shadow + a 2-arg/0-arg mix. Pre-fix: `Probe()`
// refused with «pattern Mark expects 2 args, got 0» (the WRONG name — the
// preceding body), and the accumulator's `Mark(...)` call routed its args
// to the 0-arg Target («pattern Target expects 0 args, got 2»).
const ARGS_LOST_UNIT: &str = r#"
import fixtures829/mod
profile legacy { egress: permissive_with_audit }

pattern Shim(a: String) -> String {
  return "local"
}
pattern Mark(name: String, ok: Bool) -> String {
  if ok then { return name + "=true;" }
  return name + "=false;"
}
pattern Probe() -> String {
  return "probe-ok"
}
pattern Runner(input: String) -> String {
  let p = Probe()
  let out = p + Mark("b1", true)
  return out
}
flow Main {
  input: String = "" -> Runner -> output
}
"#;

// The same units WITHOUT the profile line — negative control (а): the
// dispatch results must be IDENTICAL (the profile never enters the
// compiler's call resolution; pre-fix the mis-dispatch fired here too —
// the unit simply runs where the office llm-carrying files were gated by
// №523 first, which is what masked the cause in #1616's isolation).
const SILENT_UNIT_NO_PROFILE: &str = r#"
import fixtures829/mod

pattern Shim(a: String) -> String {
  return "local"
}
pattern Decoy() -> String {
  return "DECOY"
}
pattern Target() -> String {
  return "TARGET"
}
pattern Runner(input: String) -> String {
  let direct = Target()
  return direct
}
flow Main {
  input: String = "" -> Runner -> output
}
"#;

const ARGS_LOST_UNIT_NO_PROFILE: &str = r#"
import fixtures829/mod

pattern Shim(a: String) -> String {
  return "local"
}
pattern Mark(name: String, ok: Bool) -> String {
  if ok then { return name + "=true;" }
  return name + "=false;"
}
pattern Probe() -> String {
  return "probe-ok"
}
pattern Runner(input: String) -> String {
  let p = Probe()
  let out = p + Mark("b1", true)
  return out
}
flow Main {
  input: String = "" -> Runner -> output
}
"#;

// ── RED (а): the silent mode — the call executes the TARGET body ────────

/// The 0-arg call after a same-arity predecessor runs ITS OWN body —
/// never the preceding pattern silently (pre-fix: «DECOY»).
#[test]
fn n829_silent_shadow_executes_the_target_body() {
    let dir = write_fixture("silent", SHADOW_MODULE);
    let out = run_vm(SILENT_UNIT, &dir).expect("VM run must succeed");
    let out = out.unwrap_or_default();
    assert!(
        out.contains("TARGET"),
        "VM must execute the target body, got: {out}"
    );
    assert!(
        !out.contains("DECOY"),
        "silent corruption: the preceding body executed instead of the target: {out}"
    );
    cleanup(&dir);
}

// ── RED (б): the args-lost form — the right pattern, the right arity ────

/// The accumulator construction with imports + stubs: `Probe()` runs its
/// own 0-arg body and `Mark(...)` receives its 2 args (pre-fix: the
/// wrong-name arity refusals of the #1618 battery form).
#[test]
fn n829_args_lost_accumulator_resolves_by_name() {
    let dir = write_fixture("args", SHADOW_MODULE);
    let out = run_vm(ARGS_LOST_UNIT, &dir).expect("VM run must succeed");
    let out = out.unwrap_or_default();
    assert!(
        out.contains("probe-ok"),
        "the 0-arg call must execute Probe's own body, got: {out}"
    );
    assert!(
        out.contains("b1=true"),
        "the 2-arg call must reach Mark with its arguments, got: {out}"
    );
    cleanup(&dir);
}

// ── Negative control (а): the profile line does not change dispatch ─────

/// The same units WITHOUT `profile legacy` produce the SAME dispatch
/// results — the profile is an egress policy, not call semantics.
#[test]
fn n829_profile_removal_does_not_change_dispatch() {
    let dir = write_fixture("noprofile", SHADOW_MODULE);
    let with_silent = run_vm(SILENT_UNIT, &dir)
        .expect("with-profile run must succeed")
        .unwrap_or_default();
    let without_silent = run_vm(SILENT_UNIT_NO_PROFILE, &dir)
        .expect("no-profile run must succeed")
        .unwrap_or_default();
    assert_eq!(
        with_silent, without_silent,
        "silent unit: dispatch must be profile-independent"
    );

    let with_args = run_vm(ARGS_LOST_UNIT, &dir)
        .expect("with-profile run must succeed")
        .unwrap_or_default();
    let without_args = run_vm(ARGS_LOST_UNIT_NO_PROFILE, &dir)
        .expect("no-profile run must succeed")
        .unwrap_or_default();
    assert_eq!(
        with_args, without_args,
        "args-lost unit: dispatch must be profile-independent"
    );

    assert!(without_silent.contains("TARGET"));
    assert!(without_args.contains("probe-ok") && without_args.contains("b1=true"));
    cleanup(&dir);
}

// ── The flow-step lane: invoke_step resolves the shadow LAST-WINS ───────

/// A flow step calling a shadowed name executes the LAST declaration (the
/// stub), matching the TW registry — pre-fix the first-match scan ran the
/// SHADOWED imported body.
#[test]
fn n829_flow_step_shadow_matches_tw_last_wins() {
    let src = r#"
import fixtures829/mod

pattern Shim(a: String) -> String {
  return "local"
}
flow Main {
  input: String = "go" -> Shim -> output
}
"#;
    let dir = write_fixture("flowstep", SHADOW_MODULE);
    let vm_out = run_vm(src, &dir)
        .expect("VM flow run must succeed")
        .unwrap_or_default();
    assert!(
        vm_out.contains("local"),
        "the flow step must execute the last (shadowing) declaration, got: {vm_out}"
    );
    let tw_out = run_tw(src, &dir)
        .expect("TW flow run must succeed")
        .unwrap_or_default();
    assert_eq!(
        vm_out, tw_out,
        "VM and TW must agree on the shadow resolution"
    );
    cleanup(&dir);
}

// ── Negative control (b): the TW lane is unchanged on the same units ────

/// The reference oracle gives the same answers it always gave (the diff
/// never touches the interpreter) — and VM == TW on every fixture.
#[test]
fn n829_tw_lane_unchanged_vm_tw_agree() {
    let dir = write_fixture("twctrl", SHADOW_MODULE);

    let tw_silent = run_tw(SILENT_UNIT, &dir)
        .expect("TW silent-unit run must succeed")
        .unwrap_or_default();
    assert!(tw_silent.contains("TARGET"), "TW: {tw_silent}");
    let vm_silent = run_vm(SILENT_UNIT, &dir)
        .expect("VM silent-unit run must succeed")
        .unwrap_or_default();
    assert_eq!(
        vm_silent, tw_silent,
        "silent unit: VM must match the TW oracle"
    );

    let tw_args = run_tw(ARGS_LOST_UNIT, &dir)
        .expect("TW args-lost-unit run must succeed")
        .unwrap_or_default();
    assert!(
        tw_args.contains("probe-ok") && tw_args.contains("b1=true"),
        "TW: {tw_args}"
    );
    let vm_args = run_vm(ARGS_LOST_UNIT, &dir)
        .expect("VM args-lost-unit run must succeed")
        .unwrap_or_default();
    assert_eq!(
        vm_args, tw_args,
        "args-lost unit: VM must match the TW oracle"
    );

    cleanup(&dir);
}

// ── The map() lane: the same last-wins name resolution ──────────────────

/// `map(list, "Shim")` applies the LAST declaration's body — the first-
/// match scan executed the shadowed imported body (pre-fix).
#[test]
fn n829_map_resolves_the_shadow_last_wins() {
    let src = r#"
import fixtures829/mod
import std/collections

pattern Shim(a: String) -> String {
  return "local:" + a
}
flow Main {
  input: String = "w" -> Shim -> output
}
"#;
    // map() runs inside a pattern body (the collections builtin needs the
    // import at compile time; the flow drives the call).
    let src_with_map = r#"
import fixtures829/mod
import std/collections

pattern Shim(a: String) -> String {
  return "local:" + a
}
pattern Mapper(input: String) -> String {
  let rows = map(["1", "2"], "Shim")
  return to_string(rows)
}
flow Main {
  input: String = "" -> Mapper -> output
}
"#;
    let dir = write_fixture("maplane", SHADOW_MODULE);
    copy_std_collections(&dir);
    let vm_out = run_vm(src_with_map, &dir)
        .expect("VM map run must succeed")
        .unwrap_or_default();
    assert!(
        vm_out.contains("local:1") && vm_out.contains("local:2"),
        "map() must apply the shadowing (last) body, got: {vm_out}"
    );
    let _ = src; // the bare-shadow unit is covered by the flow-step test
    let tw_out = run_tw(src_with_map, &dir)
        .expect("TW map run must succeed")
        .unwrap_or_default();
    assert_eq!(vm_out, tw_out, "map(): VM must match the TW oracle");
    cleanup(&dir);
}
