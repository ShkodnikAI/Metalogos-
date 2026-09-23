#!/usr/bin/env bash
# ── Naryad №441 (issue #636, Wave 12) — the forecast taint/degradation
#    mutation verification ──────────────────────────────────────────────
#
# The №382 mutation-verification protocol applied to the wave-12
# example line (the two mutations the naryad names):
#
#   M1: neuter the sink gate on the forecast export surface
#       (check_export_allowed passes everything through)
#       -> tests/naryad_441_forecast_examples.rs n441_red_export_is_typed_denied
#       MUST FAIL.
#   M2: falsify the degraded flag (the ladder silently skips rungs —
#       degraded is a constant false)
#       -> n441_green_ladder_degrades_loudly MUST FAIL.
#
# Reproducibility contract: the harness is versioned IN THE REPO and
# works in a throwaway worktree sharing the main target dir — no manual
# edits, the working tree is never touched; the mutants never reach main.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n441-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (the CI box is disk-bound; a separate
# target dir died with `ld: Bus error` = ENOSPC).
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (test filter) -> prints the cargo test result line
  # A FAILING test is the EXPECTED mutation outcome, so cargo's non-zero
  # exit must not trip `set -e`.
  ( cd "$WT" && cargo test --test naryad_441_forecast_examples "$1" 2>&1 | grep -E "test result" | head -1 ) || true
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

echo "── M1: neuter the forecast taint export gate (tainted pass-through) ──"
mutate "src/forecast.rs" '
old = """    let reg = state().lock().map_err(|_| \"forecast registry poisoned\")?;
    let rec = match reg.forecasts.get(&id) {
        Some(r) => r,
        None => return Err(err_handle_unknown(\"forecast export\", \"ForecastHandle\", &id)),
    };
    if is_tainted(&rec.label) {
        return Err(deny_tainted_export(surface, &id, &rec.label.to_string()));
    }
    Ok(())
}"""
new = """    let reg = state().lock().map_err(|_| \"forecast registry poisoned\")?;
    let rec = match reg.forecasts.get(&id) {
        Some(r) => r,
        None => return Err(err_handle_unknown(\"forecast export\", \"ForecastHandle\", &id)),
    };
    // MUTATION M1: the taint gate is neutered — a tainted forecast
    // exports its points through every gated surface (the bypass).
    if false && is_tainted(&rec.label) {
        return Err(deny_tainted_export(surface, &id, &rec.label.to_string()));
    }
    Ok(())
}"""
assert old in src, "check_export_allowed taint-gate anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test n441_red_export_is_typed_denied)
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M1 VERIFIED: the taint-export test falls when the gate is neutered" ;;
  *) echo "  M1 NOT VERIFIED — the test survived a neutered taint gate"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/forecast.rs )

echo "── M2: falsify the degraded flag (the ladder silently skips rungs) ──"
mutate "src/forecast.rs" '
old = """        let degraded = !skipped.is_empty();"""
new = """        // MUTATION M2: the degraded flag is falsified — rungs skip
        // silently and the prov block lies about the degradation.
        let degraded = false;"""
assert old in src, "forecast_next degraded-flag anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test n441_green_ladder_degrades_loudly)
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M2 VERIFIED: the ladder test falls when the degraded flag lies" ;;
  *) echo "  M2 NOT VERIFIED — the test survived a falsified degraded flag"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/forecast.rs )

echo "── №441 mutation verification: 2/2 VERIFIED ──"
