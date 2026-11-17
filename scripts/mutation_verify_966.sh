#!/usr/bin/env bash
# ── Naryad №967 (issue #1954 — the same-issue form, gh#1954; the B13
#    line, ADR-0154 §13.8 — form (а), the owner decision 16.11, P1,
#    [security][compiler]) — the mutation verification for the LANDED
#    flow-output integrity gate and the §12.2 terminator claim. ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the integrity arm is DROPPED: the unannotated flow edge goes
#        back to the confidentiality half only (the pre-№967 shape)
#        -> n966_b13_unannotated_refuses MUST FAIL
#        -> n966_the_call_llm_flow_output_unannotated_refuses MUST FAIL
#        -> n948_b13_flow_output_integrity_gated MUST FAIL
#        (the boundary moved back to silence — loudly, exactly what the
#        flipped pins exist to catch).
#
#   M2 — the WHOLE output-edge gate is dropped (the negative control of
#        M1): the unannotated clearance AND the annotated confidentiality
#        layer both dead
#        -> n966_private_flow_output_still_refuses MUST FAIL
#        -> n966_private_flow_output_annotated_still_refuses MUST FAIL
#        (a private flow output passes — the №965 confidentiality gate
#        is gone; proves the pins test gate REAL behavior on both axes).
#
#   M3 — the annotation becomes a SILENCER: the §12.2 claim check is
#        skipped (any declared label passes the annotated flow)
#        -> n966_the_weaker_claim_refuses MUST FAIL
#        (`-> output<public, trusted>` over an untrusted edge would
#        pass — the silencer the design forbids).
#
#   M4 — the confidentiality half becomes DECLAREABLE: the annotated
#        private-edge layer is dead
#        -> n966_private_flow_output_annotated_still_refuses MUST FAIL
#        (the declaration would waive №965 — the layer the design keeps
#        unconditional).
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
red() { CARGO_INCREMENTAL=0 cargo test --test "$1" "$2" 2>&1 | grep -q "test result: FAILED"; }

echo "── M1: the integrity arm is dropped (the unannotated edge, conf half only) ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
needle = """                    (None, _) => {
                        // The unannotated edge answers to the REAL print
                        // sink's clearance — both axes, the frozen
                        // classes (ADR-0154 §12.4).
                        if let Some(reason) = clearance_failure("print", &edge) {"""
replacement = """                    (None, _) => {
                        // MUTATION (M1, №967): the integrity arm is
                        // dropped — the unannotated edge answers to the
                        // confidentiality half only (the pre-№967 shape).
                        if let Some(reason) = (if edge.conf != crate::labels::Conf::Public {
                            Some("private-egress")
                        } else {
                            None
                        }) {"""
assert needle in s, "the landed unannotated-arm gate not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
red naryad_966_flow_integrity n966_b13_unannotated_refuses \
  && echo "M1 RED on the b13 negative — the pin works" \
  || fail "M1: the b13 negative stayed GREEN under the integrity-drop mutation"
red naryad_948_boundary_pins n948_b13_flow_output_integrity_gated \
  && echo "M1 RED on the №948 FLIPPED harness — the pin works" \
  || fail "M1: the №948 FLIPPED harness stayed GREEN under the integrity-drop mutation"
restore

echo "── M2: the whole output-edge gate is dropped (the negative control) ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
# the unannotated arm: dead
needle1 = """                        if let Some(reason) = clearance_failure("print", &edge) {"""
repl1 = """                        if let Some(reason) = None::<&'static str> {"""
# the annotated confidentiality layer: dead
needle2 = """                        } else if edge.conf != crate::labels::Conf::Public {"""
repl2 = """                        } else if false && edge.conf != crate::labels::Conf::Public {"""
assert needle1 in s and needle2 in s, "the landed gate layers not found — the tree drifted"
s = s.replace(needle1, repl1, 1).replace(needle2, repl2, 1)
open(p, 'w').write(s)
EOF
red naryad_966_flow_integrity n966_private_flow_output_still_refuses \
  && echo "M2 RED on the unannotated conf control — the pin works" \
  || fail "M2: the unannotated conf control stayed GREEN under the gate-drop mutation"
red naryad_966_flow_integrity n966_private_flow_output_annotated_still_refuses \
  && echo "M2 RED on the annotated conf control — the pin works" \
  || fail "M2: the annotated conf control stayed GREEN under the gate-drop mutation"
restore

echo "── M3: the annotation becomes a silencer (the §12.2 claim check skipped) ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
needle = """                        } else if decl.integrity == crate::labels::Integrity::Trusted
                            && edge.integrity == crate::labels::Integrity::Untrusted
                        {"""
replacement = """                        } else if false
                            && decl.integrity == crate::labels::Integrity::Trusted
                            && edge.integrity == crate::labels::Integrity::Untrusted
                        {"""
assert needle in s, "the landed §12.2 claim layer not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
red naryad_966_flow_integrity n966_the_weaker_claim_refuses \
  && echo "M3 RED on the weaker-claim negative — the pin works" \
  || fail "M3: the weaker-claim negative stayed GREEN — the annotation became a silencer undetected"
restore

echo "── M4: the confidentiality half becomes declareable (the annotated private layer dead) ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
needle = """                        } else if edge.conf != crate::labels::Conf::Public {"""
replacement = """                        } else if false && edge.conf != crate::labels::Conf::Public {"""
assert needle in s, "the landed annotated-conf layer not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
red naryad_966_flow_integrity n966_private_flow_output_annotated_still_refuses \
  && echo "M4 RED on the annotated conf control — the pin works" \
  || fail "M4: the annotated conf control stayed GREEN — the declaration waived №965 undetected"
restore

echo "── restoring and re-checking the tree is byte-identical ──"
restore
cmp -s src/semantic.rs "$BACKUP" && echo "RESTORE OK" || fail "the restore is not byte-identical"
echo "MUTATION VERIFY №967: PASS"
