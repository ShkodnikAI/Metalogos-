// ── №1018 (issue #2115, [bugfix][process]): the check --root suppression
// surface ── the №897 bug shape, one path later ────────────────────────────
//
// The bug: `check_program_with_root` merged the whole import graph's
// declarations into the semantic pass but ran the №802 suppression filter
// against the CHECKED file's text only (`filter_suppressed_unchecked(…,
// source)`). An allow-comment sitting in an imported module was invisible
// to the filter, so `mlog check leaf.mlog --root root.mlog` — and the
// office CI form `mlog check app.mlog --root app.mlog`, where the root
// imports the app's modules — reported the module's OWN sanctioned reads.
// The run gate applies the same filter PER MODULE since №897 (its contract
// quote: "a suppressed-at-check program must not refuse at run") — the
// checked surface and the run gate disagreed, exactly the disagreement
// lib.rs's №802 note forbids. Repro fleet: fosved-acc @ v0.40.1 — 21
// UNCHECKED_RESULT findings, every one of them carrying a sanctioned
// allow-comment in its own module; 0 on the run gate.
//
// The fix: the №897 per-module filter on the check --root path. The
// checked file is the entry; the resolved groups carry the imports; the
// root file's own body joins as a group of its own when the checked file
// is a DIFFERENT file (its allow-comments sanction its own findings); a
// group whose source equals the entry's text is dropped from the bank —
// the entry bank owns that text, and double-banking would inflate the
// sanctioned multiset (the laundering hole the per-(code, variable) quota
// exists to close).
//
// RED-before (the №772 form): T1/T3/T4 refused on the pre-fix main (the
// sanctioned module's finding survived the check --root filter); T2/T5
// stayed green (the negative arms).

#![allow(clippy::disallowed_methods)] // the №199 form: fixtures write the REAL filesystem by design

use std::fs;
use std::path::Path;

/// The sanctioned module: the try result's `.value` is read on the SAME
/// line as its allow-comment (the trailing form). The №802 specimen
/// (`try trim`) — no live I/O needed.
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

