#!/usr/bin/env bash
# №990 (ADR-0200 W5) — the mutation contract for the lifecycle machine:
# each mutation goes RED against the W5 leak-suite
# (tests/naryad_990_states.rs), then the inverse-replace restores the
# file byte-identically. The restore is a file COPY, never git — the
# W4 incident taught that (uncommitted lane files must survive).
set -u
cd "$(dirname "$0")/.."
export PATH="${CARGO_BIN:-$PATH}"
SRC=src/distill_hub.rs
TEST="cargo test --test naryad_990_states 2>&1"
BACKUP=$(mktemp)
cp "$SRC" "$BACKUP"

restore() { cp "$BACKUP" "$SRC"; }
check_restored() {
  if ! cmp -s "$BACKUP" "$SRC"; then
    echo "FAIL: the restore is not byte-identical"
    exit 1
  fi
}

run_mutation() {
  local name="$1" old="$2" new="$3"
  echo "=== $name"
  OLD="$old" NEW="$new" python3 - "$SRC" <<'PYEOF'
import os, sys
p = sys.argv[1]
s = open(p).read()
old, new = os.environ["OLD"], os.environ["NEW"]
assert s.count(old) == 1, "anchor not unique (%d): %r" % (s.count(old), old[:60])
open(p, "w").write(s.replace(old, new))
PYEOF
  if cargo test --test naryad_990_states 2>&1 | grep -q "test result: FAILED"; then
    echo "RED — the suite caught the mutation"
  else
    echo "FAIL: the mutation went GREEN — the suite is leaky"
    restore
    exit 1
  fi
  restore
  check_restored
  echo "restored byte-identically"
}

# M1: the 24h bound dropped — the cooldown clock never withholds.
run_mutation "M1: the 24h cooldown bound dropped" \
'if now.saturating_sub(t) < cooldown {' \
'if false && now.saturating_sub(t) < cooldown {'

# M2: the re-freeze lock ignored — decision 6's window rule gone.
run_mutation "M2: the re-freeze window lock ignored" \
'if lc.refroze_in_window {
        trace_distill(' \
'if lc.refroze_in_window && false {
        trace_distill('

# M3: the operator lock ignored — an operator freeze would auto-release.
run_mutation "M3: the operator lock ignored" \
'if lc.operator_lock {
        trace_distill(' \
'if lc.operator_lock && false {
        trace_distill('

# The final state: the suite green on the pristine file.
echo "=== the final suite run (pristine)"
if cargo test --test naryad_990_states 2>&1 | grep -q "test result: ok. 8 passed"; then
  echo "OK — the W5 leak-suite green, the mutations all RED, the restore byte-identical"
else
  echo "FAIL: the pristine suite is not green"
  exit 1
fi
rm -f "$BACKUP"
