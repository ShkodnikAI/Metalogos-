#!/usr/bin/env bash
# ── Naryad №959 (issue #1926, the audit 12.11 — P1) — the mutation
#    verification for the fact classes and the slot surfaces ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
# The issue's «сделано, когда» names this one exactly: declare a
# structural fact behavioural WITHOUT an executable test and
# `gen_metrics.py --check` must go RED.
#
#   M1 — C8/REPLAY_MATCH (a purely structural, kernel-only fact with no
#        executed test anywhere near it) is relabelled `behavioural`
#        -> `gen_metrics.py --check` MUST exit non-zero and the message
#           MUST name the missing `exec=`.
#
#   M2 — C1/ATTEST_MAP (a real builtin surface) is relabelled
#        `kernel-only`
#        -> `--check` MUST exit non-zero: an UNDERSTATED row hides a
#           capability as surely as an overstated one overclaims it, and
#           the gate refuses both directions.
#
#   M3 — the whole roster is emptied of rows
#        -> the loader MUST refuse loudly (a roster that parses to zero
#           rows would read as «no facts», and a vacuous count must never
#           read as a pass — the №525 rule).
#
# The restore is a BACKUP-COPY restore (the working tree carries the
# naryad's uncommitted work — `git checkout --` would wipe it).
# Run from the repo root:  bash scripts/mutation_verify_959.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

ROSTER=scripts/ci/slot_class_roster.txt
BK=/tmp/n959_roster_backup.txt
cp "$ROSTER" "$BK" || exit 1
fail() { cp "$BK" "$ROSTER"; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BK" "$ROSTER"; }

echo "── M1: a structural fact relabelled behavioural, with no executed test ──"
python3 - <<'EOF'
p = 'scripts/ci/slot_class_roster.txt'
s = open(p, encoding='utf-8').read()
needle = 'slot C8 fact=REPLAY_MATCH class=structural'
assert needle in s, "the C8 row moved — repair the mutation target"
open(p, 'w', encoding='utf-8').write(
    s.replace(needle, 'slot C8 fact=REPLAY_MATCH class=behavioural', 1))
EOF
[ $? -eq 0 ] || fail "M1 could not be applied"
if python3 -I scripts/gen_metrics.py --check >/tmp/n959_m1.log 2>&1; then
  fail "M1 left --check GREEN — a behavioural claim without an executed test pins nothing"
fi
grep -q "class=behavioural with NO exec=" /tmp/n959_m1.log \
  || fail "M1 went red, but the message does not name the missing exec="
echo "   M1 OK — RED, and the refusal names the missing executed test"
restore

echo "── M2: a real builtin surface relabelled kernel-only ──"
python3 - <<'EOF'
p = 'scripts/ci/slot_class_roster.txt'
s = open(p, encoding='utf-8').read()
needle = 'slot C1 fact=ATTEST_MAP class=structural surface=builtin'
assert needle in s, "the C1 row moved — repair the mutation target"
open(p, 'w', encoding='utf-8').write(
    s.replace(needle, 'slot C1 fact=ATTEST_MAP class=structural surface=kernel-only', 1))
EOF
[ $? -eq 0 ] || fail "M2 could not be applied"
if python3 -I scripts/gen_metrics.py --check >/tmp/n959_m2.log 2>&1; then
  fail "M2 left --check GREEN — an understated surface hides a capability silently"
fi
grep -q "a builtin/CLI file NOW names" /tmp/n959_m2.log \
  || fail "M2 went red, but not on the understated-surface check"
echo "   M2 OK — RED on the understated row too"
restore

echo "── M3: the roster emptied of rows ──"
python3 - <<'EOF'
p = 'scripts/ci/slot_class_roster.txt'
s = open(p, encoding='utf-8').read()
kept = [l for l in s.splitlines() if not l.strip().startswith('slot ')]
open(p, 'w', encoding='utf-8').write("\n".join(kept) + "\n")
EOF
[ $? -eq 0 ] || fail "M3 could not be applied"
if python3 -I scripts/gen_metrics.py --check >/tmp/n959_m3.log 2>&1; then
  fail "M3 left --check GREEN — an empty roster read as a pass"
fi
grep -q "ZERO rows" /tmp/n959_m3.log \
  || fail "M3 went red, but not on the vacuous-roster refusal"
echo "   M3 OK — the vacuous roster is a loud refusal"
restore

echo "── the clean tree ──"
python3 -I scripts/gen_metrics.py --check >/tmp/n959_clean.log 2>&1 \
  || fail "the clean tree is RED — restore did not take"
grep -q "TERMINAL_STOP: structural" /tmp/n959_clean.log \
  || fail "the clean tree is green but does not print the class split"
cargo test --test naryad_959_fact_classes >/tmp/n959_rust.log 2>&1 \
  || fail "the Rust driver is RED on the clean tree"
echo "MUTATION VERIFY: OK — M1, M2 and M3 all spring, the clean tree is green"
