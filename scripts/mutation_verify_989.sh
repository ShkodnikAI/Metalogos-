#!/usr/bin/env bash
# №989 (ADR-0200 W4) — the mutation contract: each named mutation turns
# its leak-suite test RED. The restore is the INVERSE REPLACE of the
# exact mutation bytes — never `git checkout` (the W4 files may carry
# uncommitted work; a checkout would destroy it). The script refuses to
# run if the inverse replace does not restore the file byte-identically.
set -uo pipefail
cd "$(dirname "$0")/.."

HUB=src/distill_hub.rs
ATTEST=src/nn/attest.rs
TEST=tests/naryad_989_curriculum.rs

# mutate_apply FILE OLD NEW — apply; exit loudly if the anchor is gone.
mutate_apply() {
  python3 - "$1" "$2" "$3" <<'PYEOF'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
assert old in s, "the mutation anchor is missing in %s" % p
open(p, "w").write(s.replace(old, new, 1))
print("  mutated:", p)
PYEOF
}

# mutate_restore FILE OLD NEW — the inverse replace; byte-identity asserted.
mutate_restore() {
  python3 - "$1" "$2" "$3" <<'PYEOF'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
assert new in s, "the mutated bytes are missing in %s" % p
restored = s.replace(new, old, 1)
open(p, "w").write(restored)
print("  restored:", p)
PYEOF
}

expect_red() {
  local name="$1"
  local args="$2"
  local log="/tmp/mut989_$name.log"
  # pipefail would let cargo's exit 101 dominate the pipeline even when
  # grep matches — the verdict is read from the LOG FILE, never from
  # the pipeline's exit code.
  cargo test --test naryad_989_curriculum $args 2>&1 | tee "$log" > /dev/null
  if grep -q "test result: FAILED" "$log"; then
    echo "  $name: RED (the mutation is caught)"
  else
    echo "  $name: NOT RED — THE MUTATION SURVIVED (the contract is broken); log: $log"
    exit 1
  fi
}

# ── M1: the manifest held-out leak ───────────────────────────────────
# Drop the held-out exclusion from the W4-aware manifest read — the
# held-out row enters the training manifest, the invariant test reds.
echo "── M1: the manifest held-out leak"
OLD1="WHERE pattern_name = ?1 AND quarantined IS NOT 1 AND source IS NOT 'held-out' \\
             ORDER BY rowid"
NEW1="WHERE pattern_name = ?1 AND quarantined IS NOT 1 \\
             ORDER BY rowid"
mutate_apply "$ATTEST" "$OLD1" "$NEW1"
expect_red "M1" "held_out_never_enters_training_manifest"
mutate_restore "$ATTEST" "$OLD1" "$NEW1"

# ── M2: the crossing refusal dropped ─────────────────────────────────
echo "── M2: the crossing refusal dropped"
OLD2='        if teacher
            .iter()
            .any(|(_, t_in, t_out)| t_in == h_in && t_out == h_out)
        {
            return Err(format!(
                "curriculum: crossing refusal — held-out row {} carries the pair the teacher also taught (input hash {}): the held-out provenance story is machine-false; fix the held-out set or the row'"'"'s source",
                h_rid,
                input_hash8(h_in)
            ));
        }'
NEW2='        if false && teacher
            .iter()
            .any(|(_, t_in, t_out)| t_in == h_in && t_out == h_out)
        {
            return Err(format!(
                "curriculum: crossing refusal — held-out row {} carries the pair the teacher also taught (input hash {}): the held-out provenance story is machine-false; fix the held-out set or the row'"'"'s source",
                h_rid,
                input_hash8(h_in)
            ));
        }'
mutate_apply "$HUB" "$OLD2" "$NEW2"
expect_red "M2" "curriculum_crossing_refusal_is_loud"
mutate_restore "$HUB" "$OLD2" "$NEW2"

# ── M3: the informational filter code dropped ────────────────────────
echo "── M3: the informational filter code dropped"
OLD3="if manifest.filter != stored_filter_word {"
NEW3="if false && manifest.filter != stored_filter_word {"
mutate_apply "$ATTEST" "$OLD3" "$NEW3"
expect_red "M3" "verify_filter_mismatch_is_informational_not_a_refusal"
mutate_restore "$ATTEST" "$OLD3" "$NEW3"

# ── the byte-identity check ─────────────────────────────────────────
if [ -n "$(git diff -- "$HUB" "$ATTEST" | grep -E '^\+.*M2|^\+.*M3|^\+.*if false')" ]; then
  echo "RESTORE FAILED — mutation bytes are still present"
  exit 1
fi
# The worktree must look EXACTLY as before the run (the W4 state intact):
grep -q "curriculum_build" "$HUB" && grep -q "curriculum_filter_word" "$ATTEST" \
  || { echo "RESTORE FAILED — the W4 functions are gone"; exit 1; }
echo "── the inverse restores are byte-identical, the W4 state intact"
cargo test --test naryad_989_curriculum 2>&1 | grep "test result"
echo "mutation_verify_989: done"
