#!/usr/bin/env bash
# ── Naryad №428 (issue #614) — the audio-consent mutation verification ──
#
# The №382 mutation-verification protocol applied to the duplex consent gate:
#
#   M-CONSENT-GATE:  neuter the gate itself (require_audio_consent always
#                    Ok) -> n428_speak_without_consent_is_typed_and_audited
#                    MUST FAIL (the unconsented speak must refuse).
#   M-LEDGER-EGRESS: neuter the denial ledger record (a refusal leaves no
#                    duplex.speak_denied trace) ->
#                    n428_speak_without_consent_is_typed_and_audited MUST
#                    FAIL (no silent refusal).
#
# Reproducibility contract (the №418 harness pattern): the mutations are
# applied in a throwaway worktree sharing the main target dir; cargo runs
# IN THE WORKTREE; every injection asserts its anchor; the working tree is
# never touched. Run AFTER the code lands in a commit.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0

WT=$(mktemp -d /tmp/n428-mutations.XXXXXX)
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
git worktree add --detach "$WT" HEAD >/dev/null 2>&1
# Share the main target dir: a throwaway worktree must not build the
# dependency graph from scratch (disk-bound box — the ld Bus error lesson).
export CARGO_TARGET_DIR="$PWD/target"

echo "── sanity: the unmutated corpus is green at HEAD ──"
S=$( ( cd "$WT" && cargo test --test naryad_428_audio_consent 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $S"
case "$S" in
  *"0 failed"*) echo "  sanity OK: the corpus is green before mutating" ;;
  *) echo "  SANITY FAILED — the corpus must be green BEFORE mutation"; exit 1 ;;
esac

mutate() { # (file, python snippet) — apply one injection in the worktree
  python3 - "$WT/$1" << PYEOF
import sys
p = sys.argv[1]
src = open(p).read()
$2
open(p, "w").write(src)
PYEOF
}

echo "── M-CONSENT-GATE: neuter the direction consent check ──"
mutate "src/duplex.rs" '
old = """    let scope = format!(\"audio.{}\", direction_flow);
    if crate::consent::active_grant_for(&scope) {
        return Ok(());
    }"""
new = """    let scope = format!(\"audio.{}\", direction_flow);
    if true {
        // MUTATION M-CONSENT-GATE: the gate is neutered — unconsented
        // audio flows are no longer refused.
        let _ = scope;
        return Ok(());
    }"""
assert old in src, "M-CONSENT-GATE anchor not found (the direction consent check)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_428_audio_consent n428_speak_without_consent_is_typed_and_audited 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-CONSENT-GATE VERIFIED: the negative test falls when the gate is neutered" ;;
  *) echo "  M-CONSENT-GATE NOT VERIFIED — the test survived a neutered gate"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/duplex.rs )

echo "── M-LEDGER-EGRESS: neuter the denial ledger record ──"
mutate "src/duplex.rs" '
old = """    ledger_duplex_event(&format!(\"{}_denied\", direction_flow), channel_id, &detail);"""
new = """    // MUTATION M-LEDGER-EGRESS: the refusal goes unrecorded — a silent
    // refusal (the audit trail is gone).
    let _ = (&direction_flow, &channel_id, &detail);"""
assert old in src, "M-LEDGER-EGRESS anchor not found (the denial ledger record)"
src = src.replace(old, new, 1)'
R=$( ( cd "$WT" && cargo test --test naryad_428_audio_consent n428_speak_without_consent_is_typed_and_audited 2>&1 | grep -E "test result" | head -1 ) || true )
echo "  test verdict: $R"
case "$R" in
  *FAILED*) echo "  M-LEDGER-EGRESS VERIFIED: the audit test falls when the denial record is neutered" ;;
  *) echo "  M-LEDGER-EGRESS NOT VERIFIED — the test survived a silent refusal"; exit 1 ;;
esac
( cd "$WT" && git checkout -- src/duplex.rs )

echo "── mutation verification 2/2 VERIFIED ──"
