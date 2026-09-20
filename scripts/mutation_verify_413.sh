#!/usr/bin/env bash
# ── Naryad №413 (issue #558) — the try-stamp mutation verification ─────
#
# The №382 mutation-verification protocol applied to the new origin
# stamps (the mutations the naryad names — "kick out the new stamp"):
#
#   M-CRON: neuter `cron_stamped` (the CRON_JOB_FAILED origin stamp)
#           -> tests/naryad_413_try_stamps.rs cron tests MUST FAIL.
#   M-MCP:  drop the MCP entries from `ORIGIN_STAMPED_CODES`
#           -> tests/naryad_413_try_stamps.rs MCP tests MUST FAIL.
#
# Reproducibility contract: the harness is versioned IN THE REPO and
# works in a throwaway worktree sharing the main target dir — no manual
# edits, the working tree is never touched.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n413-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (the CI box is disk-bound; a separate
# target dir died with `ld: Bus error` = ENOSPC).
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (test filter) -> prints the cargo test result line
  # A FAILING test is the EXPECTED mutation outcome, so cargo's non-zero
  # exit must not trip `set -e`.
  ( cd "$WT" && cargo test --test naryad_413_try_stamps "$1" 2>&1 | grep -E "test result" | head -1 ) || true
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

echo "── M-CRON: neuter the CRON_JOB_FAILED origin stamp ──"
mutate "src/builtins/cron.rs" '
old = """fn cron_stamped(e: String) -> String {
    if split_origin_stamp(&e).is_some() {
        e
    } else {
        coded_error(CODE_CRON_JOB_FAILED, e)
    }
}"""
new = """fn cron_stamped(e: String) -> String {
    // MUTATION M-CRON: the stamp is neutered (the pass-through identity)
    e
}"""
assert old in src, "cron_stamped anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test n413_cron_invalid_expression_code_on_both_backends)
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-CRON VERIFIED: the cron stamp test falls when the stamp is removed" ;;
  *) echo "  M-CRON NOT VERIFIED — the test survived a neutered stamp"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/builtins/cron.rs )

echo "── M-MCP: drop the MCP entries from the classifier whitelist ──"
mutate "src/interpreter/values.rs" '
old = """    CODE_CRON_JOB_FAILED,
    CODE_MCP_SPAWN_FAILED,
    CODE_MCP_TIMEOUT,
    CODE_MCP_IO_ERROR,
    CODE_MCP_TOOL_ERROR,
    CODE_MCP_TOOL_NOT_FOUND,
    CODE_MCP_PROTOCOL_ERROR,
    CODE_MCP_NOT_ALLOWLISTED,
];"""
new = """    CODE_CRON_JOB_FAILED,
    // MUTATION M-MCP: the MCP entries dropped
];"""
assert old in src, "whitelist anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test n413_mcp_spawn_failure_code_on_both_backends)
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-MCP VERIFIED: the MCP stamp test falls when the whitelist drops the entries" ;;
  *) echo "  M-MCP NOT VERIFIED — the test survived a neutered whitelist"; exit 1 ;;
esac

echo "── try-stamp mutation verification: 2/2 VERIFIED ──"
