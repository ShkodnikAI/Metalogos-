#!/usr/bin/env bash
# ── Naryad №436 (issue #630, Wave 11) — the embodied monitor bypass
#    mutation verification ─────────────────────────────────────────────
#
# The №382 mutation-verification protocol applied to the wave-11 example
# line (the two bypass mutations the naryad names):
#
#   M1: neuter the chunk_make bounds refusal (refusal -> pass-through)
#       -> tests/naryad_436_embodied_examples.rs n436_red_chunk_is_typed_unbounded
#       MUST FAIL.
#   M2: neuter the print-surface WorldState materialization guard
#       (check_print_arg: the WorldState leg AND the nonprintable floor
#       for WorldState fall through) -> n436_print_surface_refuses_private
#       MUST FAIL.
#
# Reproducibility contract: the harness is versioned IN THE REPO and
# works in a throwaway worktree sharing the main target dir — no manual
# edits, the working tree is never touched; the mutants never reach main.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n436-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (the CI box is disk-bound; a separate
# target dir died with `ld: Bus error` = ENOSPC).
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (test filter) -> prints the cargo test result line
  # A FAILING test is the EXPECTED mutation outcome, so cargo's non-zero
  # exit must not trip `set -e`.
  ( cd "$WT" && cargo test --test naryad_436_embodied_examples "$1" 2>&1 | grep -E "test result" | head -1 ) || true
}

mutate() { # (file, python snippet) — apply one injection in the worktree
  python3 - "$WT/$1" << PYEOF
import sys
p = sys.argv[1]
src = open(p).read()
$2
open(p, "w").write(src)
PYEOF
}

echo "── M1: neuter the chunk_make bounds refusal (unbounded pass-through) ──"
mutate "src/embodied.rs" '
old = """    let bounds_id = match &device.bounds {
        Some(f) => f.id.clone(),
        None => {
            ledger_embodied_event(
                \"chunk_denied\",
                device_id,
                \"reason=unbounded (ADR-0159 §2.4.2: no unmonitored action)\",
            );
            return Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_EMBODIED_UNBOUNDED,
                format!(
                    \"{}: device \x27{}\x27 carries no bounds formula — an ActionChunk without a bounds formula bound to its device profile is refused at the type level (ADR-0159 §2.4.2); attach one with bounds_attach\",
                    fn_name, device_id
                ),
            ));
        }
    };"""
new = """    let bounds_id = match &device.bounds {
        Some(f) => f.id.clone(),
        // MUTATION M1: the bounds refusal is neutered — the unbounded
        // device passes through with a fabricated bounds id.
        None => format!(\"mutant-m1-unbounded-{}\", device_id),
    };"""
assert old in src, "chunk_make bounds refusal anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test n436_red_chunk_is_typed_unbounded)
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M1 VERIFIED: the unbounded-chunk test falls when the refusal is neutered" ;;
  *) echo "  M1 NOT VERIFIED — the test survived a neutered bounds refusal"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/embodied.rs )

echo "── M2: neuter the print-surface WorldState materialization guard ──"
mutate "src/builtins/embodied.rs" '
old = """pub(crate) fn check_print_arg(v: &Value) -> Result<(), String> {
    if let Value::WorldState(_) = v {
        return Err(embodied::deny_world_state_materialization(\"print\", v));
    }
    if crate::interpreter::values::is_nonprintable(v) {"""
new = """pub(crate) fn check_print_arg(v: &Value) -> Result<(), String> {
    if let Value::WorldState(_) = v {
        // MUTATION M2: the private-materialization guard is neutered —
        // the WorldState falls through to the print path (bypass).
    } else if crate::interpreter::values::is_nonprintable(v) {"""
assert old in src, "check_print_arg WorldState guard anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test n436_print_surface_refuses_private)
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M2 VERIFIED: the print-surface test falls when the guard is neutered" ;;
  *) echo "  M2 NOT VERIFIED — the test survived a neutered print guard"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/builtins/embodied.rs )

echo "── №436 mutation verification: 2/2 VERIFIED ──"
