#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# ADR-0200 W9 (naryad №998, the dispatch wave В104 — W-AH): the LINE DEMO.
#
# The provable tier-lowering line's user path in one command, from scratch
# (a fresh temp dir every run — no state carries over):
#
#   bash examples/distill/run_line_demo.sh
#
# Steps (each prints a [DEMO i/7] machine line; expected refusals print
# ok=false(EXPECTED) — the gates are the spectacle, not a failure):
#   1. mlog run                — the direct train + the save (the certificate
#                                + the ledger anchor land in one transaction)
#   2. mlog attest verify      — ok: provenance-consistent (NOT quality: the
#                                certificate attests origin and consistency,
#                                never model quality — ADR-0200 §6.1)
#   3. the spec drift          — the .mlog declaration moves (seed 42→43):
#                                verify refuses with [ATTESTATION_STALE]
#   4. mlog curriculum build   — the held-out rows load as the refutation
#                                instrument, never training data (W4)
#   5. mlog distill freeze     — the operator lock; status shows FROZEN
#   6. curriculum build again  — the frozen interlock refuses (W5)
#   7. mlog distill enable-differential — the demo pattern is unfiltered
#                                (no filter word): the enablement gate refuses
#                                with [ATTESTATION_ENABLEMENT_UNFILTERED] (W6),
#                                zero writes. The gate is the spectacle.
#   +  mlog distill unfreeze   — the only exit for a frozen lane (decision 6)
#
# The demo shows PROVENANCE and CONSISTENCY — it never shows or claims
# model quality. The binary: MLOG_BIN=path/to/mlog if set, else
# `cargo run -q --bin mlog --` from the repo root (from scratch = it builds).
# ─────────────────────────────────────────────────────────────────────────────
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"

if [ -n "${MLOG_BIN:-}" ]; then
  MLOG="$MLOG_BIN"
else
  # From scratch: build the debug binary once if it is not there yet
  # (a `cargo run` from the temp workdir would find no workspace).
  MLOG="$ROOT/target/debug/mlog"
  if [ ! -x "$MLOG" ]; then
    echo "== building the mlog binary (from scratch) =="
    (cd "$ROOT" && cargo build -q --bin mlog) || exit 1
  fi
fi

WORK="$(mktemp -d /tmp/line_demo.XXXXXX)"
cp "$HERE/line_demo.mlog" "$HERE/held_out.tsv" "$WORK/"
RESEARCH="$HERE/demo_research"
cd "$WORK" || exit 1

step=0
ok_line() {  # step_line <i> <name> <ok> <detail>
  printf '[DEMO %s/7] %-28s ok=%s  %s\n' "$1" "$2" "$3" "$4"
}

echo "== ADR-0200 provable-lowering line demo (workdir: $WORK) =="

# 1. run: train + save + the inline attest_verify builtin
if out=$($MLOG run line_demo.mlog 2>&1); then
  ok_line 1 "mlog run" true "$(printf '%s' "$out" | grep -o 'ok=true;level=[a-z]*' | head -1)"
else
  ok_line 1 "mlog run" false "UNEXPECTED: $out"; exit 1
fi

# 2. attest verify: provenance-consistent
if out=$($MLOG attest verify line_demo.mlog --json 2>&1); then
  ok_line 2 "attest verify" true "$(printf '%s' "$out" | tail -1)"
else
  ok_line 2 "attest verify" false "UNEXPECTED: $out"; exit 1
fi

# 3. the spec drift: the declaration moved under a saved certificate
sed 's/seed: 42/seed: 43/' line_demo.mlog > drifted.mlog
if $MLOG attest verify drifted.mlog --json >/dev/null 2>&1; then
  ok_line 3 "spec drift refusal" false "UNEXPECTED: verify passed on a drifted spec"; exit 1
else
  reason=$($MLOG attest verify drifted.mlog 2>&1 | grep -o '\[ATTESTATION_STALE\].*' | head -1)
  ok_line 3 "spec drift refusal" "false(EXPECTED)" "${reason:-[ATTESTATION_STALE]}"
fi

# 4. curriculum build: the held-out load + the refutation verdict
if out=$($MLOG curriculum build line_demo.mlog --held-out held_out.tsv --json 2>&1); then
  ok_line 4 "curriculum build" true "$(printf '%s' "$out" | tail -1)"
else
  ok_line 4 "curriculum build" false "UNEXPECTED: $out"; exit 1
fi

# 5. the operator freeze + the read-only status
if out=$($MLOG distill freeze --pattern Classify --db line_demo.db --json 2>&1); then
  ok_line 5 "distill freeze" true "$(printf '%s' "$out" | tail -1)"
else
  ok_line 5 "distill freeze" false "UNEXPECTED: $out"; exit 1
fi
st=$($MLOG distill status --pattern Classify --db line_demo.db --json 2>&1 | tail -1)
echo "        status: $st"

# 6. the frozen interlock: a distill move refuses
if ! out=$($MLOG curriculum build line_demo.mlog --held-out held_out.tsv --json 2>&1); then
  ok_line 6 "frozen interlock" "false(EXPECTED)" "$(printf '%s' "$out" | tail -1)"
else
  ok_line 6 "frozen interlock" false "UNEXPECTED: the build ran on a frozen lane"; exit 1
fi

# + the only exit for a frozen lane: the explicit external unfreeze
if out=$($MLOG distill unfreeze --pattern Classify --db line_demo.db --json 2>&1); then
  echo "        unfreeze: $(printf '%s' "$out" | tail -1)"
else
  echo "        unfreeze: UNEXPECTED FAILURE: $out"; exit 1
fi

# 7. the enablement gate: the demo pattern is unfiltered → refuse, zero writes
if ! out=$($MLOG distill enable-differential --pattern Classify --db line_demo.db \
     --research-dir "$RESEARCH" --json 2>&1); then
  ok_line 7 "enable-differential" "false(EXPECTED)" "$(printf '%s' "$out" | tail -1)"
else
  ok_line 7 "enable-differential" false "UNEXPECTED: an unfiltered pattern was enabled"; exit 1
fi

echo "DEMO COMPLETE — provenance verified, gates shown. (tamper-evident, not tamper-proof;"
echo "  ok=true is provenance consistency, never model quality — ADR-0200 §6.1/§6.3)"
rm -rf "$WORK"
