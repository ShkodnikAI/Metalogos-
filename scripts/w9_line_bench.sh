#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# ADR-0200 W9 (naryad №998): the LINE BENCH — measured numbers only.
#
# Every number this script prints comes from an actual run of the compiled
# binary in this invocation; the raw per-run lines land in
# docs/research/w9_bench_raw_run.txt and the summarized artifact in
# docs/research/w9-line-bench-<date>.md. No target/planned numbers exist
# anywhere in this lane (the §5 word: «numbers measured only»).
#
# Scenarios (the line's own surface, no model-quality claims):
#   1. attest verify latency      — 30 runs, the median/p95 (ms)
#   2. distill status latency     — 30 runs (a read-only op)
#   3. curriculum build latency   — 10 runs, a fresh DB copy each run
#   4. the certificate sizes      — cert_json bytes, weights blob bytes
#   5. train+save wall time       — 5 fresh programs (the save writes the
#                                   certificate + the ledger anchor in the
#                                   same transaction — the line's overhead
#                                   over a bare train)
#
# Prereqs: cargo (the release profile is built once), python3 (the timing
# aggregation + the SQLite reads). The machine context is recorded in the
# artifacts.
# ─────────────────────────────────────────────────────────────────────────────
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"   # repo root (the script lives in scripts/)
DATE="$(date +%Y-%m-%d)"
RAW="$ROOT/docs/research/w9_bench_raw_run.txt"
ART="$ROOT/docs/research/w9-line-bench-$DATE.md"

echo "== W9 line bench: building the release binary =="
(cd "$ROOT" && cargo build --release -q -p metalogos-server --bin mlog)
MLOG="$ROOT/target/release/mlog"
"$MLOG" --version || true

WORK="$(mktemp -d /tmp/w9_bench.XXXXXX)"
cp "$ROOT/examples/distill/line_demo.mlog" "$ROOT/examples/distill/held_out.tsv" "$WORK/"
cd "$WORK"

CPU="$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //')"
echo "machine: $CPU; profile: release; date: $DATE" > "$RAW"
echo "binary: $MLOG ($("$MLOG" --version 2>/dev/null || echo version-n/a))" >> "$RAW"

# The fixture: train + save once (the store every latency scenario reads).
"$MLOG" run line_demo.mlog >/dev/null 2>&1

# Guard: the fixture store MUST carry the attestation row — every number
# below is measured against the attested save path, not a refusal path.
python3 -I - <<'PY'
import sqlite3, sys
c = sqlite3.connect("line_demo.db")
row = c.execute(
    "SELECT length(cert_json), length(weights) FROM reflex_attestation a "
    "JOIN reflex_models m ON m.name = a.model ORDER BY a.seq DESC LIMIT 1"
).fetchone()
assert row and row[0] and row[1], f"the attested fixture is broken: {row}"
print(f"fixture ok: cert_json {row[0]}B, weights {row[1]}B")
PY

# ── 1. attest verify latency (30 runs) ───────────────────────────────
echo "== scenario 1: attest verify x30 ==" >> "$RAW"
for i in $(seq 1 30); do
  t0=$(date +%s%N)
  "$MLOG" attest verify line_demo.mlog --json >/dev/null 2>&1
  t1=$(date +%s%N)
  echo "verify_run $(( (t1 - t0) / 1000 ))us" >> "$RAW"
done

# ── 2. distill status latency (30 runs) ──────────────────────────────
echo "== scenario 2: distill status x30 ==" >> "$RAW"
for i in $(seq 1 30); do
  t0=$(date +%s%N)
  "$MLOG" distill status --pattern Classify --db line_demo.db --json >/dev/null 2>&1
  t1=$(date +%s%N)
  echo "status_run $(( (t1 - t0) / 1000 ))us" >> "$RAW"
done

# ── 3. curriculum build latency (10 runs, a fresh DB copy each) ──────
echo "== scenario 3: curriculum build x10 (fresh db copy each) ==" >> "$RAW"
for i in $(seq 1 10); do
  cp line_demo.db "build_$i.db"
  t0=$(date +%s%N)
  "$MLOG" curriculum build line_demo.mlog --held-out held_out.tsv --db "build_$i.db" --json >/dev/null 2>&1
  t1=$(date +%s%N)
  echo "build_run $(( (t1 - t0) / 1000 ))us" >> "$RAW"
done

