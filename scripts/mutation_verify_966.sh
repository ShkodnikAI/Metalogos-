#!/usr/bin/env bash
# ── Naryad №966 (issue #1954, the B7 line — the owner decision 13–14.11,
#    P1, [security][compiler][adr]) — the mutation verification for the
#    flow output's integrity boundary pin ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the integrity half is gated: the Flow arm of
#        `sink_clearance_violations` (src/semantic.rs) reads BOTH axes —
#        `clearance_failure("print", &edge)` instead of the landed
#        `edge.conf` check
#        -> n966_b13_idiom_pin_passes_today MUST FAIL
#        -> n966_the_call_llm_flow_output_passes_inline MUST FAIL
#        -> n948_boundary_pins_pass_check_today_the_gap_is_real MUST FAIL
#           (b13 rides STILL_OPEN_PINS — the CURRENT-behavior pin)
#        (the idiom shape refuses with [HTML_INJECTION]; the boundary
#        moved — loudly, exactly what the pin exists to catch).
#
#   M2 — the conf control is silenced (the negative control of M1): the
#        whole output-edge gate is dropped
#        -> n966_private_flow_output_still_refuses MUST FAIL
#        (a private flow output passes — the №965 confidentiality gate
#        is gone; proves the pin tests gate REAL behavior on both axes).
#
# Run from the repo root:  bash scripts/mutation_verify_966.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

# The restore is a BACKUP-COPY restore, never `git checkout --`: the
# working tree carries UNCOMMITTED naryad work — a checkout would wipe it
# (the lesson of the first M-run).
BACKUP=/tmp/n966_semantic_backup.rs
cp src/semantic.rs "$BACKUP" || exit 1
fail() { cp "$BACKUP" src/semantic.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BACKUP" src/semantic.rs; }

echo "── M1: the flow output edge is gated on BOTH axes ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
needle = """                if edge.conf != crate::labels::Conf::Public {
                    // The output edge is the print channel (the flow's
                    // stdout emission) — the frozen class of the print
                    // sink, no new check_id (ADR-0154 §12.4).
                    violations.push(SinkViolation {
                        container: format!("flow {}", f.name),
                        fn_name: "print".to_string(),
                        arg_index: 0,
                        span: f.span.clone(),
                        reason: "private-egress",
                        label: edge.clone(),
                    });
                }"""
replacement = """                // MUTATION (M1, №966): the output edge is gated as the
                // print channel it is on BOTH axes — the integrity half
                // the owner decision 13–14.11 held back.
                if let Some(reason) = clearance_failure("print", &edge) {
                    violations.push(SinkViolation {
                        container: format!("flow {}", f.name),
                        fn_name: "print".to_string(),
                        arg_index: 0,
                        span: f.span.clone(),
                        reason,
                        label: edge.clone(),
                    });
                }"""
assert needle in s, "the landed Flow-arm output gate not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_966_flow_integrity 2>&1 | grep -q "test result: FAILED" \
  && echo "M1 RED — the pin works" || fail "M1: the №966 idiom pin stayed GREEN under the both-axes mutation"
CARGO_INCREMENTAL=0 cargo test --test naryad_948_boundary_pins n948_boundary_pins_pass_check_today_the_gap_is_real 2>&1 | grep -q "test result: FAILED" \
  && echo "M1 RED on the STILL_OPEN harness — the pin works" || fail "M1: the №948 STILL_OPEN harness stayed GREEN under the both-axes mutation"
restore

echo "── M2: the whole output-edge gate is dropped (the negative control) ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
needle = """                if edge.conf != crate::labels::Conf::Public {"""
replacement = """                if false && edge.conf != crate::labels::Conf::Public {"""
assert needle in s, "the landed Flow-arm output gate not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_966_flow_integrity n966_private_flow_output_still_refuses 2>&1 | grep -q "test result: FAILED" \
  && echo "M2 RED — the conf control works" || fail "M2: the confidentiality control stayed GREEN under the gate-drop mutation"
restore

echo "── restoring and re-checking the tree is byte-identical ──"
restore
cmp -s src/semantic.rs "$BACKUP" && echo "RESTORE OK" || fail "the restore is not byte-identical"
echo "MUTATION VERIFY №966: PASS"
