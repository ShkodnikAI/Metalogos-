#!/usr/bin/env bash
# ── Benchmark run wrapper (naryad #398, issue #501) ────────────────────
# The mechanical enforcement of docs/benchmark-protocol.md for the Stage 4
# benchmark:
#   1. fixed run contract: ONE command (below), never edited between runs;
#   2. divisor policy: refuses to run without BENCH_DIVISOR declared up
#      front, and prints RAW + DIVISOR + NORMALIZED in the final line;
#   3. run-tree bookkeeping: appends "parent -> child" to
#      docs/research/bench-tree.txt (the report's tree form).
#
# Usage:
#   BENCH_DIVISOR=14 BENCH_PARENT=root BENCH_NODE=variant-a \
#     scripts/bench_run.sh [extra cargo args...]
#
# Env (all three REQUIRED — fail-closed, no silent defaults):
#   BENCH_DIVISOR — positive integer the raw numbers are normalized by
#                   (declared BEFORE the run; e.g. corpus route count = 14)
#   BENCH_PARENT  — the tree node this run branches from (e.g. "root")
#   BENCH_NODE    — this run's tree node name (e.g. "variant-a")
set -euo pipefail

: "${BENCH_DIVISOR:?BENCH_DIVISOR must be declared before the run (divisor policy, rule 2)}"
: "${BENCH_PARENT:?BENCH_PARENT must name the tree node this run branches from (rule 5)}"
: "${BENCH_NODE:?BENCH_NODE must name the tree node of this run (rule 5)}"

case "$BENCH_DIVISOR" in
  ''|*[!0-9]*|0) echo "refusing: BENCH_DIVISOR must be a positive integer, got '$BENCH_DIVISOR'" >&2; exit 2 ;;
esac

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
LOG="docs/research/bench-tree.txt"
OUT="target/stage4_bench_${BENCH_NODE}.json"

echo "[bench_run] fixed run contract (rule 1): cargo bench --bench stage4_benchmark"
cargo bench --bench stage4_benchmark "$@" | tee "$OUT"

# ── Extract the raw numbers from the bench JSON and print the mandatory
#    RAW | DIVISOR | NORMALIZED line (rule 2). The bench prints one JSON
#    object per backend on stdout; the parser below is python3 (stdlib).
python3 - "$OUT" "$BENCH_DIVISOR" <<'PY'
import json, os, sys

out_path, divisor = sys.argv[1], int(sys.argv[2])
# The bench children print their JSON to the PARENT, which assembles the
# combined report at target/bench-reports/stage4_benchmark_report.json
# (interpreter/vm objects, each with a cycle summary). Prefer the report
# file; fall back to the captured stdout stream.
candidates = [
    "target/bench-reports/stage4_benchmark_report.json",
    out_path,
]
raw = {}
for path in candidates:
    if not os.path.exists(path):
        continue
    text = open(path).read()
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            # the combined report is one pretty-printed JSON document
            try:
                obj = json.loads(text)
            except json.JSONDecodeError:
                continue
        backend = obj.get("backend")
        cycle = obj.get("cycle") or {}
        # The per-cycle MEAN total (µs) over all rounds is the summary both
        # variants are compared by; per-route samples live in obj["routes"].
        total = cycle.get("mean_us")
        if backend and total is not None:
            raw[backend] = float(total)
    if not raw:
        # The combined report is one pretty-printed JSON document with
        # per-backend objects: {"interpreter": {"cycle": …}, "vm": {…}}.
        try:
            doc = json.loads(text)
        except json.JSONDecodeError:
            continue
        if isinstance(doc, dict):
            for backend in ("interpreter", "vm"):
                entry = doc.get(backend)
                if isinstance(entry, dict):
                    total = (entry.get("cycle") or {}).get("mean_us")
                    if total is not None:
                        raw[backend] = float(total)
    if raw:
        break

if not raw:
    print("[bench_run] WARNING: no per-backend cycle summary found — raw numbers missing, treat this run as a REPAIR (rule 3)")
    sys.exit(0)

for backend, total in sorted(raw.items()):
    print(f"[bench_run] RESULT backend={backend} RAW_CYCLE_MEAN_US={total:.0f} | DIVISOR={divisor} | NORMALIZED_US_PER_UNIT={total / divisor:.1f}")
PY

printf 'parent -> child: %s -> %s (divisor=%s)\n' "$BENCH_PARENT" "$BENCH_NODE" "$BENCH_DIVISOR" >> "$LOG"
echo "[bench_run] tree appended: $BENCH_PARENT -> $BENCH_NODE (log: $LOG)"
