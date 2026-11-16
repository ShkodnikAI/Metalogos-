#!/usr/bin/env bash
# ── Naryad №957 (issue #1924, ADR-0211 §3, [security][consent], P1) —
#    the mutation verification for the verifiable mandate actor ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the counted class loses its marking: the gate reads ANY live
#        mandate from the verified store, ignoring `actor_verified`
#        («засчитать программный мандат»)
#        -> n957_the_c05_probe_refuses_a_program_issued_mandate MUST FAIL
#        -> n957_cli_approve_is_the_host_ritual_the_program_cannot_perform
#           step 1 becomes GREEN-ish? No: step 1 refuses only when the
#           program mandate does not count — under M1 the program's own
#           mandate opens the carrier, so the e2e step 1 PASSES and the
#           test FAILS at its step-3/step-1 asymmetry assertion.
#        (the trust anchor moves back INTO the program's reach — the
#        exact hole the audit's c05 probe found.)
#
#   M2 — the same mutation at the MACHINE level: the gate_036 site
#        marker (actor_verified inside authorize_overseen_call)
#        disappears -> APPROVAL_COVERAGE reads 0 against the 10000 bp
#        only-up floor -> `gen_metrics.py --check` MUST FAIL
#        (the floor follows the fact).
#
# Run from the repo root:  bash scripts/mutation_verify_957.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

# The restore is a BACKUP-COPY restore, never `git checkout --`: the
# working tree carries UNCOMMITTED naryad work — a checkout would wipe it
# (the lesson of the first M-run).
BACKUP=/tmp/n957_approval_backup.rs
cp src/approval.rs "$BACKUP" || exit 1
fail() { cp "$BACKUP" src/approval.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BACKUP" src/approval.rs; }

echo "── M1: a program-issued mandate counts (the trust anchor moves into the program) ──"
python3 - <<'EOF'
p = 'src/approval.rs'
s = open(p).read()
needle = """    match verified_store_load() {
        Ok(store) => {
            if store
                .iter()
                .any(|m| m.actor_verified && m.verify(now, &digest) == Verdict::Allow)
            {
                return Ok(());
            }
        }"""
replacement = """    match lock_store() {
        Ok(store) => {
            if store
                .iter()
                // MUTATION (M1, №957): the PROGRAM-issued in-memory store
                // is counted — a program approves itself again (the
                // audit's c05 hole, the v0.40.0 behavior restored).
                .any(|m| m.verify(now, &digest) == Verdict::Allow)
            {
                return Ok(());
            }
        }"""
assert needle in s, "the verified-class gate body not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_957_verified_mandate 2>&1 | grep -q "test result: FAILED" \
  && echo "M1 RED — the c05 pin works" || fail "M1: the c05 flip pin stayed GREEN under the count-unverified mutation"
restore

echo "── M2: the same mutation at the machine level (the gate_036 floor) ──"
python3 - <<'EOF'
p = 'src/approval.rs'
s = open(p).read()
needle = """    match verified_store_load() {
        Ok(store) => {
            if store
                .iter()
                .any(|m| m.actor_verified && m.verify(now, &digest) == Verdict::Allow)
            {
                return Ok(());
            }
        }"""
replacement = """    match lock_store() {
        Ok(store) => {
            if store
                .iter()
                .any(|m| m.verify(now, &digest) == Verdict::Allow)
            {
                return Ok(());
            }
        }"""
assert needle in s, "the verified-class gate body not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
python3 scripts/gen_metrics.py --check >/dev/null 2>&1 \
  && fail "M2: the gate_036 floor stayed GREEN while the verified-class site was removed" \
  || echo "M2 RED — the machine floor follows the fact"
restore

echo "── restoring and re-checking the tree is byte-identical ──"
restore
cmp -s src/approval.rs "$BACKUP" && echo "RESTORE OK" || fail "the restore is not byte-identical"
echo "MUTATION VERIFY №957: PASS"
