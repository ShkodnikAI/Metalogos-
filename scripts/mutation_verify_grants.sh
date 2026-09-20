#!/usr/bin/env bash
# ── Naryad №412 (issue #557) — the grant-surface mutation verification ──
#
# The №382 mutation-verification protocol applied to the ADR-0155 grant
# checks (the two mutations the naryad names):
#
#   M-SCOPE: neuter `grants::scope_covers` (the GRANT_SCOPE_MISMATCH
#            gate)  -> tests/naryad_390_grants.rs
#            grant_scope_mismatch_refuses_at_runtime MUST FAIL.
#   M-TTL:   neuter the `GRANT_EXPIRED` gate in `grants::check_record`
#            -> tests/naryad_390_grants.rs
#            born_expired_grant_refuses_with_grant_expired MUST FAIL.
#
# Reproducibility contract (№410 discipline): the harness is versioned
# IN THE REPO and does everything mechanically in a throwaway worktree —
# no manual edits, the working tree is never touched.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n412-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: the throwaway worktree must not build the
# whole dependency graph from scratch (the CI box is disk-bound; a
# separate target dir died with `ld: Bus error` = ENOSPC).
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (test filter) -> prints the cargo test result line
  # A FAILING test is the EXPECTED mutation outcome, so cargo's non-zero
  # exit must not trip `set -e` — capture the verdict line either way.
  ( cd "$WT" && cargo test --test naryad_390_grants "$1" 2>&1 | grep -E "test result" | head -1 ) || true
}

mutate() { # (python snippet file) — apply one injection in the worktree
  python3 - "$WT/src/grants.rs" << PYEOF
import sys
p = sys.argv[1]
src = open(p).read()
$1
open(p, "w").write(src)
PYEOF
}

echo "── M-SCOPE: neuter scope_covers (GRANT_SCOPE_MISMATCH gate) ──"
mutate '
old = "pub fn scope_covers(grant_scope: &str, op: &str, table: &str) -> bool {\n    scope_attenuates(grant_scope, &format!(\"db:{}:{}\", op, table.to_lowercase()))\n}"
new = "pub fn scope_covers(grant_scope: &str, op: &str, table: &str) -> bool {\n    let _ = (grant_scope, op, table); // MUTATION M-SCOPE: gate neutered\n    true\n}"
assert old in src, "scope_covers anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test grant_scope_mismatch_refuses_at_runtime)
echo "  test verdict: $R"
case "$R" in
  *FAILED*|"0 passed; 1 failed"*) echo "  M-SCOPE VERIFIED: the scope test falls when the gate is removed" ;;
  *) echo "  M-SCOPE NOT VERIFIED — the test survived a neutered gate"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/grants.rs )

echo "── M-TTL: neuter the GRANT_EXPIRED gate in check_record ──"
mutate '
old = "    if now_secs() >= rec.expires_at {\n        return Err(format!(\n            \"GRANT_EXPIRED: {} expired at unix {}\",\n            rec.id, rec.expires_at\n        ));\n    }"
new = "    if false && now_secs() >= rec.expires_at { // MUTATION M-TTL: gate neutered\n        return Err(format!(\n            \"GRANT_EXPIRED: {} expired at unix {}\",\n            rec.id, rec.expires_at\n        ));\n    }"
assert old in src, "GRANT_EXPIRED anchor not found"
src = src.replace(old, new, 1)'
R=$(run_test born_expired_grant_refuses_with_grant_expired)
echo "  test verdict: $R"
case "$R" in
  *FAILED*|"0 passed; 1 failed"*) echo "  M-TTL VERIFIED: the TTL test falls when the gate is removed" ;;
  *) echo "  M-TTL NOT VERIFIED — the test survived a neutered gate"; exit 1 ;;
esac

echo "── grant-surface mutation verification: 2/2 VERIFIED ──"