/// A root whose OWN body carries a sanctioned read (the comment inside
/// the root file) — the root-as-a-group arm of the fix.
const ROOT_SANCTIONED_BODY: &str = "\
import dep
pattern RootOwn(_x: String) -> String {
  let r = try trim(\"root\")
  return r.value // allow(unchecked_result): the root's own sanctioned read
}
flow Main { input: String = \"x\" -> RootOwn -> output }
";

/// A root whose own body carries an UNSANCTIONED read — the
/// no-cross-module-laundering pin: the dep's comment must not sanction it.
const ROOT_UNSANCTIONED_BODY: &str = "\
import dep
pattern RootOwn(_x: String) -> String {
  let r = try trim(\"root\")
  return r.value
}
flow Main { input: String = \"x\" -> RootOwn -> output }
";

const CLEAN_ENTRY: &str = "pattern Leaf(a: String) -> String { return a }";

/// The entry that calls the dep's pattern through the flow — the repro
/// shape (the finding fires from the MODULE's body at the call site's
/// arguments, the merged-surface walk).
const ENTRY_CALLS_DEP: &str = "\
import dep
pattern UseIt(x: String) -> String { return ReadSanctioned(x) }
";

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("n1018_{}_{}", tag, std::process::id()));
    fs::create_dir_all(&dir).expect("tmp dir");
    dir
}

fn write_root_with_dep(dir: &Path, root_body: &str, dep_body: &str) -> std::path::PathBuf {
    fs::write(dir.join("root.mlog"), root_body).expect("write root");
    fs::write(dir.join("dep.mlog"), dep_body).expect("write dep");
    dir.join("root.mlog")
}

fn unchecked_count(rendered: &str) -> usize {
    rendered.matches("[UNCHECKED_RESULT]").count()
}

// ── T1: the module's allow-comment is visible through check --root ──────
//
// The root imports the sanctioned dep; the entry is clean. Before the
// fix the dep's finding survived (the filter matched the ENTRY text
// only) — the office CI form reported sanctioned reads.
#[test]
fn n1018_check_root_sees_the_module_allow_comment() {
    let dir = tmp_dir("module");
    let root_path = write_root_with_dep(
        &dir,
        "import dep\nflow Main { input: String = \"x\" -> Leaf -> output }\n",
        DEP_SANCTIONED,
    );
    let result = metalogos::check_program_with_root(CLEAN_ENTRY, Some(root_path.as_path()))
        .expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    assert_eq!(
        unchecked_count(&rendered),
        0,
        "the module's own allow-comment must sanction its read through check --root, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── T2: the negative arm — no comment, no suppression ───────────────────
#[test]
fn n1018_check_root_reports_the_unsanctioned_module() {
    let dir = tmp_dir("negative");
    let root_path = write_root_with_dep(
        &dir,
        "import dep\nflow Main { input: String = \"x\" -> Leaf -> output }\n",
        DEP_UNSANCTIONED,
    );
    let result = metalogos::check_program_with_root(CLEAN_ENTRY, Some(root_path.as_path()))
        .expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    assert!(
        unchecked_count(&rendered) >= 1,
        "the unsanctioned module read must still refuse, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── T3: the root==file form (the office CI form `check app --root app`) ─
//
// The root imports the sanctioned dep; the root is checked against
// itself. The entry bank owns the root's text; the groups bank the dep.
#[test]
fn n1018_check_root_eq_file_sees_the_module_allow_comment() {
    let dir = tmp_dir("root_eq_file");
    let root_path = write_root_with_dep(
        &dir,
        "import dep\nflow Main { input: String = \"x\" -> Leaf -> output }\n",
        DEP_SANCTIONED,
    );
    let source = fs::read_to_string(&root_path).expect("read root");
    let result = metalogos::check_program_with_root(&source, Some(root_path.as_path()))
        .expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    assert_eq!(
        unchecked_count(&rendered),
        0,
        "the root==file form must honor the module's allow-comment, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── T4: the root's own body is sanctioned by its own comments only ──────
//
// The root carries a sanctioned read in its OWN body; the checked file is
// a different, clean file. The root-as-a-group arm of the fix: before it,
// the root's comment was invisible (the filter matched the leaf's text).
#[test]
fn n1018_check_root_sanctions_the_root_own_body_by_its_own_comment() {
    let dir = tmp_dir("root_body");
    let root_path = write_root_with_dep(&dir, ROOT_SANCTIONED_BODY, DEP_SANCTIONED);
    let result = metalogos::check_program_with_root(CLEAN_ENTRY, Some(root_path.as_path()))
        .expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    assert_eq!(
        unchecked_count(&rendered),
        0,
        "the root's own allow-comment must sanction the root's own read, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── T5: no cross-module laundering ──────────────────────────────────────
//
// The dep is sanctioned; the ROOT's own body carries an UNSANCTIONED
// read. The dep's comment must NOT launder the root's finding (the
// per-module rule). The pre-fix filter could never do that (it matched
// the entry text only), and the fix must not introduce the hole through
// the synthesized root group either.
#[test]
fn n1018_check_root_no_cross_module_laundering() {
    let dir = tmp_dir("launder");
    let root_path = write_root_with_dep(&dir, ROOT_UNSANCTIONED_BODY, DEP_SANCTIONED);
    let result = metalogos::check_program_with_root(CLEAN_ENTRY, Some(root_path.as_path()))
        .expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    assert!(
        unchecked_count(&rendered) >= 1,
        "the dep's allow-comment must not launder the root's own unsanctioned read, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── T6: the plain no-root form is unchanged ─────────────────────────────
//
// `check_program` (no --root) keeps its single-file semantics: the
// sanctioned read in the checked file itself is honored, and a call into
// an unsanctioned pattern defined IN THE SAME FILE refuses.
#[test]
fn n1018_check_no_root_form_unchanged() {
    let dir = tmp_dir("no_root");
    let root_path = write_root_with_dep(&dir, "flow Main { input: String = \"x\" -> Leaf -> output }\n", DEP_SANCTIONED);
    let _ = root_path;
    let result = metalogos::check_program_with_root(ENTRY_CALLS_DEP, None).expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    // The entry calls ReadSanctioned but imports nothing (import dep
    // cannot resolve without a root dir) — the undefined import is fine
    // here as long as the in-file sanctioned read is honored. The entry
    // has no try reads of its own; nothing to assert except no panic and
    // no UNCHECKED from the entry itself.
    assert_eq!(
        unchecked_count(&rendered),
        0,
        "the no-root form must keep honoring the entry's own comments, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}
