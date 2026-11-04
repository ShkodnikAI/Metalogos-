// Naryad №897 (issue #ISSUE_NUM, patch 0.38.2 — [process][bugfix]): the
// run/serve gate №523 applies the №802 allow-comment suppression PER
// MODULE — each finding is sanctioned only by its own module's source
// text.
//
// The bug (main 38430258, v0.38.1): enforce_run_gates checked the MERGED
// surface (entry ++ statically-resolved imports) but ran
// filter_suppressed_unchecked against the ENTRY source text only — an
// allow-comment sitting in an imported module was invisible to the gate,
// so a program that is clean under `mlog check` refused at `mlog run`
// with "Compilation error (Naryad #523)". lib.rs's own №802 note states
// the intent: "the checked surface and the run gate must agree — a
// suppressed-at-check program must not refuse at run."
//
// The fix: the unchanged №802 filter (the per-line quota, the
// non-empty-reason rule) runs per module source; an UNCHECKED finding
// survives iff its module's own filter kept it — one module's
// allow-comment cannot launder another module's access.
//
// RED-before (the №772 form): T1/T3/T4/T5/T6 refused on main @ 38430258
// with "[UNCHECKED_RESULT] … Compilation error (Naryad #523)" — the
// module's own sanctioned comment never matched (the same code inlined
// in the entry ran).

#![allow(clippy::disallowed_methods)] // the №199 form: fixtures write the REAL filesystem by design

use std::path::Path;

/// The sanctioned module: the try result's `.value` is read on the SAME
/// line as its allow-comment (the trailing form). This is the field
/// shape of the fosved-acc guards (the redact("hash_only") barrier),
/// reduced to the №802 specimen (`try trim`) so the fixture needs no
/// live I/O — the gate walks the body statically, before any execution.
const DEP_SANCTIONED: &str = "\
pattern ReadSanctioned(_input: String) -> String {
  let r = try trim(\"hello\")
  return r.value // allow(unchecked_result): the sanctioned read — the redact(\"hash_only\") barrier sits downstream (naryad #802 form)
}
";

/// The same module WITHOUT the allow-comment — the negative arm of the
/// regression pair (the suppression must not appear from nowhere).
const DEP_UNSANCTIONED: &str = "\
pattern ReadSanctioned(_input: String) -> String {
  let r = try trim(\"hello\")
  return r.value
}
";

/// The dedicated-comment form (the comment on the line ABOVE the
/// access) inside an imported module.
const DEP_DEDICATED: &str = "\
pattern ReadDedicated(_input: String) -> String {
  let r = try trim(\"hello\")
  // allow(unchecked_result): the dedicated-comment form sanctions the NEXT line only
  return r.value
}
";

/// The unsanctioned SIBLING for the laundering pin — a DIFFERENT pattern
/// name, so the two modules' declarations never collide in the merged
/// surface (the collision dedup would drop the second landing and mute
/// the pin).
const DEP_DIRTY: &str = "\
pattern ReadDirty(_input: String) -> String {
  let r = try trim(\"hello\")
  return r.value
}
";

fn write_module(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).expect("fixture module write");
}

fn run_in_dir(main_source: &str, dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(main_source, dir.to_path_buf())
}

/// The entry imports the module and CALLS its pattern through the flow —
/// the repro shape (the finding fires from the MODULE's body at the
/// static gate, before any execution).
fn main_calling(module: &str, stage: &str) -> String {
    format!("import {module}\nflow Main {{ input: String = \"x\" -> {stage} -> output }}\n")
}

// ── T1: the regression pair, positive arm — runs ────────────────────

/// T1: an imported module with a sanctioned read → the run gate passes
/// and the program executes (RED-before: "Compilation error (Naryad
/// #523)" — the module's comment never matched the entry-only source).
#[test]
fn n897_imported_module_allow_comment_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_module(dir.path(), "dep.mlog", DEP_SANCTIONED);
    let main = main_calling("dep", "ReadSanctioned");
    let run = run_in_dir(&main, dir.path());
    match run {
        Ok(output) => {
            let text = output.unwrap_or_default();
            assert!(
                text.contains("hello"),
                "the sanctioned module must run to its output, got: {:?}",
                text
            );
        }
        Err(msg) => panic!(
            "the module's own allow-comment must sanction the run path, refused with: {}",
            msg
        ),
    }
}

