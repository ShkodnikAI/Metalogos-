#!/usr/bin/env bash
# №983 mutation verification: the ONLY hand this naryad adds is the
# machine-id split itself. The mutation restores the reason in the
# format string of `builtin_quarantine_write` (the old audit-text body's
# echo) and watches the №983 format pins redden. The №980 pins (flipped
# by №983) survive the mutation BY DESIGN — with the hand removed the
# echoed reason arrives bottom again, which is exactly the №977 hole —
# so the format pins are the discriminating assertions (the header of
# tests/naryad_983_quarantine_machine_id.rs).
set -u
cd "$(dirname "$0")/.."
trap 'git checkout -- src/ 2>/dev/null' EXIT

restore() { git checkout -- "$1"; touch "$1"; sleep 1; }
check_byte_identical() {
    if [ -n "$(git status --porcelain "$1")" ]; then
        echo "RESTORE FAILED for $1"; exit 1
    fi
}

# ── M1: the reason re-enters the RETURN (the echo is back) ──
cat > /tmp/m983_1.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
old = '''    Ok(Value::String(id))
}'''
new = '''    Ok(Value::String(format!(
        "[QUARANTINE_EGRESS] quarantine record {id} (reason: {reason})"
    )))
}'''
assert old in s, "M1: the return anchor not found"
open(p, 'w').write(s.replace(old, new, 1))
PYEOF
python3 /tmp/m983_1.py src/builtins/consent.rs
if cargo test --test naryad_983_quarantine_machine_id n983_tw_the_receipt_is_a_machine_id 2>&1 | grep -q "FAILED"; then
    echo "M1 reason-in-return: RED as expected (the id split broke)"
else
    echo "M1 reason-in-return: NOT RED — the pin does not bite"; exit 1
fi
restore src/builtins/consent.rs; check_byte_identical src/builtins/consent.rs

echo "MUTATION VERIFY 983: PASS (M1 red, restore byte-identical)"
