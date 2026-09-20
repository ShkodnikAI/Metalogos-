#!/usr/bin/env bash
# ── Naryad №409 (issue #554), Step A driver ──────────────────────────
# Runs every RSS-decomposition probe in its OWN process (clean VmRSS
# slopes) against the pinned №398/№404 fixture and assembles the
# decomposition table for ADR-0141 Addendum 4.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0

cargo build --release --example rss_decomposition >/dev/null 2>&1
BIN=target/release/examples/rss_decomposition

echo "── VM-serve peak-RSS decomposition (fixture: benches/fixtures/production_workload.mlog, 2344 lines) ──"
echo "probe          | N   | per-instance bytes | hwm delta kB"
$BIN --probe vm_new      --n 200
$BIN --probe load_nodb   --n 100
$BIN --probe load_db     --n 100
$BIN --probe pool_idle   --n 32
$BIN --probe reflex_model --n 100
$BIN --probe exec_loop   --n 50
