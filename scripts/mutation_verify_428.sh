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
# Reproducibility contract: the harness is versioned IN THE REPO and works
# in a throwaway worktree sharing the main target dir — no manual edits,
# the working tree is never touched. Run AFTER the code lands in a commit.
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
cargo test --test naryad_428_audio_consent 2>&1 | rg "test result:" | head -1

mutate_and_run() {
  local name="$1" file="$2" expect_fail="$3"
  local backup="$WT/$(basename "$file").bak"
  cp "$WT/$file" "$backup"
  echo "── mutant $name ──"
  if [ "$name" = "M-CONSENT-GATE" ]; then
    # Neuter the gate: the direction check always passes.
    python3 - "$WT" <<'EOF'
import sys, pathlib
wt = sys.argv[1]
p = pathlib.Path(wt) / "src/duplex.rs"
s = p.read_text()
s = s.replace(
    '    let scope = format!("audio.{}", direction_flow);\n    if crate::consent::active_grant_for(&scope) {\n        return Ok(());\n    }',
    '    let scope = format!("audio.{}", direction_flow);\n    if true {\n        return Ok(());\n    }')
p.write_text(s)
EOF
  elif [ "$name" = "M-LEDGER-EGRESS" ]; then
    # Neuter the denial ledger record: the refusal goes unrecorded.
    python3 - "$WT" <<'EOF'
import sys, pathlib
wt = sys.argv[1]
p = pathlib.Path(wt) / "src/duplex.rs"
s = p.read_text()
s = s.replace(
    '    ledger_duplex_event(\n        &format!("{}_denied", direction_flow),\n        channel_id,\n        &detail,\n    );\n',
    '')
p.write_text(s)
EOF
  fi
  set +e
  cargo test --test naryad_428_audio_consent 2>&1 | rg "test result:|panicked at|FAILED" | head -4
  local rc=${PIPESTATUS[0]}
  set -e
  cp "$backup" "$WT/$file"
  if [ "$rc" -ne 0 ]; then
    echo "   KILLED by $expect_fail (exit $rc) — MUTATION VERIFIED"
  else
    echo "   SURVIVED — MUTATION FAILED"
    exit 1
  fi
}

mutate_and_run M-CONSENT-GATE src/duplex.rs n428_speak_without_consent_is_typed_and_audited
mutate_and_run M-LEDGER-EGRESS src/duplex.rs n428_speak_without_consent_is_typed_and_audited

echo "── mutation verification 2/2 VERIFIED ──"
