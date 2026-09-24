#!/usr/bin/env bash
# ── Naryad №449 (issue #657, Wave 15) — the effects-module mutation
#    verification ───────────────────────────────────────────────────────
#
# The two mutations the naryad names (task 4, contract ≥2/2 VERIFIED):
#
#   M1 — remove the irreversible effect from the derivation
#        (builtin_effects hardcodes `irreversible: false`)
#        -> tests/naryad_449_effects.rs a2 (canonical mappings: db_insert/
#        print are irreversible) MUST FAIL — the SSOT test/чек catches it.
#
#   M2 — remove the tainted+network escalation (is_taint_sink returns
#        false for effect-derived sinks)
#        -> c1 (prompt egress) and the leak-suite negative
#        n449_prompt_egress_chain MUST FAIL (the negative stops being
#        caught — BLOCKING mode panics on not_caught).
#
# Reproducibility contract: worktree + shared target dir; mutants never
# reach main. HAZARD: after a harness run, `touch src/audit.rs` in the
# MAIN tree before trusting cargo test results (mtime fingerprint
# collision with the last mutant binary).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0

WT=$(mktemp -d /tmp/n449-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
export CARGO_TARGET_DIR="$PWD/target"

mutate() { # (file, old, new)
  python3 - "$WT/$1" "$2" "$3" << 'PYEOF'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(path).read()
assert old in src, "mutation anchor not found: " + repr(old[:70])
open(path, "w").write(src.replace(old, new, 1))
print("mutated OK")
PYEOF
}

run449() { ( cd "$WT" && cargo test --test naryad_449_effects "$1" 2>&1 | grep -E "test result" | head -1 ) || true; }
runleak() { ( cd "$WT" && cargo test --test run_leak_suite leak_negatives_report 2>&1 | grep -E "test result" | head -1 ) || true; }

echo "── M1: remove the irreversible effect from the derivation ──"
mutate src/audit.rs \
  "irreversible: c.reversibility
            == crate::builtins_classification::Reversibility::Irreversible," \
  "irreversible: false,"
M1=$(run449 a2_)
echo "  a2 after M1: $M1"

echo "── M2: remove the tainted+network escalation ──"
mutate src/audit.rs \
  "    builtin_effects(name).map(|e| e.network).unwrap_or(false)" \
  "    false"
M2_C1=$(run449 c1_)
echo "  c1 after M2: $M2_C1"
M2_LEAK=$(runleak)
echo "  leak negatives after M2: $M2_LEAK"

echo "── verdict ──"
fail() { echo "  MUTANT SURVIVED: $1"; exit 1; }
echo "$M1" | grep -q "0 passed" || fail "M1 (a2 SSOT pins)"
echo "$M2_C1" | grep -q "0 passed" || fail "M2 (c1 prompt egress)"
echo "$M2_LEAK" | grep -q "0 passed" || fail "M2 (leak negative not caught)"
echo "  3/3 mutants KILLED — mutation contract VERIFIED (M1 SSOT irreversible; M2 network escalation × unit + corpus)."
