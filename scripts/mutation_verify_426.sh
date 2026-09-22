#!/usr/bin/env bash
# ── Naryad №426 (issue #597) — the tick-context mutation verification ──
#
# The №382 mutation-verification protocol applied to the ADR-0175 tick
# context (the mutations the naryad names):
#
#   M-CRON-BIND:   neuter the tick context's db binding (drop the
#                  reconnect_db call from fresh_program_context)
#                  -> tests/naryad_426_cron_context.rs tick-db tests MUST FAIL.
#   M-SCHEMA-REPL: neuter the schema replay (empty replay_schemas body)
#                  -> the schema-visibility tests MUST FAIL.
#
# Reproducibility contract: the harness is versioned IN THE REPO and
# works in a throwaway worktree sharing the main target dir — no manual
# edits, the working tree is never touched.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n426-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (the CI box is disk-bound; a separate
# target dir died with `ld: Bus error` = ENOSPC).
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (test filter) -> prints the cargo test result line
  # A FAILING test is the EXPECTED mutation outcome, so cargo's non-zero
  # exit must not trip `set -e`.
  ( cd "$WT" && timeout 240 cargo test --test naryad_426_cron_context "$1" 2>&1 | grep -E "test result" | head -1 ) || true
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

verdict() { # (name, result line) — a mutation VERIFIED when its named test FAILED
  if echo "$2" | grep -q "FAILED"; then
    echo "  [$1] VERIFIED — the mutant is killed ($2)"
  else
    echo "  [$1] NOT VERIFIED — the mutant survived ($2)"
    exit 1
  fi
}

echo "── M-CRON-BIND: neuter the tick context's program binding ──"
# The binding IS the context construction (definitions + db + schema —
# the route-style stor-set). Neutered, the tick cannot even resolve the
# pattern, let alone the db: the named tests MUST fail.
mutate "src/server.rs" '
old = """async fn fresh_program_context(state: &ServerState) -> Interpreter {
    let mut interp = Interpreter::new();
    {
        let shared = state.interpreter.read().await;
        shared.clone_definitions_into(&mut interp);
    }"""
new = """async fn fresh_program_context(state: &ServerState) -> Interpreter {
    // M-CRON-BIND: the program-context binding is neutered — the tick
    // gets a BARE interpreter (no definitions, no db, no schema).
    let _ = state;
    return Interpreter::new();
    #[allow(unreachable_code)]
    {
    let mut interp = Interpreter::new();
    {
        let shared = state.interpreter.read().await;
        shared.clone_definitions_into(&mut interp);
    }"""
assert old in src, "M-CRON-BIND anchor not found"
src = src.replace(old, new)
# close the injected unreachable block at the fn end
old2 = """    interp.reconnect_db();
    interp
}

/// №426 (ADR-0175 §3.1-3.3): the cron-dispatch executor."""
new2 = """    interp.reconnect_db();
    interp
    }
}

/// №426 (ADR-0175 §3.1-3.3): the cron-dispatch executor."""
assert old2 in src, "M-CRON-BIND tail anchor not found"
src = src.replace(old2, new2)
'
res=$(run_test "tick_binds_db_and_replays_the_schema_ddl")
verdict "M-CRON-BIND" "$res"

echo "── M-SCHEMA-REPL: neuter the schema replay ──"
mutate "src/interpreter/db.rs" '
old = """    pub fn replay_schemas(&self) {
        let has_conn = self"""
new = """    pub fn replay_schemas(&self) {
        // M-SCHEMA-REPL: the replay is neutered — the stored DDL never
        // reaches any connection.
        return;
        let has_conn = self"""
assert old in src, "M-SCHEMA-REPL anchor not found"
src = src.replace(old, new)
'
res=$(run_test "tick_binds_db_and_replays_the_schema_ddl")
verdict "M-SCHEMA-REPL" "$res"

echo "── 2/2 VERIFIED ──"
