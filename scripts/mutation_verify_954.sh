#!/usr/bin/env bash
# ── Naryad №954 (issue #1921, the audit 12.11 — P0) — the mutation
#    verification for the capability-permit origin rule ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the origin check is neutered (the refusal and the window both
#        degrade to Ok — the H-2 hole reopens at the gates)
#        -> n954_check_refuses_the_imported_capability_profile MUST FAIL
#        -> n954_run_gate_refuses_the_imported_profile_before_execution MUST FAIL.
#
#   M2 — the compiler merge re-unions the imported declarations (the
#        v0.39.0 roster hole — `permits_from_declarations(&all_decls)`
#        over the unfiltered import set)
#        -> n954_run_sink_refuses_even_in_the_warn_window MUST FAIL
#        (the roster extends; the POST is attempted instead of the
#        IRREVERSIBLE_NO_GRANT refusal).
#
# The restore is a BACKUP-COPY restore (the working tree carries the
# naryad's uncommitted work — `git checkout --` would wipe it; the
# lesson of the first №953 mutation run).
# Run from the repo root:  bash scripts/mutation_verify_954.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

BK_CAP=/tmp/n954_cap_backup.rs
BK_CMP=/tmp/n954_cmp_backup.rs
cp src/capability.rs "$BK_CAP" || exit 1
cp src/compiler.rs "$BK_CMP" || exit 1
fail() { cp "$BK_CAP" src/capability.rs; cp "$BK_CMP" src/compiler.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BK_CAP" src/capability.rs; cp "$BK_CMP" src/compiler.rs; }

echo "── M1: the origin check neutered ──"
python3 - <<'EOF'
p = 'src/capability.rs'
s = open(p).read()
needle = """    if found.is_empty() {
        return Ok(());
    }
    let entry_warns = entry.iter().any(|decl| {"""
replacement = """    if found.is_empty() {
        return Ok(());
    }
    if true {
        // MUTATION (M1): the origin check neutered — always Ok
        return Ok(());
    }
    let entry_warns = entry.iter().any(|decl| {"""
assert needle in s, "the origin check block not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_954_capability_root 2>&1 | grep -q "test result: FAILED" \
  && echo "M1 RED — the pins work" || fail "M1: the origin pins stayed GREEN under the neuter mutation"
restore

echo "── M2: the compiler merge re-unions the imports ──"
python3 - <<'EOF'
p = 'src/compiler.rs'
s = open(p).read()
needle = """        // №954: the imported declarations merge MINUS the capability
        // profiles — the permit roster below (and the TW runtime loader,
        // which skips `Profile` already) never extends from an import.
        // The entry's OWN `profile capability` rides `all_decls` as
        // before: the roster of the ENTRY program is the whole roster.
        all_decls.extend(
            imported_decls
                .into_iter()
                .filter(|d| !matches!(d, Declaration::Profile(p) if p.name == "capability")),
        );"""
replacement = """        // MUTATION (M2): the v0.39.0 union returns — the imported
        // capability profile rides the roster resolution again.
        all_decls.extend(imported_decls);"""
assert needle in s, "the compiler merge block not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_954_capability_root n954_run_sink_refuses_even_in_the_warn_window 2>&1 | grep -q "test result: FAILED" \
  && echo "M2 RED — the pin works" || fail "M2: the sink pin stayed GREEN under the union mutation"
restore

echo "── final: the clean tree is GREEN ──"
CARGO_INCREMENTAL=0 cargo test --test naryad_954_capability_root 2>&1 | grep -q "test result: ok" \
  && echo "MUTATION VERIFY: OK — 2 mutations red, the clean tree green" || fail "the clean tree is not green after the restores"
