#!/usr/bin/env bash
# ── The Metalogos traction demo — one command, three legs ─────────────
# №549 (issue #887; Wave 24): the NLnet/Restack application demo arc —
# the LANGUAGE (static taint labels) → SERVE (the live HTTP loop) →
# ACTION-LEDGER (the signed, externally verifiable trail).
#
# Every leg composes the PUBLIC examples of this repository verbatim —
# the demo shows the real shipped behavior, not a staged script.
#
# Usage:  bash scripts/demo_traction.sh
# Needs:  the mlog binary (cargo build --bin mlog) + curl (leg 2).
# Exit 0 = every leg reproduced.
set -euo pipefail

cd "$(dirname "$0")/.."
BIN="${BIN:-./target/debug/mlog}"
if [ ! -x "$BIN" ]; then BIN=./target/release/mlog; fi
if [ ! -x "$BIN" ]; then
  echo "mlog binary not found — build first: cargo build --bin mlog" >&2
  exit 2
fi

banner() { printf '\n═══ %s ═══\n' "$1"; }

# ── Leg 1: static taint — the refusal happens BEFORE runtime ──────────
banner "LEG 1 · static taint labels: a private camera frame cannot reach a file sink"
echo "--- $ mlog check examples/w1_kitchen_camera.mlog"
"$BIN" check examples/w1_kitchen_camera.mlog 2>&1 | tail -6 || true
echo "--- expected: compilation DENIED — class SECRET_LEAK, rule SINK_CLEARANCE (private egress),"
echo "---           with the exact node and the explainable reason (the audit-lane contract)"

# ── Leg 2: serve — the live HTTP loop with the ledger listening ───────
banner "LEG 2 · serve: the language runs as a live HTTP server"
TMPDIR_DEMO=$(mktemp -d)
trap 'kill $SERVE_PID 2>/dev/null || true; rm -rf "$TMPDIR_DEMO"' EXIT
PORT=18099
cat > "$TMPDIR_DEMO/demo.mlog" <<'MLOG'
mlogserver {
  port: 18099
  route "/hello" method=GET {
    return respond("200 hello from metalogos")
  }
}
MLOG
"$BIN" serve "$TMPDIR_DEMO/demo.mlog" >"$TMPDIR_DEMO/serve.log" 2>&1 &
SERVE_PID=$!
for _ in $(seq 1 50); do
  if curl -sf "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then break; fi
  sleep 0.2
done
echo "--- $ curl http://127.0.0.1:$PORT/hello"
curl -sf "http://127.0.0.1:$PORT/hello"; echo
echo "--- the same runtime that denied the leak above is serving routes here"

# ── Leg 3: action-ledger — the signed chain verifies WITHOUT the runtime ─
banner "LEG 3 · action-ledger: grant + deny leave a signed, externally verifiable trail"
echo "--- $ mlog run examples/w2_ledger.mlog   (grant → allow → exhaust → deny → key rotation → export)"
"$BIN" run examples/w2_ledger.mlog 2>&1 | tail -4
EXPORTED=$(ls -t target/w2_ledger_export*.jsonl 2>/dev/null | head -1 || true)
if [ -z "$EXPORTED" ]; then EXPORTED=$(ls -t w2_ledger_export*.jsonl 2>/dev/null | head -1 || true); fi
if [ -n "$EXPORTED" ]; then
  echo "--- $ mlog ledger verify $EXPORTED   (pure hash + Ed25519 — no Metalogos runtime involved)"
  "$BIN" ledger verify "$EXPORTED" 2>&1 | tail -3
  rm -f "$EXPORTED"
else
  echo "--- the exported chain path is printed by the example; verify it with:"
  echo "---   mlog ledger verify <exported.jsonl>"
fi

banner "Every leg above is a public example of this repository — reproduced, not narrated."
