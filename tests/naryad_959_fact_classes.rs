// ── №959 (issue #1926): the fact CLASSES and the slot SURFACES ─────────
//
// The fact the audit of 12.11 left behind: the Phase-8 machine facts count
// the PRESENCE OF AN ANCHOR — a decision name in a test file, `pub fn
// replay_gate` in a module, eight test-function names, twelve headings, a
// seed region. Those are real things to check and the gates that read them
// are honest gates. But an anchor is not a BEHAVIOUR, and a tree can carry
// every anchor of a capability no user can invoke: read as readiness,
// `TERMINAL_STOP 13/13` says more than the tree supports.
//
// So every fact declares its class and every slot its surface, in
// `scripts/ci/slot_class_roster.txt`, and this file is the driver that
// keeps the roster honest from the Rust side — the №551 form (the CI lane
// runs the python gate; the test lane runs it too, so the class cannot
// drift between CI runs).
//
// THE MUTATION THIS FILE PINS (the issue's «сделано, когда»): declare a
// structural fact behavioural WITHOUT an executable test and
// `gen_metrics.py --check` must go RED. `n959_behavioural_without_exec_is_red`
// performs exactly that mutation on a COPY of the roster and asserts the
// refusal; `scripts/mutation_verify_959.sh` does the same against the live
// tree.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    let m = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    Path::new(&m).to_path_buf()
}

