#!/bin/bash
# Наряд №51 Блок 2: Concurrency measurement (fixed timing)
set -e

METALOGOS_BIN="${1:-./target/debug/mlog}"
MLOG_FILE="${2:-./examples/p51_concurrency.mlog}"
HELPER_PORT=10098
SERVER_PORT=10099

cleanup() {
    kill $HELPER_PID 2>/dev/null || true
    kill $SERVER_PID 2>/dev/null || true
    wait 2>/dev/null || true
}
trap cleanup EXIT

echo "=== Narjad 51 Concurrency Measurement ==="

# Start helper
python3 ./scripts/helper_server.py $HELPER_PORT &
HELPER_PID=$!
sleep 0.5

# Start mlog serve, wait for "listening" line
METALOGOS_WORKERS=4 $METALOGOS_BIN serve $MLOG_FILE > /tmp/mlog-serve.log 2>&1 &
SERVER_PID=$!

# Wait up to 10s for server to be ready
for i in $(seq 1 20); do
    if curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$SERVER_PORT/fast" 2>/dev/null | grep -q "200"; then
        echo "[OK] Server ready after ${i}x0.5s"
        break
    fi
    sleep 0.5
done

echo "Workers: $(grep 'worker thread' /tmp/mlog-serve.log)"

# Test 1: two concurrent /slow
echo ""
echo "[Test 1] Two concurrent /slow..."
T1=$(mktemp) T2=$(mktemp)
START=$(date +%s%N)
curl -s -o /dev/null -w "%{time_total}" "http://127.0.0.1:$SERVER_PORT/slow" > $T1 &
C1=$!
curl -s -o /dev/null -w "%{time_total}" "http://127.0.0.1:$SERVER_PORT/slow" > $T2 &
C2=$!
wait $C1 $C2
END=$(date +%s%N)
WALL=$(( (END - START) / 1000000 ))
SLOW1=$(cat $T1)
SLOW2=$(cat $T2)
echo "  /slow #1: ${SLOW1}s"
echo "  /slow #2: ${SLOW2}s"
echo "  Wall time: ${WALL}ms"
rm $T1 $T2

# Test 2: /fast during /slow
echo ""
echo "[Test 2] /fast during /slow..."
curl -s -o /dev/null "http://127.0.0.1:$SERVER_PORT/slow" &
SPID=$!
sleep 1
T3=$(mktemp)
curl -s -o /dev/null -w "%{time_total}" "http://127.0.0.1:$SERVER_PORT/fast" > $T3
FAST=$(cat $T3)
echo "  /fast during /slow: ${FAST}s"
rm $T3
wait $SPID 2>/dev/null || true

# Test 3: isolation
echo ""
echo "[Test 3] Request isolation..."
R1=$(curl -s "http://127.0.0.1:$SERVER_PORT/echo?x=alice")
R2=$(curl -s "http://127.0.0.1:$SERVER_PORT/echo?x=bob")
echo "  /echo?x=alice => '$R1'"
echo "  /echo?x=bob   => '$R2'"
if [ "$R1" = "echo=alice" ] && [ "$R2" = "echo=bob" ]; then
    echo "  ISOLATION: PASS"
else
    echo "  ISOLATION: FAIL"
fi

echo ""
echo "=== Done ==="
