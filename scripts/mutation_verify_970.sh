#!/usr/bin/env bash
# ── Naryad №970 (issue #1974 — the B14 line, the red-team finding F2,
#    p38_exec_step_stdout_display; form (ii), the owner decision 18.11,
#    P1, [security][compiler]) — the mutation verification for the
#    LANDED exec-stdout join on the flow edge. ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the exec-stdout join is DROPPED: the builtin `exec` pipeline
#        step stops joining `Integrity::Untrusted` into the flow edge
#        (the pre-№970 shape — the red-team F2 finding returns)
#        -> n970_b14_unannotated_refuses MUST FAIL
#        -> n970_the_exec_step_flow_unannotated_refuses MUST FAIL
#        -> n948_b14_exec_stdout_display_gated MUST FAIL
#        (the boundary moved back to silence — loudly, exactly what the
#        flipped pins exist to catch).
#
#   M2 — the join is moved OFF the exec step (it fires for EVERY
#        pipeline step): the zero-migration control is the named
#        mutation target — a join that rides the flow shape instead of
#        the exec step is the over-refusal bug
#        -> n970_flow_without_exec_stays_clean MUST FAIL
#
# Run from the repo root:  bash scripts/mutation_verify_970.sh
set -u
export PATH="/home/z/my-project/.rust-toolchain/cargo/bin:$PATH"
export CARGO_HOME=/home/z/my-project/.rust-toolchain/cargo
export RUSTUP_HOME=/home/z/my-project/.rust-toolchain/rustup

# The restore is a BACKUP-COPY restore, never `git checkout --`: the
# working tree carries UNCOMMITTED naryad work — a checkout would wipe it
# (the lesson of the first M-run).
BACKUP=/tmp/n970_semantic_backup.rs
cp src/semantic.rs "$BACKUP" || exit 1
fail() { cp "$BACKUP" src/semantic.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BACKUP" src/semantic.rs; }
red() { CARGO_INCREMENTAL=0 cargo test --test "$1" "$2" 2>&1 | grep -q "test result: FAILED"; }

echo "── M1: the exec-stdout join is dropped (the exec step back to silence) ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
needle = """                    if step_name == "exec" {
                        edge = edge.join(&crate::labels::Label {
                            conf: crate::labels::Conf::Public,
                            integrity: crate::labels::Integrity::Untrusted,
                            consent: crate::labels::ConsentScope::default(),
                        });
                    }"""
replacement = """                    // MUTATION (M1, №970): the exec-stdout join is
                    // dropped — the builtin exec pipeline step stops
                    // carrying Integrity::Untrusted into the flow edge
                    // (the pre-№970 shape; the red-team F2 finding
                    // returns).
                    if false && step_name == "exec" {
                        edge = edge.join(&crate::labels::Label {
                            conf: crate::labels::Conf::Public,
                            integrity: crate::labels::Integrity::Untrusted,
                            consent: crate::labels::ConsentScope::default(),
                        });
                    }"""
assert needle in s, "the landed exec-stdout join not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
red naryad_970_exec_stdout_claim n970_b14_unannotated_refuses \
  && echo "M1 RED on the b14 negative — the pin works" \
  || fail "M1: the b14 negative stayed GREEN under the join-drop mutation"
red naryad_970_exec_stdout_claim n970_the_exec_step_flow_unannotated_refuses \
  && echo "M1 RED on the inline exec-step negative — the pin works" \
  || fail "M1: the inline exec-step negative stayed GREEN under the join-drop mutation"
red naryad_948_boundary_pins n948_b14_exec_stdout_display_gated \
  && echo "M1 RED on the №948 FLIPPED harness — the pin works" \
  || fail "M1: the №948 FLIPPED harness stayed GREEN under the join-drop mutation"
restore

echo "── M2: the join moves off the exec step (fires for EVERY pipeline step) ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p).read()
needle = """                    if step_name == "exec" {
                        edge = edge.join(&crate::labels::Label {
                            conf: crate::labels::Conf::Public,
                            integrity: crate::labels::Integrity::Untrusted,
                            consent: crate::labels::ConsentScope::default(),
                        });
                    }"""
replacement = """                    // MUTATION (M2, №970): the join fires for EVERY
                    // pipeline step — the zero-migration control must
                    // catch the over-refusal (a join riding the flow
                    // shape instead of the exec step).
                    if true {
                        edge = edge.join(&crate::labels::Label {
                            conf: crate::labels::Conf::Public,
                            integrity: crate::labels::Integrity::Untrusted,
                            consent: crate::labels::ConsentScope::default(),
                        });
                    }"""
assert needle in s, "the landed exec-stdout join not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
red naryad_970_exec_stdout_claim n970_flow_without_exec_stays_clean \
  && echo "M2 RED on the no-exec control — the pin works" \
  || fail "M2: the no-exec control stayed GREEN — the join is not pinned to the exec step"
restore

cp "$BACKUP" src/semantic.rs
echo "MUTATION VERIFY: PASS — M1/M2 all named pins RED under their mutations; the tree restored from the backup copy."
