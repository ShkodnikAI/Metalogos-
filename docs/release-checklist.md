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

## The train (ADR-0189, Accepted 2026-10-08 — gh#1148)

The release rhythm: MONDAYS. **Activated: the first window is
2026-10-12 — train №1** (the owner's verdict verbatim in the ADR
header). The window is a working block of the day, not a minute-exact
schedule; the executor prepares everything machine-side, the owner
publishes when satisfied.

1. **Check [Unreleased]** — is there content? An empty `[Unreleased]`
   means the window is SKIPPED without blame (the honest-content rule:
   a train without content does not depart — never a version bump with
   no user-visible change).
2. **Class the version number** — Security/Fixed only → a PATCH cut
   (`0.30.x`); Added/Changed present → a MINOR cut (`0.31.0`).
3. **The gate read** — `unfreeze_gate.py --gate-target <target>
   --strict` must exit 0 for the record the number class names.
4. **No open release-block** (ADR-0179 §4) against the target — an open
   one holds the train; the hold is recorded in the owning wave's
   dispatch thread.
5. **The lockstep PR** (the №646 shape: version + `Cargo.lock` ×5 +
   badge sync + the generated Version line + the CHANGELOG date-stamp),
   CI green, squash-merge.
6. **The tag** on the merge commit; **the build** (release.yml — the
   assets, the SBOM, the attestations).
7. **The publication** — the OWNER's act, always (ADR-0186 §4.5/§6.5;
   the machine never publishes).
8. **The post-release sync** — the CHANGELOG links, the REALITY line
   STAGED→PUBLISHED (the №646 form).

An unscheduled security cut (the 0.28.1 shape) stays a PERMITTED
exception when a defect cannot wait for the next Monday window — with
the reason recorded in the release notes (the №620/№629 ADVISORY
precedent).

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
