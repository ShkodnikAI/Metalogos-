#!/usr/bin/env bash
# ── Naryad №423 (issue #583) — Fosved-class serve-soak probe ─────────
# Boots `mlog serve` on the seed app (VM DEFAULT per ADR-0171), then:
#   P0 static pre-flight (mlog check + audit)
#   P1 boot health + the loud VM-default startup line
#   P2 cron registration via /init
#   P3 the granted destructive path via /grantop (3 uses, quota metered)
#   P4 latency: 200 probes of /status (db + ledger_verify on every call)
#   P5 RSS growth inside the probe window
#   P6 a REAL native-cron minute-window tick (≤130 s wait)
#   P7 external ledger verification via the №415 CLI surface
#   P8 pool opt-in boot (METALOGOS_VM_POOL=1) with 0 unexpected discards
# Every audit-21.09-P0-2 criterion prints its own GREEN/RED line; the
# script exits 1 on ANY red (the nightly job failure IS the alert).
set -u

SEED_DIR="$(cd "$(dirname "$0")/serve_seed" && pwd)"
MLOG="${MLOG_BIN:-$SEED_DIR/../../../target/release/mlog}"
PORT=8091
LEDGER_REL="data/ledger/soak_latest.jsonl"
KEY="${SOAK_LEDGER_KEY:-$(openssl rand -hex 32)}"   # the nightly chain identity, NOT a secret
REPORT="${SOAK_REPORT:-/tmp/serve_soak_report.txt}"
pass=0; fail=0

say()  { echo "$1" | tee -a "$REPORT"; }
crit() { # crit "description" ok|fail tag
  if [ "$2" = "ok" ]; then say "  [$3] GREEN — $1"; pass=$((pass + 1))
  else say "  [$3] RED — $1"; fail=$((fail + 1)); fi
}

: > "$REPORT"
say "═══ Fosved-class serve-soak report — $(date -u +%Y-%m-%dT%H:%M:%SZ) ═══"
say "binary: $MLOG ($("$MLOG" --version 2>/dev/null || echo 'VERSION FAIL'))"

# ── P0. Static pre-flight ────────────────────────────────────────────
say "--- P0. static pre-flight"
if "$MLOG" check "$SEED_DIR/app.mlog" >"$REPORT.check" 2>&1; then
  crit "mlog check clean" ok P0
else
  crit "mlog check failed: $(tail -2 "$REPORT.check" | tr '\n' ' ')" fail P0
fi

