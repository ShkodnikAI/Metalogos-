#!/usr/bin/env bash
# ── Naryad №415 (issue #567) — the ledger-verify mutation verification ─
#
# The №382 mutation-verification protocol applied to the runtime hook:
# the two crypto checks the structural verdict must pin are mutated, and
# each mutant MUST fail its named test in tests/naryad_415_ledger_verify.rs:
#
#   M-SIG:  neuter the per-record Ed25519 signature check
#           (vk.verify(...) -> ignored)
#           -> n415_ledger_verify_flipped_signature_fails_at_record MUST FAIL.
#   M-LINK: neuter the prev-hash chain linkage
#           (r.prev_hash != prev_hash -> never taken)
#           -> n415_ledger_verify_relinked_prev_hash_fails_at_record MUST FAIL.
#
# The RE-LINK attack test (5) is the only one that isolates the linkage:
# the forged record carries a valid body hash and a valid signature under
# the correct key, so nothing but the prev-hash check sees it. Likewise
# the flipped-signature test (4) isolates the signature check: the body
# and the hash are untouched, the sig lives outside the hashed body.
#
# Reproducibility contract: the harness is versioned IN THE REPO and
# works in a throwaway worktree sharing the main target dir — no manual
# edits, the working tree is never touched. Run AFTER the code lands in
# a commit (the worktree checks out HEAD — the №413 lesson).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n415-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (the CI box is disk-bound; a separate
# target dir died with `ld: Bus error` = ENOSPC — the №413 lesson).
export CARGO_TARGET_DIR="$PWD/target"

echo "── sanity: the unmutated corpus is green at HEAD ──"
S=$( ( cd "$WT" && cargo test --test naryad_415_ledger_verify 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $S"
case "$S" in
  *"0 failed"*) echo "  sanity OK: 11/11 green before mutating" ;;
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

echo "── M-SIG: neuter the per-record Ed25519 signature check ──"
mutate "src/ledger.rs" '
old = """        vk.verify(r.hash.as_bytes(), &sig)
            .map_err(|_| VerifyFault {
                record: Some(n as u64),
                reason: "signature verification FAILED".to_string(),
            })?;"""
new = """        // MUTATION M-SIG: the signature check is neutered (the forged
        // signature is accepted — tamper-evidence is gone).
        let _ = &sig;"""
assert old in src, "M-SIG anchor not found (the vk.verify block)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_415_ledger_verify n415_ledger_verify_flipped_signature_fails_at_record 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-SIG VERIFIED: the flipped-signature test falls when the signature check is neutered" ;;
  *) echo "  M-SIG NOT VERIFIED — the test survived a neutered signature check"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/ledger.rs )

echo "── M-LINK: neuter the prev-hash chain linkage ──"
mutate "src/ledger.rs" '
old = """        if r.prev_hash != prev_hash {"""
new = """        if false {
            // MUTATION M-LINK: the linkage check is neutered (the
            // re-linked record is accepted — chain-walk integrity gone).
            let _ = prev_hash;"""
assert old in src, "M-LINK anchor not found (the prev_hash check)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_415_ledger_verify n415_ledger_verify_relinked_prev_hash_fails_at_record 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-LINK VERIFIED: the re-link test falls when the linkage is neutered" ;;
  *) echo "  M-LINK NOT VERIFIED — the test survived a neutered linkage"; exit 1 ;;
esac

echo "── ledger-verify mutation verification: 2/2 VERIFIED ──"