# ── 4. the certificate sizes ─────────────────────────────────────────
echo "== scenario 4: sizes ==" >> "$RAW"
python3 -I - <<'PY' >> "$RAW"
import sqlite3
c = sqlite3.connect("line_demo.db")
cert = c.execute("SELECT length(cert_json) FROM reflex_attestation ORDER BY seq DESC LIMIT 1").fetchone()[0]
weights = c.execute("SELECT length(weights) FROM reflex_models WHERE name='Classify'").fetchone()[0]
print(f"cert_json_bytes {cert}")
print(f"weights_blob_bytes {weights}")
PY

# ── 5. train+save wall time (5 fresh programs) ───────────────────────
echo "== scenario 5: train+save x5 (fresh dir each) ==" >> "$RAW"
for i in $(seq 1 5); do
  mkdir -p "run_$i"
  cp line_demo.mlog "run_$i/"
  t0=$(date +%s%N)
  (cd "run_$i" && "$MLOG" run line_demo.mlog >/dev/null 2>&1)
  t1=$(date +%s%N)
  echo "train_save_run $(( (t1 - t0) / 1000 ))us" >> "$RAW"
done

# ── the artifact: aggregate the raw lines ────────────────────────────
python3 - "$RAW" "$ART" "$CPU" "$DATE" <<'PY'
import statistics, sys

raw, art, cpu, date = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
runs = {"verify_run": [], "status_run": [], "build_run": [], "train_save_run": []}
sizes = {}
for line in open(raw):
    parts = line.split()
    if not parts:
        continue
    if parts[0] in runs and len(parts) == 2 and parts[1].endswith("us"):
        runs[parts[0]].append(int(parts[1][:-2]))
    elif len(parts) == 2 and parts[0].endswith("_bytes") and parts[1].isdigit():
        sizes[parts[0]] = int(parts[1])

def ms(v):
    return v / 1000.0

def stat_line(name, vals):
    s = sorted(ms(v) for v in vals)
    med = statistics.median(s)
    p95 = s[int(len(s) * 0.95) - 1] if len(s) > 1 else s[0]
    return (f"| `{name}` | {len(vals)} | {min(s):.1f} | {med:.1f} | "
            f"{p95:.1f} | {max(s):.1f} |")

with open(art, "w") as f:
    f.write("# W9 line bench — the provable-lowering line's own overhead\n\n")
    f.write(f"Recorded: {date}. Machine: {cpu} (a shared sandbox — the timings "
            "are indicative, not contractual). Profile: release; the numbers "
            "come only from the raw run logged next to this file "
            "(`w9_bench_raw_run.txt`); no planned or target numbers exist in "
            "this lane (ADR-0200 §5: «numbers measured only»). The demo "
            "pattern is examples/distill/line_demo.mlog (the direct train on "
            "labeled rows — deterministic, no LLM).\n\n")
    f.write("## Latency (wall time per single CLI invocation, ms)\n\n")
    f.write("| scenario | runs | min | median | p95 | max |\n|---|---|---|---|---|---|\n")
    f.write(stat_line("mlog attest verify --json", runs["verify_run"]) + "\n")
    f.write(stat_line("mlog distill status --json", runs["status_run"]) + "\n")
    f.write(stat_line("mlog curriculum build --json (fresh db copy, held-out 4)",
                      runs["build_run"]) + "\n")
    f.write(stat_line("mlog run (train + save + inline verify, fresh store)",
                      runs["train_save_run"]) + "\n")
    f.write("\n## Sizes (the store's attestation row)\n\n")
    f.write("| artifact | bytes |\n|---|---|\n")
    f.write(f"| certificate (`cert_json`) | {sizes.get('cert_json_bytes')} |\n")
    f.write(f"| weights blob | {sizes.get('weights_blob_bytes')} |\n")
    f.write("\n## Reading\n\n")
    f.write("The certificate + the ledger anchor ride the save in the same "
            "transaction (the W1 artifact rule), so the `mlog run` scenario IS "
            "the attested save path — no separate overhead line exists to "
            "measure. The E-series (E1–E4, `docs/research/`) measured the "
            "model-training and calibration economics; this bench measures the "
            "line's verification surface on top of them. No before/after "
            "baseline exists for the CLI verbs themselves — the verbs are new "
            "surface introduced by this line (honest note: no baseline).\n")
print(f"artifact written: {art}")
PY

cd /
rm -rf "$WORK"
echo "raw run: $RAW"
