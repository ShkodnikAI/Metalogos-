#!/usr/bin/env bash
# ── Naryad №418 (issue #571) — the cron-reliability mutation verification ──
#
# The №382 mutation-verification protocol applied to the fire decision:
#
#   M-TZ:   neuter the per-job timezone resolution (the decision matches in
#           UTC regardless of the job's IANA zone)
#           -> n418_tz_identity_moscow_vs_utc MUST FAIL.
#   M-DEDUP: neuter the last-window dedup guard (a fired window re-fires
#           on every tick — the pre-№418 12×/minute bug)
#           -> n418_dedup_one_fire_per_window MUST FAIL.
#
# Reproducibility contract: the harness is versioned IN THE REPO and works
# in a throwaway worktree sharing the main target dir — no manual edits,
# the working tree is never touched. Run AFTER the code lands in a commit
# (the worktree checks out HEAD — the №413 lesson).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n418-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (the CI box is disk-bound — the №413
# lesson: a separate target dir died with `ld: Bus error` = ENOSPC).
export CARGO_TARGET_DIR="$PWD/target"

echo "── sanity: the unmutated corpus is green at HEAD ──"
S=$( ( cd "$WT" && cargo test --test naryad_418_cron_reliability 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $S"
case "$S" in
  *"0 failed"*) echo "  sanity OK: the corpus is green before mutating" ;;
  *) echo "  SANITY FAILED — the corpus must be green BEFORE mutation"; exit 1 ;;
esac

mutate() { # (file, python snippet) — apply one injection in the worktree
  python3 - "$WT/$1" << PYEOF
import sys
p = sys.argv[1]
src = open(p).read()
$2
open(p, "w").write(src)
PYEOF
}

echo "── M-TZ: neuter the per-job timezone resolution ──"
mutate "src/builtins/cron.rs" '
old = """    let tz = resolve_job_tz(Some(&spec.tz))?;"""
new = """    // MUTATION M-TZ: the per-job timezone is neutered (every job matches
    // in UTC regardless of its IANA zone — the Render-host drift is back).
    let tz: chrono_tz::Tz = "UTC".parse().unwrap();
    let _ = resolve_job_tz;"""
assert old in src, "M-TZ anchor not found (the decision TZ resolution)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_418_cron_reliability n418_tz_identity_moscow_vs_utc 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-TZ VERIFIED: the TZ identity test falls when the zone resolution is neutered" ;;
  *) echo "  M-TZ NOT VERIFIED — the test survived a neutered timezone"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/builtins/cron.rs )

echo "── M-DEDUP: neuter the last-window dedup guard ──"
mutate "src/builtins/cron.rs" '
old = """    if spec.last_window == Some(current) {"""
new = """    if false {
            // MUTATION M-DEDUP: the dedup guard is neutered (a fired
            // window re-fires on every 5s tick — 12×/minute).
            let _ = spec.last_window;"""
assert old in src, "M-DEDUP anchor not found (the dedup guard)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_418_cron_reliability n418_dedup_one_fire_per_window 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-DEDUP VERIFIED: the dedup test falls when the guard is neutered" ;;
  *) echo "  M-DEDUP NOT VERIFIED — the test survived a neutered dedup"; exit 1 ;;
esac

echo "── cron-reliability mutation verification: 2/2 VERIFIED ──"
