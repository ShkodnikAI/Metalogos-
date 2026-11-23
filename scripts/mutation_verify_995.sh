#!/usr/bin/env bash
# №995 (ADR-0200 W7) — the mutation contract for the audit lane: each
# mutation goes RED against the W7 leak-suite
# (tests/naryad_995_audit.rs), then the inverse-replace restores the
# file byte-identically. The restore is a file COPY, never git — the
# W4 incident taught that (uncommitted lane files must survive).
# The №990 form: the anchor must be unique, the RED is the proof the
# suite is not leaky, the byte-identical restore is the proof the sweep
# leaves no trace.
set -u
cd "$(dirname "$0")/.."
export PATH="${CARGO_BIN:-$PATH}"
SRC_ATTEST=src/nn/attest.rs
SRC_LOAD=src/builtins/reflex.rs
TEST="cargo test --test naryad_995_audit 2>&1"
BACKUP=$(mktemp)

run_mutation() {
  local name="$1" src="$2" old="$3" new="$4"
  echo "=== $name"
  cp "$src" "$BACKUP"
  OLD="$old" NEW="$new" SRCF="$src" python3 - <<'PYEOF'
import os, sys
p = os.environ["SRCF"]
s = open(p).read()
old, new = os.environ["OLD"], os.environ["NEW"]
assert s.count(old) == 1, "anchor not unique (%d): %r" % (s.count(old), old[:60])
open(p, "w").write(s.replace(old, new))
PYEOF
  if cargo test --test naryad_995_audit 2>&1 | grep -q "test result: FAILED"; then
    echo "RED — the suite caught the mutation"
  else
    echo "FAIL: the mutation went GREEN — the suite is leaky"
    cp "$BACKUP" "$src"
    exit 1
  fi
  cp "$BACKUP" "$src"
  if ! cmp -s "$BACKUP" "$src"; then
    echo "FAIL: the restore is not byte-identical"
    exit 1
  fi
  echo "restored byte-identically"
}

# M1: the teacher-policy gate disabled — the W7 policy enforcement gone
# (the (c) refusal matrix must catch it: the non-permissive refusal, the
# forged mismatch and the trace all live on this one machine truth).
run_mutation "M1: the teacher-policy gate disabled (always Ok)" \
  "$SRC_ATTEST" \
  'match check_teacher_block(cert_json) {' \
  'match Ok::<Option<String>, String>(None) {'

# M2: the STALE drift feed removed from the load lane — the hysteresis
# never sees the signal (the (b) wiring must catch it).
run_mutation "M2: the STALE lifecycle feed removed" \
  "$SRC_LOAD" \
  'if let Some(pattern) = stale_pattern {' \
  'if let Some(pattern) = Option::<String>::None {'

# M3: the rollback capture write dropped — the column stays empty and
# the ledger anchor never lands (the (a) capture must catch it).
run_mutation "M3: the rollback capture write dropped" \
  "$SRC_ATTEST" \
  'let rollback_json = attest.rollback_rule.clone().unwrap_or_default();' \
  'let rollback_json = String::new();'

# The final state: the suite green on the pristine files.
echo "=== the final suite run (pristine)"
if cargo test --test naryad_995_audit 2>&1 | grep -q "test result: ok. 10 passed"; then
  echo "OK — the W7 leak-suite green, the mutations all RED, the restores byte-identical"
else
  echo "FAIL: the pristine suite is not green (expected 10 passed)"
  exit 1
fi
rm -f "$BACKUP"