// ── T2: the regression pair, negative arm — blocks ──────────────────

/// T2: the same module WITHOUT the comment → the run gate still refuses
/// with the stable code (the suppression is not granted silently).
#[test]
fn n897_imported_module_without_comment_blocks() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_module(dir.path(), "dep.mlog", DEP_UNSANCTIONED);
    let main = main_calling("dep", "ReadSanctioned");
    let run = run_in_dir(&main, dir.path());
    match run {
        Ok(_) => panic!("the unsanctioned module read must block the run path"),
        Err(msg) => {
            assert!(
                msg.contains("UNCHECKED_RESULT"),
                "the refusal carries the stable code, got: {}",
                msg
            );
            assert!(
                msg.contains("Naryad #523"),
                "the refusal is the №523 gate's, got: {}",
                msg
            );
        }
    }
}

// ── T3: cross-module laundering stays closed ────────────────────────

/// T3: one module's allow-comment must NOT launder another module's
/// identical access — the sanctioned module runs clean, the unsanctioned
/// sibling still blocks the program.
#[test]
fn n897_one_module_comment_cannot_launder_another() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_module(dir.path(), "dep_ok.mlog", DEP_SANCTIONED);
    write_module(dir.path(), "dep_dirty.mlog", DEP_DIRTY);
    let main =
        "import dep_ok\nimport dep_dirty\nflow Main { input: String = \"x\" -> ReadDirty -> output }\n".to_string();
    let run = run_in_dir(&main, dir.path());
    match run {
        Ok(_) => panic!(
            "the dirty module's finding must survive — the ok module's comment is not its own"
        ),
        Err(msg) => assert!(
            msg.contains("UNCHECKED_RESULT"),
            "the refusal carries the stable code, got: {}",
            msg
        ),
    }
}

// ── T4: the check ↔ run agreement (the lib.rs №802 intent) ──────────

/// T4: the same module text is clean under `check` (the №802 surface)
/// AND runs when imported (the №523 surface) — the two surfaces agree.
#[test]
fn n897_check_and_run_surfaces_agree() {
    let check = metalogos::check_program(DEP_SANCTIONED).expect("the module must parse");
    assert!(
        !check.errors.iter().any(|e| e.message.contains("UNCHECKED")),
        "the sanctioned module is clean at check, got: {:?}",
        check.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    let dir = tempfile::tempdir().expect("tempdir");
    write_module(dir.path(), "dep.mlog", DEP_SANCTIONED);
    let main = main_calling("dep", "ReadSanctioned");
    let run = run_in_dir(&main, dir.path());
    assert!(
        run.is_ok(),
        "the checked-clean module must not refuse at run, got: {:?}",
        run.err()
    );
}

// ── T5: the dedicated-comment form in an imported module ────────────

/// T5: the dedicated-comment form (the comment above the access) in an
/// imported module sanctions the NEXT line only — the module runs.
#[test]
fn n897_dedicated_comment_form_in_module_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_module(dir.path(), "dep.mlog", DEP_DEDICATED);
    let main = main_calling("dep", "ReadDedicated");
    let run = run_in_dir(&main, dir.path());
    assert!(
        run.is_ok(),
        "the dedicated-comment form must sanction the next line, got: {:?}",
        run.err()
    );
}

// ── T6: the per-line quota is unchanged inside a module ─────────────

/// T6: ONE allow-comment sanctions ONE access line — a second unguarded
/// access line in the same module still blocks (the №802 per-line quota,
/// now enforced per module source).
#[test]
fn n897_module_quota_one_comment_one_access_line() {
    let dep = "\
pattern ReadTwice(_input: String) -> String {
  let r = try trim(\"hello\")
  let a = r.value // allow(unchecked_result): the first read only
  let b = r.value
  return a
}
";
    let dir = tempfile::tempdir().expect("tempdir");
    write_module(dir.path(), "dep.mlog", dep);
    let main = main_calling("dep", "ReadTwice");
    let run = run_in_dir(&main, dir.path());
    match run {
        Ok(_) => panic!("one comment cannot launder two access lines (the №802 quota)"),
        Err(msg) => assert!(
            msg.contains("UNCHECKED_RESULT"),
            "the second access still refuses with the stable code, got: {}",
            msg
        ),
    }
}
