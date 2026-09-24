#!/usr/bin/env bash
# ── Naryad №448 (issue #656, Wave 15) — the interp statement-coverage
#    mutation verification ──────────────────────────────────────────────
#
# The two mutations the naryad names (task 4, contract ≥2/2 VERIFIED):
#
#   M1 — remove the Match-arm merge (the arm end-state never joins the
#        post-match state) and the summary-side Match union
#        -> a2/a3/a4 (tests/naryad_448_interp_statement_coverage.rs)
#        MUST FAIL (the leak in the arm stops being detected).
#
#   M2 — remove the break/continue exit-state collection
#        (loop_exits.push) -> b6 (taint+break inside an if-branch inside
#        a loop: the terminated branch reaches the exit ONLY through the
#        merge) MUST FAIL.
#
# Reproducibility contract: the harness is versioned IN THE REPO and
# works in a throwaway worktree sharing the main target dir — no manual
# edits, the working tree is never touched; the mutants never reach main.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0

WT=$(mktemp -d /tmp/n448-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (the CI box is disk-bound).
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (test filter) -> prints the cargo test result line
  # A FAILING test is the EXPECTED mutation outcome, so cargo's non-zero
  # exit must not trip `set -e`.
  ( cd "$WT" && cargo test --test naryad_448_interp_statement_coverage "$1" 2>&1 | grep -E "test result" | head -1 ) || true
}

mutate() { # (file, python snippet) — apply one injection in the worktree
  python3 - "$WT/$1" << PYEOF
import sys
path = sys.argv[1]
src = open(path).read()
needle = sys.stdin.read().strip()
assert needle in src, "mutation anchor not found: " + needle[:70]
src = src.replace(needle, "", 1)
open(path, "w").write(src)
print("mutated:", needle[:60].replace(chr(10), " "))
PYEOF
}

echo "── M1: remove the Match-arm state merge (walker) ──"
mutate src/audit.rs "                    if !terminated {
                        state.join_into(&arm_state);
                    }"
M1_WALKER=$(run_test a2_)
echo "  a2 after M1-walker: $M1_WALKER"
M1_WALKER_B=$(run_test a3_)
echo "  a3 after M1-walker: $M1_WALKER_B"

echo "── M1b: remove the summary-side Match union ──"
mutate src/audit.rs "                for arm in arms {
                    collect_params_into_return(arm.body(), params, out);
                }"
M1_SUMMARY=$(run_test a4_)
echo "  a4 after M1-summary: $M1_SUMMARY"

echo "── M2: remove the break/continue exit-state collection ──"
mutate src/audit.rs "loop_exits.push(state.clone());"
M2=$(run_test b6_)
echo "  b6 after M2: $M2"

echo "── verdict ──"
fail() { echo "  MUTANT SURVIVED: $1" ; exit 1; }
echo "$M1_WALKER" | grep -q "0 passed" || fail "M1 walker (a2)"
echo "$M1_WALKER_B" | grep -q "0 passed" || fail "M1 walker (a3)"
echo "$M1_SUMMARY" | grep -q "0 passed" || fail "M1 summary (a4)"
echo "$M2" | grep -q "0 passed" || fail "M2 (b6)"
echo "  4/4 mutants KILLED — mutation contract VERIFIED (M1: walker+summary, M2: break/continue merge)."
