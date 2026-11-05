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

## The train (ADR-0189, Accepted 2026-10-08 — gh#1148; AMENDED 2026-11-05 — №899/gh#1793)

The release rhythm: the CALENDAR TRAIN (the ADR-0189 amendment §5) —
the work runs Mon–Sat, the candidate is cut on SATURDAY 12:00 MSK, the
office stand verifies on SUNDAY, the final tag sits on the SAME commit
and the owner publishes on MONDAY. **Activated: the first amended
cycle is the week of 2026-11-09 — the candidate `v0.39.0-rc.1` on
Saturday 2026-11-07 12:00 MSK** (the owner's directive on gh#1793). The
window is a working block of the day, not a minute-exact schedule; the
executor prepares everything machine-side, the owner publishes when
satisfied.

1. **Check [Unreleased]** — is there content? An empty `[Unreleased]`
   means the window is SKIPPED without blame (the honest-content rule:
   a train without content does not depart — never a version bump with
   no user-visible change).
2. **Class the version number** — Security/Fixed only → a PATCH cut
   (`0.30.x`); Added/Changed present → a MINOR cut (`0.31.0`). The
   version classifies the CONTENT only: the R38 «subphase = minor
   version» rule is REPLACED (the plan-canon §13.13) — the subphases
   are named in the release notes (the template below).
3. **The gate read** — `unfreeze_gate.py --gate-target <target>
   --strict` must exit 0 for the record the number class names.
4. **The BREAKING read (№865)** —
   `python3 scripts/ci/breaking_change_gate.py` must exit 0: a tag
   range that adds a deny-path name to `GRANT_ALWAYS` /
   `PROFILE_PERMIT_SHARP` / `HUMAN_OVERSIGHT` holds the cut until the
   version section carries a `### Changed` BREAKING row naming the
   carriers and the migration path ([docs/upgrade.md](upgrade.md) —
   the 0.36.0/0.38.0 lesson: the deny rule shipped as `### Added` and
   broke the 0.34-era programs at the first call).
5. **No open release-block** (ADR-0179 §4) against the target — an open
   one holds the train; the hold is recorded in the owning wave's
   dispatch thread.
6. **The lockstep PR** (the №646 shape: version + `Cargo.lock` ×5 +
   badge sync + the generated Version line + the CHANGELOG date-stamp),
   CI green, squash-merge.
7. **The Saturday candidate** — the rc tag `vX.Y.0-rc.N` on the merge
   commit; the machine cut: a `workflow_dispatch` on release.yml
   (`tag` = the rc tag, `upload` = true) — the DRAFT PRERELEASE with
   the binary, the SBOM, the attestations and `SHA256SUMS` attached;
   the train gate reads the numbering before any asset is built. The
   machine never publishes — the draft sits until the owner acts.
8. **The Sunday stand** — the office stand (FOSVED-office-v2) pins the
   rc draft: the e2e battery, the `mlog check` baseline. A defect →
   the fix → `rc.N+1` (the numbering gate enforces 1..N without gaps).
9. **The Monday final** — the final tag `vX.Y.0` on the SAME commit the
   stand verified (the rc↔final SHA equality is machine-enforced:
   `scripts/ci/release_train_gate.py` blocks the release build and the
   ci.yml lane); the publication is the OWNER's act, always
   (ADR-0186 §4.5/§6.5; the machine never publishes); the assets ride
   the `release: published` build (the №497 path).
10. **The post-release sync** — the CHANGELOG links, the REALITY line
    STAGED→PUBLISHED (the №646 form).

### The release-notes template (the minor trains — the amendment §5.2)

Every minor-train release body carries the two blocks (the body is the
CHANGELOG section verbatim — `scripts/ci/release_notes.py`; the blocks
live in the version section of the CHANGELOG):

```markdown
Вошедшие подфазы: <the Phase-8 subphases carried by this train; "none" if empty>

Проверено на стенде офиса: <the date and the verdict — the e2e battery
plus the `mlog check` baseline over the rc candidate; "not verified" =
the train does not depart>
```

The block TITLES are verbatim Russian (the owner's template on gh#1793
— they surface in the published release notes as-is); the placeholders
are described in English here, filled in the actual notes. The R38
replacement in one line: the subphase identity lives in the release
notes of the train that carries it, never in the version number.

An unscheduled security cut (the 0.28.1 shape) stays a PERMITTED
exception when a defect cannot wait for the next window — with the
reason recorded in the release notes (the №620/№629 ADVISORY
precedent); a patch cut does not require an rc candidate.

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
