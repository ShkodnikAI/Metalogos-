#!/usr/bin/env bash
# ── Naryad №758 (issue #758, P1 vm) — the mutation verification ──────
#
#   M1 — the compiler drops the env NAME (db { url: env(...) } compiles
#        to the old silent None)
#        -> n758_env_db_url_resolves_lazily_and_db_execute_works MUST
#        FAIL (the Program no longer carries the name).
#
#   M2 — the loud reason for a FAILED env resolution is not stored
#        (the legacy "no database connection" riddle comes back)
#        -> n758_unset_env_db_url_is_a_loud_error MUST FAIL.
#
#   M3 — the loud reason for an UNSUPPORTED SCHEME is not stored
#        -> n758_non_sqlite_url_is_a_loud_error MUST FAIL.
#
# Reproducibility contract: versioned in the repo, a throwaway worktree
# sharing the main target dir; mutants never reach main. HAZARD NOTE
# (shared target): `touch src/vm.rs` in the MAIN tree before trusting a
# cargo test result that follows a harness run. CI (isolated) unaffected.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0

WT=$(mktemp -d /tmp/n758-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (filter) -> the result line; failure IS expected
  ( cd "$WT" && cargo test --test naryad_758_vm_db_env "$1" 2>&1 | grep -E "test result" | head -1 ) || true
}

mutate() { # (file, needle, replacement)
  python3 - "$WT/$1" "$2" "$3" << 'PYEOF'
import sys
path, needle, repl = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(path).read()
assert needle in src, "mutation anchor not found: " + repr(needle[:70])
src = src.replace(needle, repl, 1)
open(path, "w").write(src)
print("mutated OK:", repr(needle[:55]))
PYEOF
}

check_red() { # (label, result line)
  case "$2" in
    *"0 failed"*)
      echo "✗ $1: test stayed GREEN under mutation — the pin is broken"; exit 1 ;;
    *failed*)
      echo "✓ $1: RED as required — $2" ;;
    *)
      echo "✗ $1: no result line (compile error counts as caught, verify manually): $2"; exit 1 ;;
  esac
}

echo "── M1: the compiler drops the env NAME ──"
mutate src/compiler.rs "Some(crate::ast::Expr::StringLit { value: var, .. }) => {
                                    self.db_url_env = Some(var.clone());
                                }" \
                  "Some(crate::ast::Expr::StringLit { value: var, .. }) => {
                                    let _ = var;
                                    self.db_url_env = None;
                                }"
M1=$(run_test n758_env_db_url_resolves_lazily_and_db_execute_works)
check_red "M1" "$M1"
git -C "$WT" checkout -- src/compiler.rs

echo "── M2: the loud env-resolution reason is dropped ──"
mutate src/vm.rs "                    Err(e) => {
                        eprintln!(\"[vm/db] {}\", e);
                        self.db_open_failed = true;
                        self.db_open_error = Some(e);
                        return;
                    }" \
                  "                    Err(e) => {
                        eprintln!(\"[vm/db] {}\", e);
                        self.db_open_failed = true;
                        return;
                    }"
M2=$(run_test n758_unset_env_db_url_is_a_loud_error)
check_red "M2" "$M2"
git -C "$WT" checkout -- src/vm.rs

echo "── M3: the loud unsupported-scheme reason is dropped ──"
mutate src/vm.rs "            eprintln!(\"[vm/db] {}\", msg);
            self.db_open_failed = true;
            self.db_open_error = Some(msg);
            return;" \
                  "            eprintln!(\"[vm/db] {}\", msg);
            self.db_open_failed = true;
            return;"
M3=$(run_test n758_non_sqlite_url_is_a_loud_error)
check_red "M3" "$M3"
git -C "$WT" checkout -- src/vm.rs

echo ""
echo "All №758 mutations caught (3/3). The pins hold."