# ── Fresh state for THIS run (the chain identity is nightly) ─────────
cd "$SEED_DIR"
mkdir -p data/ledger
rm -f data/soak.db data/soak.db-wal data/soak.db-shm data/ledger/*.jsonl

# ── P1/P2. Boot (VM default — NO METALOGOS_SERVE_BACKEND) + /init ────
say "--- P1. serve boot (VM default, loud startup line)"
METALOGOS_LEDGER_KEY="$KEY" "$MLOG" serve app.mlog >"$REPORT.serve1" 2>&1 &
SRV=$!
boot=0
for _ in $(seq 1 30); do
  sleep 1
  curl -fsS "http://127.0.0.1:$PORT/health" >/dev/null 2>&1 && { boot=1; break; }
done
crit "serve answered /health after boot" $([ "$boot" = 1 ] && echo ok || echo fail) P1
if grep -qi "backend: vm" "$REPORT.serve1"; then
  crit "the loud VM-default startup line is present" ok P1
else
  crit "no 'backend: vm' line in the startup log" fail P1
fi
if [ "$boot" != 1 ]; then
  kill -9 "$SRV" 2>/dev/null; wait "$SRV" 2>/dev/null
  say "═══ RESULT: $pass GREEN / $fail RED (aborted at boot) ═══"; exit 1
fi

say "--- P2. cron registration (/init)"
init_resp="$(curl -fsS "http://127.0.0.1:$PORT/init" 2>/dev/null || echo CURL_FAIL)"
crit "/init scheduled the cron job (got: $init_resp)" \
  "$(echo "$init_resp" | grep -q "scheduled" && echo ok || echo fail)" P2

# ── P3. The granted destructive path ─────────────────────────────────
say "--- P3. grant path (/grantop ×3 — scoped, metered, under grant)"
gok=0
for _ in 1 2 3; do
  curl -fsS "http://127.0.0.1:$PORT/grantop" 2>/dev/null | grep -q "grantop-ok" && gok=$((gok + 1))
  sleep 1
done
uses=$(grep -c "GRANT_USE" "$REPORT.serve1" || true)
crit "3/3 granted uses ok, $uses GRANT_USE ledger events in the log" \
  "$([ "$gok" = 3 ] && [ "$uses" -ge 3 ] && echo ok || echo fail)" P3

# ── P4/P5. Latency + RSS ─────────────────────────────────────────────
say "--- P4. latency: 200 probes of /status"
rm -f /tmp/soak_rss_samples; touch /tmp/soak_rss_samples
(
  while kill -0 "$SRV" 2>/dev/null; do
    ps -o rss= -p "$SRV" | tr -d ' ' >> /tmp/soak_rss_samples
    sleep 2
  done
) &
SAMPLER=$!
times=""
for _ in $(seq 1 200); do
  t="$(curl -fsS -o /dev/null -w '%{time_total}' "http://127.0.0.1:$PORT/status" 2>/dev/null)" && times="$times $t"
done
cnt="$(echo $times | wc -w)"
p95="$(echo $times | tr ' ' '\n' | sort -n | awk '{a[NR]=$1} END {if (NR > 0) printf "%.4f", a[int(NR * 0.95 + 0.999)]}')"
mx="$(echo $times | tr ' ' '\n' | sort -n | tail -1)"
say "    probes_ok=$cnt/200  p95=${p95}s  max=${mx}s (recorded; the trend is judged across nightly runs)"
crit "at least 190/200 probes succeeded" "$([ "$cnt" -ge 190 ] && echo ok || echo fail)" P4

rss_first="$(head -1 /tmp/soak_rss_samples 2>/dev/null || echo 0)"
rss_max="$(sort -n /tmp/soak_rss_samples 2>/dev/null | tail -1)"
growth=$(( rss_max > rss_first ? rss_max - rss_first : 0 ))
say "    rss_kb: first=$rss_first max=$rss_max growth=$growth"
crit "RSS growth inside the window ≤ 20 MB ($growth KB)" "$([ "$growth" -le 20480 ] && echo ok || echo fail)" P5

# ── P6. A REAL native-cron minute-window tick ────────────────────────
say "--- P6. native cron tick (≤130 s)"
fired=0
for _ in $(seq 1 26); do
  grep -q "SOAK_TICK done" "$REPORT.serve1" && { fired=1; break; }
  sleep 5
done
crit "≥1 real minute-window tick executed (dedup/catch-up/payload per №418)" \
  "$([ "$fired" = 1 ] && echo ok || echo fail)" P6

# ── P7. External ledger verification (№415 CLI) ──────────────────────
say "--- P7. ledger verify (external verifier, no runtime)"
if [ -f "$LEDGER_REL" ]; then
  if "$MLOG" ledger verify --json "$LEDGER_REL" >"$REPORT.verify" 2>&1; then
    recs="$(grep -o '"records":[0-9]*' "$REPORT.verify" | head -1 | cut -d: -f2)"
    crit "ledger verify ok ($recs records exported)" ok P7
  else
    crit "ledger verify FAILED: $(tail -2 "$REPORT.verify" | tr '\n' ' ')" fail P7
  fi
else
  crit "the exported ledger file is missing: $LEDGER_REL" fail P7
fi
st="$(curl -fsS "http://127.0.0.1:$PORT/status" 2>/dev/null || echo CURL_FAIL)"
crit "/status verdict: $st" "$(echo "$st" | grep -q "ledger-ok" && echo ok || echo fail)" P7

kill -9 "$SRV" 2>/dev/null; wait "$SRV" 2>/dev/null
kill "$SAMPLER" 2>/dev/null

# ── P8. Pool opt-in boot ─────────────────────────────────────────────
say "--- P8. pool opt-in boot (METALOGOS_VM_POOL=1)"
METALOGOS_VM_POOL=1 METALOGOS_LEDGER_KEY="$KEY" "$MLOG" serve app.mlog >"$REPORT.serve2" 2>&1 &
SRV2=$!
boot2=0
for _ in $(seq 1 30); do
  sleep 1
  curl -fsS "http://127.0.0.1:$PORT/health" >/dev/null 2>&1 && { boot2=1; break; }
done
discards="$(grep -ci "discard" "$REPORT.serve2" || true)"
crit "pool boot healthy, 0 unexpected discards (found $discards)" \
  "$([ "$boot2" = 1 ] && [ "$discards" = "0" ] && echo ok || echo fail)" P8
kill -9 "$SRV2" 2>/dev/null; wait "$SRV2" 2>/dev/null

say "═══ RESULT: $pass GREEN / $fail RED ═══"
[ "$fail" = 0 ]
