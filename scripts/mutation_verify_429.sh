#!/usr/bin/env bash
# ── Naryad №429 (issue #615) — the memory office-path mutation verification ──
#
# The №382 mutation-verification protocol applied to the office-path
# contour (the runtime facts the audit named):
#
#   M-CASCADE-LEDGER: neuter the forget_cascade ledger record (an
#                     irreversible cascade leaves no Action-Ledger trace)
#                     -> n429_ledger_records_every_office_step MUST FAIL.
#   M-CONSENT-PATH:   neuter the private-container consent gate (a private
#                     memory_open succeeds without an active grant)
#                     -> n429_office_path_end_to_end MUST FAIL.
#
# Reproducibility contract (the №418 harness pattern): the mutations are
# applied in a throwaway worktree sharing the main target dir; cargo runs
# IN THE WORKTREE; every injection asserts its anchor; the working tree is
# never touched. Run AFTER the code lands in a commit.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n429-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
export CARGO_TARGET_DIR="$PWD/target"

echo "── sanity: the unmutated corpus is green at HEAD ──"
S=$( ( cd "$WT" && cargo test --test naryad_429_memory_office_path 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $S"
case "$S" in
  *"0 failed"*) echo "  sanity OK: the corpus is green before mutating" ;;
  *) echo "  SANITY FAILED — the corpus must be green BEFORE mutation"; exit 1 ;;
esac

mutate() { # (file, python snippet)
  python3 - "$WT/$1" << PYEOF
import sys
p = sys.argv[1]
src = open(p).read()
$2
open(p, "w").write(src)
PYEOF
}

echo "── M-CASCADE-LEDGER: neuter the forget_cascade ledger record ──"
mutate "src/memory_typed.rs" '
old = """    ledger_memory_event(
        \"forget_cascade\",
        handle_id,
        &format!(
            \"root={}|deleted={}|batch={}|grant={}\",
            key,
            outcome.deleted.len(),
            outcome.batch_id,
            grant.grant_id
        ),
    );"""
new = """    // MUTATION M-CASCADE-LEDGER: the irreversible cascade leaves NO
    // Action-Ledger trace (the audit trail of forgetting is gone).
    let _ = (&handle_id, &key, &grant, &outcome);"""
assert old in src, "M-CASCADE-LEDGER anchor not found (the forget_cascade record)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_429_memory_office_path n429_ledger_records_every_office_step 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-CASCADE-LEDGER VERIFIED: the ledger test falls when the cascade record is neutered" ;;
  *) echo "  M-CASCADE-LEDGER NOT VERIFIED — the test survived a silent cascade"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/memory_typed.rs )

echo "── M-CONSENT-PATH: neuter the private-container consent gate ──"
mutate "src/memory_typed.rs" '
old = """    if !crate::consent::active_grant_for(&scope) {"""
new = """    if false {
        // MUTATION M-CONSENT-PATH: the private consent gate is neutered."""
assert old in src, "M-CONSENT-PATH anchor not found (the private consent gate)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_429_memory_office_path n429_office_path_end_to_end 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-CONSENT-PATH VERIFIED: the office-path test falls when the consent gate is neutered" ;;
  *) echo "  M-CONSENT-PATH NOT VERIFIED — the test survived a neutered consent gate"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/memory_typed.rs )

echo "── mutation verification 2/2 VERIFIED ──"
