#!/usr/bin/env bash
# №981 mutation verification (the mutation_verify_980.sh form): remove
# EACH runtime lane's external-return hand, watch EXACTLY that lane's
# pins redden, restore byte-identical. Two mutations (the two hands №981
# adds; the static hands' pins live in tests/naryad_978_external_returns.rs
# and were verified by №978):
#   M1 interpreter/labels.rs  → the TW pins (http_post + exec refusals)
#   M2 vm.rs                  → the VM pin (the exec return refusal)
# PASS before the push — the dispatch rule. The restore is `git checkout
# --` — run against a COMMITTED tree (see mutation_verify_980.sh).
# NOTE: the crosscheck file sets the same env vars in-process; run the
# pins via cargo test (fresh processes) — do not run the two suites in
# one cargo invocation with the mutations applied (env leakage across
# test binaries is impossible: separate processes per binary).
set -u
cd "$(dirname "$0")/.."

# On ANY exit (including a mid-mutation failure) restore the src tree.
trap 'git checkout -- src/ 2>/dev/null' EXIT

restore() { # $1 file
    git checkout -- "$1"
    touch "$1"
    sleep 1
}

check_byte_identical() {
    if [ -n "$(git status --porcelain "$1")" ]; then
        echo "RESTORE FAILED for $1"; exit 1
    fi
}

# ── M1: the TW lane hand ──
cat > /tmp/m981_1.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """        if crate::sink_receipt::external_sink_return(name) {
            return Label {
                conf: crate::labels::Conf::Public,
                integrity: crate::labels::Integrity::Untrusted,
                consent: Default::default(),
            };
        }
"""
assert block in s, "M1: the TW external-return hand not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
python3 /tmp/m981_1.py src/interpreter/labels.rs
if cargo test --test naryad_981_external_returns_runtime 2>&1 | grep -E "n981_tw_runtime_the_(http_post|exec)_return_refuses" | grep -q "FAILED"; then
    echo "M1 TW runtime: RED as expected"
else
    echo "M1 TW runtime: NOT RED — the TW pins do not bite"; exit 1
fi
restore src/interpreter/labels.rs; check_byte_identical src/interpreter/labels.rs

# ── M2: the VM lane hand ──
cat > /tmp/m981_2.py <<'PYEOF'
import sys
p = sys.argv[1]
s = open(p).read()
block = """                if crate::sink_receipt::external_sink_return(name) {
                    return crate::labels::Label {
                        conf: crate::labels::Conf::Public,
                        integrity: crate::labels::Integrity::Untrusted,
                        consent: Default::default(),
                    };
                }
"""
assert block in s, "M2: the VM external-return hand not found"
open(p, 'w').write(s.replace(block, ""))
PYEOF
python3 /tmp/m981_2.py src/vm.rs
if cargo test --test naryad_981_external_returns_runtime 2>&1 | grep -E "n981_vm_runtime_the_exec_return_refuses" | grep -q "FAILED"; then
    echo "M2 VM runtime: RED as expected"
else
    echo "M2 VM runtime: NOT RED — the VM pin does not bite"; exit 1
fi
restore src/vm.rs; check_byte_identical src/vm.rs

echo "MUTATION VERIFY 981: PASS (M1-M2 red, restores byte-identical)"
