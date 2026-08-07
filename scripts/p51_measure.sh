#!/bin/bash
cd /home/z/my-project/Metalogos-
export PATH="$HOME/.cargo/bin:$PATH"

pkill -9 -f helper_server 2>/dev/null; pkill -9 -f "mlog serve" 2>/dev/null; pkill -9 -f curl 2>/dev/null; sleep 0.5

python3 scripts/helper_server.py 10098 &>/dev/null &
sleep 0.3
METALOGOS_WORKERS=4 ./target/debug/mlog serve examples/p51_concurrency.mlog &>/tmp/s.log &
sleep 3

echo "=== T1: two concurrent /slow ==="
START=$(date +%s%N)
curl -so/dev/null -w"s1=%{time_total}s\n" http://127.0.0.1:10099/slow &
curl -so/dev/null -w"s2=%{time_total}s\n" http://127.0.0.1:10099/slow &
wait
END=$(date +%s%N)
echo "wall=$(( (END-START)/1000000 ))ms"

echo ""
echo "=== T2: /fast during /slow ==="
curl -so/dev/null http://127.0.0.1:10099/slow &
sleep 1
curl -so/dev/null -w"fast=%{time_total}s\n" http://127.0.0.1:10099/fast
wait

echo ""
echo "panics=$(grep -c panic /tmp/s.log 2>/dev/null || echo 0)"
pkill -9 -f helper_server 2>/dev/null; pkill -9 -f "mlog serve" 2>/dev/null; pkill -9 -f curl 2>/dev/null
