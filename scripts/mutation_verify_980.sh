#!/usr/bin/env bash
# №980 mutation verification (the mutation_verify_957.sh form): remove
# EACH lane's echo hand, watch EXACTLY that lane's pins redden, restore
# byte-identical. Four mutations:
#   M1 semantic.rs            → the static-lane pins (R2 + secret e2e)
#   M2 interpreter/labels.rs  → the TW runtime pin (reason echo)
#   M3 vm.rs                  → the VM runtime pin (reason echo)
#   M4 secret_label.rs        → the typed-lane unit pin
# PASS before the push — the dispatch rule («снять руку в одном —
# краснеет ровно этот пин, а не „что-нибудь"»).
# NOTE: run against a COMMITTED tree — the restore is `git checkout --`,
# which answers to the index; an uncommitted working tree would be wiped.
set -u
cd "$(dirname "$0")/.."

# On ANY exit (including a mid-mutation failure) restore the src tree:
# a mutation left applied would poison every later run (the M2 lesson —
# the script once exited between mutate and restore).
trap 'git checkout -- src/ 2>/dev/null' EXIT

mutate() { # $1 file, $2 python-replace-file
    python3 "$2" "$1" apply
}

restore() { # $1 file
    git checkout -- "$1"
    # cargo keys builds off mtime — the mutate/restore pair can land in
    # the same second and the mutated lib survives as «fresh». Force the
    # rebuild with an explicit bump.
    touch "$1"
    sleep 1
}

check_byte_identical() {
    if [ -n "$(git status --porcelain "$1")" ]; then
        echo "RESTORE FAILED for $1"; exit 1
    fi
}

# ── M1: the static lattice hand ──
cat > /tmp/m1.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """            if let Some(i) = crate::sink_receipt::receipt_echo_arg(name) {
                if let Some(a) = args.get(i) {
                    return expr_label(a, env);
                }
            }
"""
assert block in s, "M1: the semantic echo hand not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
mutate src/semantic.rs /tmp/m1.py
if cargo test --test naryad_980_receipt_echo 2>&1 | grep -q "test result: FAILED"; then
    echo "M1 semantic: RED as expected"
else
    echo "M1 semantic: NOT RED — the static pin does not bite"; exit 1
fi
restore src/semantic.rs; check_byte_identical src/semantic.rs
cargo test --test naryad_980_receipt_echo 2>&1 | grep -q "test result: ok" \
    || { echo "M1 restore broke the green"; exit 1; }

# ── M2: the TW runtime hand ──
cat > /tmp/m2.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """        if let Some(i) = crate::sink_receipt::receipt_echo_arg(name) {
            if let Some(a) = args.get(i) {
                return self.label_of_expr_env(a, depth, overlay);
            }
        }
"""
assert block in s, "M2: the TW echo hand not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
mutate src/interpreter/labels.rs /tmp/m2.py
if cargo test --test naryad_980_receipt_echo_runtime n980_tw_runtime_the_reason_echo 2>&1 | grep -q "test result: FAILED"; then
    echo "M2 TW runtime: RED as expected"
else
    echo "M2 TW runtime: NOT RED — the TW pin does not bite"; exit 1
fi
restore src/interpreter/labels.rs; check_byte_identical src/interpreter/labels.rs

# ── M3: the VM runtime hand ──
cat > /tmp/m3.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """                if let Some(i) = crate::sink_receipt::receipt_echo_arg(name) {
                    if let Some(l) = arg_labels.get(i) {
                        return l.clone();
                    }
                }
"""
assert block in s, "M3: the VM echo hand not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
mutate src/vm.rs /tmp/m3.py
if cargo test --test naryad_980_receipt_echo_runtime n980_vm_runtime_the_reason_echo 2>&1 | grep -q "test result: FAILED"; then
    echo "M3 VM runtime: RED as expected"
else
    echo "M3 VM runtime: NOT RED — the VM pin does not bite"; exit 1
fi
restore src/vm.rs; check_byte_identical src/vm.rs

# ── M4: the typed-secret lane hand ──
cat > /tmp/m4.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """            if let Some(i) = crate::sink_receipt::receipt_echo_arg(name) {
                if let Some(a) = args.get(i) {
                    if let Some(t) = label_of_expr(a, vars) {
                        return Some(t);
                    }
                }
            }
"""
assert block in s, "M4: the typed-lane echo hand not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
mutate src/secret_label.rs /tmp/m4.py
if cargo test --test naryad_980_receipt_echo n980_typed_lane_carries_the_secret_reason 2>&1 | grep -q "test result: FAILED"; then
    echo "M4 typed lane: RED as expected"
else
    echo "M4 typed lane: NOT RED — the typed pin does not bite"; exit 1
fi
restore src/secret_label.rs; check_byte_identical src/secret_label.rs

echo "MUTATION VERIFY 980: PASS (M1-M4 red, restores byte-identical)"
