#!/bin/bash
# Red-team pass driver: runs check (both backends) + run for every probe,
# writes verbatim transcripts and a per-probe summary line.
RT=/home/z/my-project/metalogos-work/rt-978
cd "$RT" || exit 1
RES=pocs/check_results.txt
: > "$RES"
for f in pocs/p*.mlog; do
  base=$(basename "$f" .mlog)
  t="pocs/${base}_transcript.txt"
  cout=$(./mlog-candidate check "$f" 2>&1); cexit=$?
  rout=$(./mlog-candidate check --reference "$f" 2>&1); rexit=$?
  uout=$(METALOGOS_ALLOW_EXEC=1 METALOGOS_HTTP_ALLOW_PRIVATE=1 timeout 20 ./mlog-candidate run "$f" 2>&1); uexit=$?
  {
    echo "===== PROBE: $base ====="
    echo "----- source ($f) -----"
    cat "$f"
    echo "----- check (VM backend) -----"
    echo "$cout"
    echo "[check exit=$cexit]"
    echo "----- check --reference (reference backend) -----"
    echo "$rout"
    echo "[check --reference exit=$rexit]"
    echo "----- run (env knobs: METALOGOS_ALLOW_EXEC=1 METALOGOS_HTTP_ALLOW_PRIVATE=1, timeout 20s) -----"
    echo "$uout"
    echo "[run exit=$uexit]"
    echo "===== END $base ====="
  } > "$t" 2>&1
  ccls=$(echo "$cout" | grep -o '\[[A-Z_]*\]' | sort -u | paste -sd, -); [ -z "$ccls" ] && ccls="none"
  rcls=$(echo "$rout" | grep -o '\[[A-Z_]*\]' | sort -u | paste -sd, -); [ -z "$rcls" ] && rcls="none"
  cstat=$([ $cexit -eq 0 ] && echo "PASS" || echo "REFUSE")
  rstat=$([ $rexit -eq 0 ] && echo "PASS" || echo "REFUSE")
  ufirst=$(echo "$uout" | head -2 | paste -sd' | ' -); [ -z "$ufirst" ] && ufirst="(no output)"
  echo "$base | check=$cstat classes=[$ccls] | ref=$rstat classes=[$rcls] | run: $ufirst" >> "$RES"
done
echo "driver done"; wc -l "$RES"
