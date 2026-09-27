#!/usr/bin/env bash
# ── Naryad №757 (issue #757, P1 llm/hardening) — the mutation
#    verification for the configurable ceiling + the visible truncation ──
#
# The contract (pipeline v1.2): the tests must be RED under each
# mutation the naryad's logic names — a green-under-mutation test does
# not pin anything.
#
#   M1 — the default ceiling regresses 4096 → 1024 (the pre-№757 value)
#        -> n757_default_ceiling_on_the_wire_is_4096 MUST FAIL (the wire
#        body carries 1024, not 4096).
#
#   M2 — the block value is ignored by from_config (llm { max_tokens }
#        silently dropped)
#        -> n757_block_max_tokens_reaches_body_and_truncation_is_visible
#        MUST FAIL (the body carries 4096, not 8000).
#
#   M3 — extract_finish_reason returns None everywhere (the reason is
#        never read on the stream path)
#        -> n757_stream_truncation_is_visible_after_close MUST FAIL
#        (the probe stays empty after close).
#
#   M4 — the parser's loud range validation is dropped (max_tokens: 0
#        parses silently)
#        -> test_parse_llm_config_max_tokens_zero_is_loud MUST FAIL.
#
# Reproducibility contract: the harness is versioned IN THE REPO, works
# in a throwaway worktree sharing the main target dir — no manual edits,
# the working tree is never touched, mutants never reach main.
# HAZARD NOTE (shared target): run `touch src/llm.rs` (or cargo clean)
# in the MAIN tree before trusting a cargo test result that follows a
# harness run. CI (isolated) is unaffected.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0

WT=$(mktemp -d /tmp/n757-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
export CARGO_TARGET_DIR="$PWD/target"

run_test() { # (suite, filter) -> the result line; failure IS expected
  ( cd "$WT" && cargo test --test "$1" "$2" 2>&1 | grep -E "test result" | head -1 ) || true
}

mutate() { # (file, needle, replacement)
  python3 - "$WT/$1" "$2" "$3" << 'PYEOF'
import sys
path, needle, repl = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(path).read()
assert needle in src, "mutation anchor not found: " + repr(needle[:70])
src = src.replace(needle, repl, 1)
open(path, "w").write(src)
print("mutated OK:", repr(needle[:55]))
PYEOF
}

check_red() { # (label, result line)
  case "$2" in
    *"0 failed"*)
      echo "✗ $1: test stayed GREEN under mutation — the pin is broken"; exit 1 ;;
    *failed*)
      echo "✓ $1: RED as required — $2" ;;
    *)
      echo "✗ $1: no result line (compile error counts as caught, verify manually): $2"; exit 1 ;;
  esac
}

echo "── M1: DEFAULT_LLM_MAX_TOKENS regresses to 1024 ──"
mutate src/llm.rs "pub const DEFAULT_LLM_MAX_TOKENS: u32 = 4096;" \
                  "pub const DEFAULT_LLM_MAX_TOKENS: u32 = 1024;"
M1=$(run_test naryad_757_llm_limits n757_default_ceiling_on_the_wire_is_4096)
check_red "M1" "$M1"
git -C "$WT" checkout -- src/llm.rs

echo "── M2: from_config ignores the block max_tokens ──"
mutate src/llm.rs "            max_tokens: config
                .max_tokens
                .or_else(env_max_tokens)
                .unwrap_or(DEFAULT_LLM_MAX_TOKENS)," \
                  "            max_tokens: std::option::Option::<u32>::None
                .or_else(env_max_tokens)
                .unwrap_or(DEFAULT_LLM_MAX_TOKENS),"
M2=$(run_test naryad_757_llm_limits n757_block_max_tokens_reaches_body_and_truncation_is_visible)
check_red "M2" "$M2"
git -C "$WT" checkout -- src/llm.rs

echo "── M3: the stream finish_reason is never extracted ──"
mutate src/llm.rs "                if let Some(fr) = extract_finish_reason(&state.provider_type, &parsed) {
                    state.finish_reason = Some(fr);
                }" \
                  "                let _ = extract_finish_reason(&state.provider_type, &parsed);"
M3=$(run_test naryad_757_llm_limits n757_stream_truncation_is_visible_after_close)
check_red "M3" "$M3"
git -C "$WT" checkout -- src/llm.rs

echo "── M4: the parser's zero-ceiling guard is dropped ──"
mutate src/parser/decl.rs "            match raw.parse::<u32>() {
                Ok(n) if n >= 1 => Some(n)," \
                  "            match raw.parse::<u32>() {
                Ok(n) => Some(n),"
M4=$(run_test "" test_parse_llm_config_max_tokens_zero_is_loud 2>/dev/null || true)
# parser tests live in the LIB suite
M4=$( ( cd "$WT" && cargo test --lib test_parse_llm_config_max_tokens_zero_is_loud 2>&1 | grep -E "test result" | head -1 ) || true )
check_red "M4" "$M4"
git -C "$WT" checkout -- src/parser/decl.rs

echo ""
echo "All №757 mutations caught (4/4). The pins hold."
