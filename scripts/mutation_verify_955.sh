#!/usr/bin/env bash
# ── Naryad №955 (issue #1922, the audit 12.11 — P0) — the mutation
#    verification for the error-text hygiene ──
#
# The contract (pipeline v1.2): the tests must be RED under the mutation
# the naryad names — a green-under-mutation test does not pin anything.
#
#   M1 — the builtin-level mask is dropped (the raw `{s:?}` value returns
#        to the to_int error text)
#        -> n955_to_int_error_text_masks_the_value MUST FAIL
#        (the secret reappears in the error text).
#
#   M2 — the serve generic-500 is dropped (the full handler text returns
#        to the HTTP body unconditionally)
#        -> n955_serve_500_body_is_generic_and_the_log_is_masked MUST FAIL
#        (the client receives `Handler error: …` again).
#
# The restore is a BACKUP-COPY restore (the working tree carries the
# naryad's uncommitted work — `git checkout --` would wipe it).
# Run from the repo root:  bash scripts/mutation_verify_955.sh
set -u
export PATH="$HOME/.cargo/bin:$PATH"

BK_MATH=/tmp/n955_math_backup.rs
BK_SRV=/tmp/n955_srv_backup.rs
cp src/builtins/math.rs "$BK_MATH" || exit 1
cp metalogos-server/src/server.rs "$BK_SRV" || exit 1
fail() { cp "$BK_MATH" src/builtins/math.rs; cp "$BK_SRV" metalogos-server/src/server.rs; echo "MUTATION VERIFY: FAIL — $1"; exit 1; }
restore() { cp "$BK_MATH" src/builtins/math.rs; cp "$BK_SRV" metalogos-server/src/server.rs; }

echo "── M1: the builtin-level mask dropped ──"
python3 - <<'EOF'
p = 'src/builtins/math.rs'
s = open(p).read()
needle = '''                "to_int({}): not a number — use to_int_or(value, <default>) for an explicit fallback (№514; №955 — the value is masked: the error text never quotes string contents)",
                        crate::interpreter::values::masked_string(s)'''
replacement = '''                "to_int({s:?}): not a number — use to_int_or(value, <default>) for an explicit fallback (№514; №955 MUTATED)"'''
assert needle in s, "the masked to_int site not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test --test naryad_955_error_text_hygiene 2>&1 | grep -q "test result: FAILED" \
  && echo "M1 RED — the pin works" || fail "M1: the mask pin stayed GREEN under the mask-drop mutation"
restore

echo "── M2: the serve generic-500 dropped ──"
python3 - <<'EOF'
p = 'metalogos-server/src/server.rs'
s = open(p).read()
needle = """    if serve_verbose_errors() {
        eprintln!("[ROUTE_HANDLER_ERROR] {} {} -> 500 (VERBOSE MODE): {}", method, uri_path, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Handler error: {}", e),
        )
            .into_response();
    }"""
replacement = """    if true {
        // MUTATION (M2): the generic-500 form dropped — the full text
        // returns to the client unconditionally.
        eprintln!("[ROUTE_HANDLER_ERROR] {} {} -> 500 (VERBOSE MODE): {}", method, uri_path, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Handler error: {}", e),
        )
            .into_response();
    }"""
assert needle in s, "the serve helper block not found — the tree drifted"
open(p, 'w').write(s.replace(needle, replacement, 1))
EOF
CARGO_INCREMENTAL=0 cargo test -p metalogos-server --test naryad_955_serve_500 n955_serve_500_body_is_generic_and_the_log_is_masked 2>&1 | grep -q "test result: FAILED" \
  && echo "M2 RED — the pin works" || fail "M2: the serve pin stayed GREEN under the generic-form-drop mutation"
restore

echo "── final: the clean tree is GREEN ──"
CARGO_INCREMENTAL=0 cargo test --test naryad_955_error_text_hygiene 2>&1 | grep -q "test result: ok" \
  && CARGO_INCREMENTAL=0 cargo test -p metalogos-server --test naryad_955_serve_500 2>&1 | grep -q "test result: ok" \
  && echo "MUTATION VERIFY: OK — 2 mutations red, the clean tree green" || fail "the clean tree is not green after the restores"
