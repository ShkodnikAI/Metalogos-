#!/bin/bash
# Red-team probe runner — black-box observation only.
# Usage: bash run_probes.sh pNN... (default: all p*.mlog in rt-977 root)
RT=/home/z/my-project/metalogos-work/rt-977
BIN=$RT/mlog-candidate
mkdir -p $RT/pocs
cd $RT
probes="$@"
if [ -z "$probes" ]; then
  probes=$(ls p[0-9][0-9]_*.mlog | sort)
fi
for f in $probes; do
  base=$(basename $f .mlog)
  out=$RT/pocs/${base}_transcript.txt
  {
    echo "### PROBE: $base.mlog"
    echo "### COMMAND: $BIN check $base.mlog"
    echo "--- check (default/VM backend) ---"
    $BIN check $base.mlog 2>&1
    echo "--- check exit code: $? ---"
    echo "### COMMAND: $BIN check --reference $base.mlog"
    echo "--- check (reference backend) ---"
    $BIN check --reference $base.mlog 2>&1
    echo "--- check --reference exit code: $? ---"
    echo "### COMMAND: SP=mocksecret $BIN run $base.mlog"
    echo "--- run (SP=mocksecret, mock value, never a real credential) ---"
    SP=mocksecret timeout 20 $BIN run $base.mlog 2>&1
    echo "--- run exit code: $? ---"
  } > $out 2>&1
  echo "=== $base ==="
  cat $out
done
