# Release Checklist (№509, ADR-0179)

The machine gate (`unfreeze_gate.py`) collects the evidence; the OWNER
decides the publication. This checklist is the named human steps between
the evidence and the decision — the 0.27.0 lesson (a release that passes
a no-regress gate while High findings are open and a feature is dead) is
what each step closes.

## The 0.28 window (ADR-0179 §6)

1. **The release-block sync.** Count the open GitHub issues labeled
   `release-block` (known High findings in the server path); write the
   count and the issue numbers into `fact_open_high_server` in
   `scripts/ci/gate_028_goals.txt`; link the syncing PR from the release
   notes.
2. **The quorum sync.** Run `scripts/ci/count_duplicated_names.py`,
   write the still-carrying groups over the №466 total into
   `fact_quorum_num` / `fact_quorum_den`.
3. **The serve-e2e inventory.** Every row of
   `scripts/ci/serve_e2e_inventory.txt` must be `done` — flipped ONLY in
   the PR that adds the actual both-backend `run_test_server` test for
   that declaration (distill — №496; memory/conversation/cron — the
   follow-up naryads from gh#792).
4. **The gate, v2 mode.** Run
   `python3 scripts/ci/unfreeze_gate.py --office-tests <pass|fail> --gate-target 0.28`;
   attach `unfreeze_summary.md` to the release. A RED anywhere blocks
   the read — the release does not proceed while blocked.
5. **The owner's read.** The OWNER reads the summary and decides the
   publication. The machine never lifts a gate and never publishes.

## After the owner publishes

6. **The assets.** Publication fires `.github/workflows/release.yml`
   (`release: published`): it builds the tagged commit, generates the
   CycloneDX SBOM, signs the two attestations (build provenance + SBOM) and
   attaches binary, SBOM, `BUILD-INFO.txt` and `SHA256SUMS` to the release.
   Confirm the run is green and the release shows the four assets; spot-check with
   `gh attestation verify mlog-linux-x86_64 --repo ShkodnikAI/Metalogos-`.
   A red run does NOT invalidate the release — re-run it with
   `workflow_dispatch` (`tag` = the release tag, `upload` = true). The
   machine still never publishes: this step only decorates a release the
   owner already published.

## The legacy read (0.27.x)

`unfreeze_gate.py --office-tests <pass|fail>` without `--gate-target` —
the ADR-0177 §4 verdicts only. Unchanged.
