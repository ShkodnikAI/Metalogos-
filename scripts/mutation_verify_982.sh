#!/usr/bin/env bash
# №982 mutation verification (the mutation_verify_981.sh form): remove
# EACH lane's sidecar-record hand, watch EXACTLY that lane's trusted-lift
# pin redden, restore byte-identical. Two mutations (the two record hands
# №982 adds):
#   M1 interpreter/execution.rs (the TW arm-head record)
#      → n982_tw_file_roundtrip_the_trusted_own_config_stays_green REDS
#        (the trusted read-back falls to the №325 miss default → refuses)
#   M2 src/vm.rs (the VM dispatch-gate record)
#      → n982_vm_file_roundtrip_the_trusted_own_config_stays_green REDS
# The mem/session untrusted pins are PRESERVATION pins (the entrance
# closure predates №982 and does not redden under these mutations — the
# honest mapping, documented in the pin file and the PR body).
set -u
cd "$(dirname "$0")/.."
trap 'git checkout -- src/ 2>/dev/null' EXIT

restore() { git checkout -- "$1"; touch "$1"; sleep 1; }
check_byte_identical() {
    if [ -n "$(git status --porcelain "$1")" ]; then
        echo "RESTORE FAILED for $1"; exit 1
    fi
}

# ── M1: the TW arm-head record ──
cat > /tmp/m982_1.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """                if let Some(labels) = &gate_arg_labels {
                    crate::runtime_labels::record_store_write(name, &eval_args, labels);
                }
"""
assert block in s, "M1: the TW record block not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
python3 /tmp/m982_1.py src/interpreter/execution.rs
if cargo test --test naryad_982_storage_label_sidecar n982_tw_file_roundtrip_the_trusted 2>&1 | grep -q "FAILED"; then
    echo "M1 TW record: RED as expected (the trusted lift died)"
else
    echo "M1 TW record: NOT RED — the pin does not bite"; exit 1
fi
restore src/interpreter/execution.rs; check_byte_identical src/interpreter/execution.rs

# ── M2: the VM dispatch-gate record ──
cat > /tmp/m982_2.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """        crate::runtime_labels::record_store_write(name, args, arg_labels);
"""
assert block in s, "M2: the VM record call not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
python3 /tmp/m982_2.py src/vm.rs
if cargo test --test naryad_982_storage_label_sidecar n982_vm_file_roundtrip_the_trusted 2>&1 | grep -q "FAILED"; then
    echo "M2 VM record: RED as expected (the trusted lift died)"
else
    echo "M2 VM record: NOT RED — the pin does not bite"; exit 1
fi
restore src/vm.rs; check_byte_identical src/vm.rs

echo "MUTATION VERIFY 982: PASS (M1-M2 red, restores byte-identical)"
