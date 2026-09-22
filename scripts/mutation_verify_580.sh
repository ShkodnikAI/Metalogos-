#!/usr/bin/env bash
# ── gh#580 (mutants smoke finding) — the killer-extension mutation
# verification ──
#
# The №382 mutation-verification protocol applied to the NEW properties
# (the two injections the fix must catch):
#
#   M-DICT-SET:   dict_set inserts Value::Unit instead of the given value
#                 -> tests/property_json_roundtrip.rs
#                 property_dict_store_consistency MUST FAIL.
#   M-HAS-FIELD:  has_field answers 0.0 for every present field
#                 -> tests/property_json_roundtrip.rs
#                 property_has_field_exact_answers MUST FAIL.
#
# Reproducibility contract (№410 discipline): the harness is versioned
# IN THE REPO and does everything mechanically in a throwaway worktree —
# no manual edits, the working tree is never touched.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n580-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true; rm -rf "$WT"' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: the throwaway worktree must not build the
# whole dependency graph from scratch (the sandbox is disk-bound).
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (test name) -> prints the cargo test result line
  ( cd "$WT" && cargo test --test property_json_roundtrip "$1" 2>&1 | grep -E "test result" | head -1 ) || true
}

mutate() { # (python snippet) — apply one injection in the worktree
  python3 - "$WT/src/builtins/json.rs" << PYEOF
import sys
p = sys.argv[1]
src = open(p).read()
$1
open(p, "w").write(src)
PYEOF
}

echo "── M-DICT-SET: dict_set stores Unit instead of the given value ──"
mutate '
old = "    fields.insert(key, args[2].clone());"
new = "    fields.insert(key, Value::Unit); // MUTATION M-DICT-SET: value dropped"
assert old in src, "dict_set anchor not found"
src = src.replace(old, new, 1)
'
RESULT=$(run_test property_dict_store_consistency)
echo "  $RESULT"
echo "$RESULT" | grep -q "FAILED" && echo "  M-DICT-SET: VERIFIED (the new property catches the mutation)" || {
  echo "  M-DICT-SET: NOT VERIFIED"; exit 1; }

echo "── M-HAS-FIELD: has_field answers 0.0 for every present field ──"
mutate '
old = "                if i == segments.len() - 1 {\n                    return Ok(Value::Float(1.0));\n                }"
new = "                if i == segments.len() - 1 {\n                    return Ok(Value::Float(0.0)); // MUTATION M-HAS-FIELD: present answered absent\n                }"
assert old in src, "has_field anchor not found"
src = src.replace(old, new, 1)
'
RESULT=$(run_test property_has_field_exact_answers)
echo "  $RESULT"
echo "$RESULT" | grep -q "FAILED" && echo "  M-HAS-FIELD: VERIFIED (the new property catches the mutation)" || {
  echo "  M-HAS-FIELD: NOT VERIFIED"; exit 1; }

echo "── 2/2 VERIFIED ──"
