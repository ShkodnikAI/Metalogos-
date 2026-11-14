#!/usr/bin/env bash
# ── Naryad №956 (issue #1923, the audit 12.11 — P0) — the mutation
#    verification for the fuzzer's delivery-form matrix ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
# The issue's «сделано, когда» names this one exactly: neuter the №950
# interprocedural summary and the fuzzer must go RED **by itself**,
# without any help from the leak suite.
#
#   M1 — the interprocedural summary is neutered (`interproc_summary_of`
#        always answers None, so a call falls back to the join of its
#        arguments — the v0.39.0 «open world»)
#        -> n956_every_delivery_cell_matches_its_recorded_verdict_on_both_backends
#           MUST FAIL on the pattern_return / param_to_sink / two_levels
#           cells, which drop back to PASS.
#
#   M2 — the entity seeds are taken OUT of the summary again (the №956
#        find restored as the bug: `entity_seed_env` is not consulted when
#        the return label is computed)
#        -> the same test MUST FAIL on sec_entity_secret/pattern_return
#           alone — the one cell the find was about. M2 is the narrow
#           probe: it proves the find is pinned by a cell of its own and
#           not merely covered by the broad M1.
#
# The restore is a BACKUP-COPY restore (the working tree carries the
# naryad's uncommitted work — `git checkout --` would wipe it).
# Run from the repo root:  bash scripts/mutation_verify_956.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

TEST=naryad_956_delivery_forms
BK=/tmp/n956_semantic_backup.rs
cp src/semantic.rs "$BK" || exit 1
fail() { cp "$BK" src/semantic.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BK" src/semantic.rs; }

echo "── M1: the interprocedural summary neutered ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p, encoding='utf-8').read()
needle = 'fn interproc_summary_of(name: &str) -> Option<InterprocSummary> {'
assert needle in s, "interproc_summary_of not found — the tree drifted"
mutated = needle + '\n    if true {\n        return None; // №956 M1 MUTATION\n    }'
open(p, 'w', encoding='utf-8').write(s.replace(needle, mutated, 1))
EOF
[ $? -eq 0 ] || fail "M1 could not be applied"
if cargo test --test "$TEST" >/tmp/n956_m1.log 2>&1; then
  fail "M1 (summary neutered) left the matrix GREEN — the interprocedural cells pin nothing"
fi
grep -q "the STATIC verdict moved" /tmp/n956_m1.log \
  || fail "M1 failed for the wrong reason (no cell-verdict assertion in the log)"
echo "   M1 OK — the matrix is RED, and on the cell assertion"
restore

echo "── M2: the entity seeds removed from the summary ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p, encoding='utf-8').read()
needle = '    let return_internal = infer_pattern_labels_seeded(&baseline_pattern, &seed).output_label;'
assert needle in s, "the seeded baseline not found — the tree drifted"
mutated = '    let return_internal = infer_pattern_labels(&baseline_pattern).output_label; // №956 M2 MUTATION'
open(p, 'w', encoding='utf-8').write(s.replace(needle, mutated, 1))
EOF
[ $? -eq 0 ] || fail "M2 could not be applied"
if cargo test --test "$TEST" >/tmp/n956_m2.log 2>&1; then
  fail "M2 (entity seeds out of the summary) left the matrix GREEN — the №956 find is not pinned"
fi
grep -q "sec_entity_secret/pattern_return" /tmp/n956_m2.log \
  || fail "M2 failed, but not on the cell the find is about (sec_entity_secret/pattern_return)"
echo "   M2 OK — the RED names the entity-return cell"
restore

echo "── the clean tree ──"
cargo test --test "$TEST" >/tmp/n956_clean.log 2>&1 \
  || fail "the clean tree is RED — restore did not take"
echo "MUTATION VERIFY: OK — M1 and M2 both spring, the clean tree is green"
