// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file reads repo source files (the
// №904 roster and the ratchet baseline) from the working tree by design —
// the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── №905 (issue #1799, wave 81 — W-J, [testing][ci]): the taint-triples
//    coverage ratchet — the floor follows the fact ─────────────────────
//
// The wave-81 coverage rule: taint saturation, once reached, cannot roll
// back. The №904 taint lane made the taint surface (3 lanes — secret/sql/
// html; source×sink carriers) visible to the differential harness; this
// ratchet pins the achieved (lane, source, sink) footprint ONLY-UP.
//
// The form is the №766 nesting-pairs baseline
// (scripts/ci/fuzz_nesting_pairs_baseline.txt) × the №621 LIVENESS lesson
// («the sweep was vacuously green 0/150» — a lane must carry a lower
// guard) × the known_divergences ratchet №479:
//   - the floor lives in scripts/ci/fuzz_taint_triples_baseline.txt;
//   - a PR that RAISES the achieved saturation MUST bump the file in the
//     SAME PR (a higher count than the floor is a FAILURE demanding the
//     bump — the №776 discipline; a lower count is a regression);
//   - the roster count is read FROM THE GENERATOR'S CODE (the №904 roster
//     markers in tests/naryad_465_diff_fuzzer.rs), NOT from a run — the
//     sandbox-independence the naryad demands: the machine count does not
//     depend on any backend being runnable;
//   - an EMPTY roster is RED (the LIVENESS guard: the lane cannot go
//     silent — the №621 lesson);
//   - a missing/unreadable baseline or an unparsable roster row is RED
//     (fail-closed: silence is never a pass).
//
// The report prints the full triple roster (the last_report form) — the
// human-readable «what is saturated» ledger.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const ROSTER_REL: &str = "tests/naryad_465_diff_fuzzer.rs";
const BASELINE_REL: &str = "scripts/ci/fuzz_taint_triples_baseline.txt";
const MARKER_BEGIN: &str = "№904 taint roster BEGIN";
const MARKER_END: &str = "№904 taint roster END";

/// One roster row: (id, lane, source, sink, expect).
type Row = (String, String, String, String, String);

/// Parse the №904 roster OUT OF THE GENERATOR'S SOURCE between the BEGIN
/// and END markers. The scan is TUPLE-based, not line-based: rustfmt may
/// break a long row across lines, and the count must survive a pure
/// re-format (a formatter is not a saturation change). Fail-closed: a
/// lost marker or a block without exactly 5 string fields panics — the
/// count cannot silently shrink or grow.
fn parse_roster(repo: &Path) -> Vec<Row> {
    let path = repo.join(ROSTER_REL);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("the roster source must exist ({path:?}): {e}"));
    let begin = src
        .find(MARKER_BEGIN)
        .unwrap_or_else(|| panic!("the roster marker BEGIN ({MARKER_BEGIN:?}) is lost from {ROSTER_REL} — the №906 machine source is unparsable"));
    let end = src[begin..].find(MARKER_END).unwrap_or_else(|| {
        panic!("the roster marker END ({MARKER_END:?}) is lost from {ROSTER_REL}")
    });
    let section = &src[begin..begin + end];
    let bytes = section.as_bytes();

    let mut rows: Vec<Row> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        // A tuple-block starts with "(" followed (after any whitespace —
        // rustfmt may put the first field on the next line) by a quote
        // (every roster row begins with the scenario id string).
        let mut first = i + 1;
        while first < bytes.len() && bytes[first].is_ascii_whitespace() {
            first += 1;
        }
        if bytes[i] == b'(' && first < bytes.len() && bytes[first] == b'"' {
            // Collect until the matching ')' at depth 0 (roster fields are
            // flat strings — no nested parens by the roster contract).
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            let block = &section[i + 1..j.saturating_sub(1)];
            // Every quoted segment inside the block is a field (comments
            // rustfmt may keep inside are skipped — only "..." counts).
            let mut fields: Vec<&str> = Vec::new();
            let mut k = 0usize;
            let bb = block.as_bytes();
            while k < bb.len() {
                if bb[k] == b'"' {
                    let close = block[k + 1..].find('"').unwrap_or_else(|| {
                        panic!("unterminated string in a roster block: {block:?}")
                    });
                    fields.push(&block[k + 1..k + 1 + close]);
                    k += close + 2;
                } else {
                    k += 1;
                }
            }
            if fields.len() != 5 {
                panic!(
                    "a roster row must carry exactly 5 fields (id, lane, source, \
                     sink, expect), got {} in {ROSTER_REL}: {block:?}",
                    fields.len()
                );
            }
            rows.push((
                fields[0].to_string(),
                fields[1].to_string(),
                fields[2].to_string(),
                fields[3].to_string(),
                fields[4].to_string(),
            ));
            i = j;
        } else {
            i += 1;
        }
    }
    rows
}

/// №905: the taint-triples ratchet — the floor follows the fact.
#[test]
fn n905_taint_triples_floor() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // ── the baseline: fail-closed read (missing/unreadable/non-integer
    //    is RED — the №766 form) ────────────────────────────────────────
    let baseline_path = repo.join(BASELINE_REL);
    let floor: usize = std::fs::read_to_string(&baseline_path)
        .unwrap_or_else(|e| panic!("the baseline file must exist ({baseline_path:?}): {e}"))
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("the baseline must be an integer: {e}"));

    // ── the roster: parsed from the generator's code (sandbox-independent) ──
    let rows = parse_roster(&repo);

    // ── the LIVENESS guard (the №621 lesson): an empty roster means the
    //    taint lane went SILENT — red, never a vacuous pass ─────────────
    assert!(
        !rows.is_empty(),
        "the №904 taint roster is EMPTY — the taint lane went silent (the \
         №621 vacuous-sweep lesson); the saturation ratchet cannot stand on \
         zero scenarios. Restore the roster, never lower the floor to hide it."
    );

    // ── the saturation footprint: the DISTINCT (lane, source, sink)
    //    triples the roster exercises (the №904 taint_triples_run form) ──
    let mut triples: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (_, lane, source, sink, _) in &rows {
        triples.insert((lane.clone(), source.clone(), sink.clone()));
    }
    let triples = triples.len();

    // ── the report: the full roster printed (the last_report form — the
    //    human-readable «what is saturated» ledger) ──────────────────────
    for (id, lane, source, sink, expect) in &rows {
        println!("taint triple: {lane}/{source} -> {sink} [{id}] expect={expect}");
    }
    println!(
        "№905 taint ratchet: {n} roster rows, {t} distinct (lane/source/sink) \
         triples, floor={floor}",
        n = rows.len(),
        t = triples,
    );

    // ── the only-up pair (the №766 form): a drop is a regression; a rise
    //    without the same-PR baseline bump is a failure demanding the bump
    //    (the №776 discipline: «пол follows the fact», никогда наоборот) ──
    assert!(
        triples >= floor,
        "the taint-triples saturation dropped to {triples} (the floor {floor}) — \
         the №904 roster lost a lane/source/sink combination (the wave-81 \
         coverage rule: saturation cannot roll back); repair the roster, \
         never lower the floor",
    );
    assert!(
        triples <= floor,
        "the taint-triples saturation rose to {triples} (the floor {floor}) — \
         bump {BASELINE_REL} to {triples} in this PR (the only-up ratchet, \
         the №766/№757/№613 form)",
    );
}