/// Run a short python snippet against the live `gen_metrics` module and
/// return its stdout. The gate itself is python (the generator's home);
/// this test is the driver, not a re-implementation (the №924 rule: a
/// substituted checker never counts).
fn gen_metrics_eval(snippet: &str) -> String {
    let root = repo_root();
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(format!(
            "import sys; sys.path.insert(0, 'scripts')\nimport gen_metrics as g\n{snippet}"
        ))
        .current_dir(&root)
        .output()
        .expect("python3 runs");
    assert!(
        out.status.success(),
        "the gen_metrics snippet failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The roster parses, and it is IN SYNC with the live tree in both
/// directions (a dead evidence needle, a behavioural claim with no live
/// executed test, a kernel-only row whose module gained a surface).
#[test]
fn n959_the_slot_class_roster_is_in_sync_with_the_tree() {
    let problems = gen_metrics_eval("print(len(g.slot_class_problems()))");
    assert_eq!(
        problems, "0",
        "the fact-class roster diverged from the tree — run \
         `python3 -I scripts/gen_metrics.py --check` for the named rows"
    );
}

/// The three counts are PRINTED, and the structural one still covers all
/// 13 slots (the anchors are real; the only-up floor over them stands).
/// The behavioural and user-surface counts are the honest additions, and
/// they are deliberately allowed to be low — a low number that is true is
/// the point of this naryad.
#[test]
fn n959_the_three_counts_are_live_and_the_structural_one_is_complete() {
    let counts = gen_metrics_eval(
        "c = g.slot_class_counts(); \
         print(c['slots'], c['structural'], c['behavioural'], c['user_surface'])",
    );
    let nums: Vec<usize> = counts
        .split_whitespace()
        .map(|s| s.parse().expect("a number"))
        .collect();
    assert_eq!(nums.len(), 4, "expected four counts, got {counts:?}");
    let (slots, structural, behavioural, surface) = (nums[0], nums[1], nums[2], nums[3]);
    assert_eq!(slots, 13, "the slot set is the ADR-0221 thirteen");
    assert_eq!(
        structural, 13,
        "every slot's fact must still count a live anchor — a dropped \
         anchor is a gate failure, not a class change"
    );
    assert!(
        behavioural >= 1,
        "at least one fact must be behavioural (C9 TAINT_PROGRAMS_PER_RUN, \
         naryad №956) — a tree with zero behavioural facts has no executed \
         number anywhere, which is the state this naryad exists to name"
    );
    assert!(
        behavioural <= slots && surface <= slots,
        "a count cannot exceed the slot set"
    );
    // The honest direction of travel: both numbers may only be raised by
    // WIRING something, never by relabelling. The roster check above is
    // what enforces that; this assertion records the intent.
    assert!(
        behavioural < slots,
        "if every slot is behavioural-GREEN, the ADR-0221 §2.3a amendment \
         is satisfied — update this test and the 1.0 gate together, with \
         the owner's acceptance, rather than letting it pass silently"
    );
}

/// THE MUTATION. A structural fact relabelled `behavioural` with no
/// `exec=` must be REFUSED. The mutation runs on a temp copy of the
/// roster — the live tree is never touched.
#[test]
fn n959_behavioural_without_exec_is_red() {
    let roster = repo_root()
        .join("scripts")
        .join("ci")
        .join("slot_class_roster.txt");
    let text = std::fs::read_to_string(&roster).expect("the roster is readable");
    // Relabel C8/REPLAY_MATCH — a purely structural fact, kernel-only, with
    // no executed test anywhere near it.
    let mutated = text.replace(
        "slot C8 fact=REPLAY_MATCH class=structural",
        "slot C8 fact=REPLAY_MATCH class=behavioural",
    );
    assert_ne!(
        mutated, text,
        "the C8 row moved — repair this mutation target, not the gate"
    );
    let out = gen_metrics_eval(&format!(
        "rows = g._slot_class_rows({mutated:?})\n\
         probs = g.slot_class_problems(rows)\n\
         print('PROBLEMS', len(probs))\n\
         print('\\n'.join(probs))\n\
         c = g.slot_class_counts(rows)\n\
         print('BEHAVIOURAL', c['behavioural'])"
    ));
    assert!(
        out.contains("PROBLEMS 1"),
        "a behavioural claim with no executable test must be exactly one \
         named problem; got:\n{out}"
    );
    assert!(
        out.contains("class=behavioural with NO exec="),
        "the refusal must NAME the missing executed test — a gate that \
         fails without saying why teaches nothing; got:\n{out}"
    );
    assert!(
        out.contains("BEHAVIOURAL 0"),
        "fail-closed: while the roster disagrees with the tree the \
         behavioural count must read 0, never the relabelled number; \
         got:\n{out}"
    );
}

/// The other direction of the same honesty: a `kernel-only` row whose
/// module HAS a user surface is also a refusal. Relabelling C1 (a real
/// builtin surface) as kernel-only must be caught — an understated row
/// hides a capability as surely as an overstated one overclaims it.
#[test]
fn n959_an_understated_surface_is_red_too() {
    let roster = repo_root()
        .join("scripts")
        .join("ci")
        .join("slot_class_roster.txt");
    let text = std::fs::read_to_string(&roster).expect("the roster is readable");
    let mutated = text.replace(
        "slot C1 fact=ATTEST_MAP class=structural surface=builtin",
        "slot C1 fact=ATTEST_MAP class=structural surface=kernel-only",
    );
    assert_ne!(mutated, text, "the C1 row moved — repair the target");
    let out = gen_metrics_eval(&format!(
        "rows = g._slot_class_rows({mutated:?})\n\
         print('\\n'.join(g.slot_class_problems(rows)) or 'NONE')"
    ));
    assert!(
        out.contains("surface=kernel-only but a builtin/CLI file NOW names"),
        "a kernel-only row over a module with a live builtin surface must \
         be refused, and the refusal must say which module; got:\n{out}"
    );
}

/// The renamed fact (the issue's п.2): the live machine fact is
/// `TAINT_GENERATOR_SPACE`, and the behavioural counterpart from №956
/// stands beside it rather than replacing it — a product over a space is
/// still a fact about the generator, it is just not a fact about a run.
#[test]
fn n959_the_generator_space_is_renamed_and_the_run_fact_stands_beside_it() {
    let metrics = std::fs::read_to_string(repo_root().join("docs").join("metrics.md"))
        .expect("metrics.md readable");
    assert!(
        metrics.contains("TAINT_GENERATOR_SPACE"),
        "the renamed fact is missing from the generated table"
    );
    assert!(
        metrics.contains("TAINT_PROGRAMS_PER_RUN"),
        "the behavioural counterpart (№956) must stand beside the space"
    );
    assert!(
        metrics.contains("TERMINAL_STOP by CLASS"),
        "the class split must be in the generated table (the issue's п.1)"
    );
    let plan = std::fs::read_to_string(repo_root().join("docs").join("PLAN-SUMMARY.md"))
        .expect("PLAN-SUMMARY.md readable");
    assert!(
        plan.contains("TERMINAL_STOP by class"),
        "the class split must be in the PLAN-SUMMARY numbers block too"
    );
}

/// The ADR-0221 amendment exists and is **Proposed**, not Accepted: an
/// agent never marks an ADR Accepted (канон §13.14 п.6). This test is the
/// guard against a silent promotion.
#[test]
fn n959_the_adr_amendment_is_proposed_not_accepted() {
    let adr = std::fs::read_to_string(
        repo_root()
            .join("docs")
            .join("adr")
            .join("0221-terminal-stop-1-0.md"),
    )
    .expect("ADR-0221 readable");
    assert!(
        adr.contains("### 2.3a AMENDMENT (naryad №959"),
        "the §2.3a amendment is missing from ADR-0221"
    );
    let amendment = adr
        .split("### 2.3a AMENDMENT")
        .nth(1)
        .expect("the amendment section")
        .split("### 2.4")
        .next()
        .expect("the amendment ends before §2.4");
    assert!(
        amendment.contains("status **Proposed**"),
        "the amendment must stand as Proposed until the owner's explicit \
         acceptance — an agent never promotes an ADR"
    );
    assert!(
        amendment.contains("break attempt"),
        "the amendment must carry the fourth gate item (an external break \
         attempt that found no bypass) — the owner's directive of 12.11"
    );
}

/// The kernel-only marks are in the reader-facing documents (the issue's
/// п.3): the per-slot table in limitations.md and the «what is NOT here»
/// note in REFERENCE.md.
#[test]
fn n959_the_kernel_only_marks_are_in_the_reader_documents() {
    let lim = std::fs::read_to_string(repo_root().join("docs").join("limitations.md"))
        .expect("limitations.md readable");
    assert!(
        lim.contains("kernel → surface"),
        "limitations.md lost the per-slot kernel→surface table"
    );
    for slot in ["C4", "C7", "C8", "C12", "C13"] {
        assert!(
            lim.contains(&format!("| {slot} |")),
            "limitations.md lost the {slot} row"
        );
    }
    let reference =
        std::fs::read_to_string(repo_root().join("REFERENCE.md")).expect("REFERENCE.md readable");
    assert!(
        reference.contains("the kernel-only capabilities"),
        "REFERENCE.md lost the note on what is deliberately NOT in it"
    );
}
