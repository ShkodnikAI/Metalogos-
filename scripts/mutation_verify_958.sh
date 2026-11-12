#!/usr/bin/env bash
# ── Naryad №958 (issue #1925, the audit 12.11 — P1) — the mutation
#    verification for the input-depth limits ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the parser pre-scan is neutered (the scan result is ignored)
#        -> n958_deep_plus_refuses_cleanly MUST FAIL (the 50 000-`+` probe
#        aborts / hangs the parser thread again).
#
#   M2 — the whole depth-guard mechanism is neutered (DepthGuard::exceeded
#        answers false AND the entry measurement returns None)
#        -> n958_walk_guard_refuses_the_deep_ast_cleanly MUST FAIL (the
#        50 000-deep synthetic AST overflows the recursive walkers —
#        the process aborts).
#
# The restore is a BACKUP-COPY restore (the working tree carries the
# naryad's uncommitted work — `git checkout --` would wipe it).
# Run from the repo root:  bash scripts/mutation_verify_958.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

BK_PARSER=/tmp/n958_parser_backup.rs
BK_GUARD=/tmp/n958_guard_backup.rs
cp src/parser/mod.rs "$BK_PARSER" || exit 1
cp src/depth_guard.rs "$BK_GUARD" || exit 1
fail() { cp "$BK_PARSER" src/parser/mod.rs; cp "$BK_GUARD" src/depth_guard.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BK_PARSER" src/parser/mod.rs; cp "$BK_GUARD" src/depth_guard.rs; }

echo "── M1: the parser pre-scan neutered ──"
python3 -c "
p = 'src/parser/mod.rs'
s = open(p).read()
needle = '    scan_input_depth(source)?;'
replacement = '    // MUTATION (M1): the depth pre-scan neutered.\n    let _ = scan_input_depth(source);'
assert needle in s, 'the scan call not found — the tree drifted'
open(p, 'w').write(s.replace(needle, replacement, 1))
"
CARGO_INCREMENTAL=0 cargo test -p metalogos-server --test naryad_958_input_depth n958_deep_plus_refuses_cleanly 2>&1 | grep -q "test result: FAILED" \
  && echo "M1 RED — the pin works" || fail "M1: the parser-scan pin stayed GREEN under the neuter mutation"
restore

echo "── M2: the whole depth-guard mechanism neutered ──"
python3 -c "
p = 'src/depth_guard.rs'
s = open(p).read()
needle = '        WALK_DEPTH.with(|d| d.get() > crate::limits::max_input_depth())'
replacement = '        false // MUTATION (M2): the walk-guard mechanism neutered'
assert needle in s, 'exceeded() not found'
s = s.replace(needle, replacement, 1)
needle2 = '    // The statement walk is FLAT'
replacement2 = '    if true { return None; } // MUTATION (M2): the entry measurement neutered\n    // The statement walk is FLAT'
assert needle2 in s, 'the entry guard block not found'
s = s.replace(needle2, replacement2, 1)
open(p, 'w').write(s)
"
CARGO_INCREMENTAL=0 cargo test --test naryad_958_walk_guard 2>&1 | grep -qE "test result: FAILED|signal: 6|SIGABRT" \
  && echo "M2 RED — the pin works" || fail "M2: the walk-guard pin stayed GREEN under the neuter mutation"
restore

echo "── final: the clean tree is GREEN ──"
CARGO_INCREMENTAL=0 cargo test --test naryad_958_walk_guard 2>&1 | grep -q "test result: ok" \
  && CARGO_INCREMENTAL=0 cargo test -p metalogos-server --test naryad_958_input_depth 2>&1 | grep -q "test result: ok" \
  && echo "MUTATION VERIFY: OK — 2 mutations red, the clean tree green" || fail "the clean tree is not green after the restores"
