#!/usr/bin/env bash
# ── Naryad №964 (the B7 line, the owner decision 13.11) — the mutation
#    verification for the sink-receipt rule ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
# The receipt rule has TWO hooks, one per label lane, and a mutation of
# each must turn a DIFFERENT test red. That is the point of running both:
# if only one mutation sprang, the other hook would be unpinned decoration.
#
#   M1 — the SECRET lane's hook is removed (secret_label::label_of_expr
#        falls back to the argument join)
#        -> n964_the_receipt_of_a_legal_quarantine_write_is_usable MUST
#           FAIL: [SECRET_LEAK] returns on `print(receipt)`.
#
#   M2 — the LATTICE hook is removed (semantic::expr_label falls back to
#        the argument join)
#        -> n964_a_receipt_may_travel_to_another_sink MUST FAIL: the
#           receipt arrives at the second sink carrying the payload's
#           label and the clearance gate refuses it.
#
#   M3 — a DELIVERY sink (`print`) is added to the receipt table
#        -> n964_the_delivery_sinks_are_not_receipts MUST FAIL: that is
#           the laundering path the table exists to refuse, and the guard
#           must catch a widening of the table into the egress family.
#
# The restore is a BACKUP-COPY restore (the working tree carries the
# naryad's uncommitted work — `git checkout --` would wipe it).
# Run from the repo root:  bash scripts/mutation_verify_964.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

TEST=naryad_964_sink_receipt
BK_SEC=/tmp/n964_secret_label.rs
BK_SEM=/tmp/n964_semantic.rs
BK_CLS=/tmp/n964_classification.rs
cp src/secret_label.rs "$BK_SEC" || exit 1
cp src/semantic.rs "$BK_SEM" || exit 1
cp src/builtins_classification.rs "$BK_CLS" || exit 1
restore() {
  cp "$BK_SEC" src/secret_label.rs
  cp "$BK_SEM" src/semantic.rs
  cp "$BK_CLS" src/builtins_classification.rs
}
fail() { restore; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }

echo "── M1: the SECRET lane's receipt hook removed ──"
python3 - <<'EOF'
p = 'src/secret_label.rs'
s = open(p, encoding='utf-8').read()
needle = """            if crate::builtins_classification::is_sink_receipt(name) {
                return declared_base_type(name);
            }"""
assert needle in s, "the secret-lane hook not found — the tree drifted"
open(p, 'w', encoding='utf-8').write(s.replace(needle, "            // №964 M1 MUTATION", 1))
EOF
[ $? -eq 0 ] || fail "M1 could not be applied"
if cargo test --test "$TEST" >/tmp/n964_m1.log 2>&1; then
  fail "M1 left the suite GREEN — the secret-lane hook pins nothing"
fi
grep -q "n964_the_receipt_of_a_legal_quarantine_write_is_usable" /tmp/n964_m1.log \
  || fail "M1 went red, but not on the quarantine-receipt test"
echo "   M1 OK — the quarantine receipt test is RED without the secret-lane hook"
restore

echo "── M2: the LATTICE receipt hook removed ──"
python3 - <<'EOF'
p = 'src/semantic.rs'
s = open(p, encoding='utf-8').read()
needle = """            if crate::builtins_classification::is_sink_receipt(name) {
                return Label::bottom();
            }"""
assert needle in s, "the lattice hook not found — the tree drifted"
open(p, 'w', encoding='utf-8').write(s.replace(needle, "            // №964 M2 MUTATION", 1))
EOF
[ $? -eq 0 ] || fail "M2 could not be applied"
if cargo test --test "$TEST" >/tmp/n964_m2.log 2>&1; then
  fail "M2 left the suite GREEN — the LATTICE hook pins nothing (it would be decoration)"
fi
grep -q "n964_a_receipt_may_travel_to_another_sink" /tmp/n964_m2.log \
  || fail "M2 went red, but not on the receipt-to-another-sink test"
echo "   M2 OK — the second-sink test is RED without the lattice hook"
restore

echo "── M3: a delivery sink added to the receipt table ──"
python3 - <<'EOF'
p = 'src/builtins_classification.rs'
s = open(p, encoding='utf-8').read()
needle = 'pub const SINK_RECEIPT_NAMES: &[&str] = &[\n    "quarantine_write",'
assert needle in s, "the receipt table not found — the tree drifted"
open(p, 'w', encoding='utf-8').write(
    s.replace(needle, 'pub const SINK_RECEIPT_NAMES: &[&str] = &[\n    "print",\n    "quarantine_write",', 1))
EOF
[ $? -eq 0 ] || fail "M3 could not be applied"
if cargo test --test "$TEST" >/tmp/n964_m3.log 2>&1; then
  fail "M3 left the suite GREEN — the table could be widened into the egress family silently"
fi
grep -q "n964_the_delivery_sinks_are_not_receipts" /tmp/n964_m3.log \
  || fail "M3 went red, but not on the delivery-sink guard"
echo "   M3 OK — the delivery-sink guard is RED when the table is widened"
restore

echo "── the clean tree ──"
cargo test --test "$TEST" >/tmp/n964_clean.log 2>&1 \
  || fail "the clean tree is RED — restore did not take"
echo "MUTATION VERIFY: OK — M1, M2 and M3 all spring, the clean tree is green"
