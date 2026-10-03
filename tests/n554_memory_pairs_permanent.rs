// ── tests/n554_memory_pairs_permanent.rs ─────────────────────────────
// №554 (Wave 25 P1; the audit 02.10 §3.3 — position Е on the memory
// pairs): the memory pairs are PERMANENT BY DESIGN — the honest
// record, not a silent acceptance. This file is the consistency pin:
// the three artifacts that carry the record cannot drift apart.
//
//   1. docs/limitations.md — the TW/VM Divergences table carries the
//      row ("the memory builtins run on TWO ENGINES ... | №442 |
//      PERMANENT BY DESIGN") with live links;
//   2. src/memory_ops.rs — the module docstring cross-links the
//      limitations record and states the no-merge rule;
//   3. scripts/ci/ops_pair_baseline.txt — the five memory pairs are
//      marked documented-permanent, the threshold stays 6.
//
// The audit §5.3 rule this pins: the memory pairs are NEVER merged at
// the cost of semantics. If any of the three artifacts loses the
// record (or the record and the code disagree), this file fails.

const LIMITATIONS: &str = include_str!("../docs/limitations.md");
const MEMORY_OPS: &str = include_str!("../src/memory_ops.rs");
const BASELINE: &str = include_str!("../scripts/ci/ops_pair_baseline.txt");

#[test]
fn limitations_carries_the_permanent_by_design_row() {
    // The row exists in the TW/VM Divergences table...
    assert!(
        LIMITATIONS.contains("The memory builtins run on TWO ENGINES"),
        "limitations.md must carry the memory-pairs row in the TW/VM \
         Divergences table — the honest record is the deliverable"
    );
    // ...with the audit's own status verdict...
    assert!(
        LIMITATIONS.contains("PERMANENT BY DESIGN"),
        "the row's status must read PERMANENT BY DESIGN (the audit \
         position Е — documentation instead of a semantics-costly merge)"
    );
    // ...and names the №442 source the audit cited.
    assert!(
        LIMITATIONS.contains("№442"),
        "the row must cite №442 (the honest simple-memory twin posture)"
    );
}

#[test]
fn memory_ops_docstring_cross_links_the_record() {
    assert!(
        MEMORY_OPS.contains("docs/limitations.md"),
        "the memory_ops module docstring must cross-link the limitations \
         record (the ↔ direction the naryad requires)"
    );
    assert!(
        MEMORY_OPS.contains("PERMANENT BY DESIGN"),
        "the memory_ops docstring must state the permanent-by-design \
         posture"
    );
    // The shared-HOME posture stays: the module is not a unification.
    assert!(
        MEMORY_OPS.contains("the shared HOME, not a unification"),
        "the module must keep the №442 shared-HOME-not-unification \
         statement — the docstring and the record must not contradict"
    );
}

#[test]
fn baseline_marks_the_five_memory_pairs_documented_permanent() {
    for pair in [
        "dispatch (memory_ops.rs)",
        "forget (memory_ops.rs)",
        "memorize (memory_ops.rs)",
        "recall (memory_ops.rs)",
        "recall_top_k (memory_ops.rs)",
    ] {
        let marked = BASELINE
            .lines()
            .any(|l| l.starts_with(pair) && l.contains("[permanent"));
        assert!(
            marked,
            "the baseline must mark '{pair}' as documented-permanent \
             (the №554 format: a [permanent — №554] suffix)"
        );
    }
    // The db pair that stays is NOT marked permanent (it is a
    // candidate, not a design decision).
    for line in BASELINE.lines() {
        if line.starts_with("query (db_ops.rs)") {
            assert!(
                !line.contains("[permanent"),
                "the query pair must stay UNMARKED — it is a parity-fix \
                 candidate (the §3.3 position А), never a permanent-by-\
                 design decision"
            );
        }
    }
    // The threshold fact: 6, and the №554 entry is present.
    assert!(
        BASELINE.contains("DOCUMENTED-PERMANENT BY DESIGN"),
        "the baseline must carry the №554 history entry"
    );
}
