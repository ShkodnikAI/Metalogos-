# ADR-0189: The release train — the Monday cadence, the window, and the honest-content rule

**Status:** Accepted — the owner's verdict, 2026-10-08; **AMENDED
2026-11-05** (№899, gh#1793 — the owner's calendar-train decision: the
Saturday rc candidate, the Sunday office stand, the Monday publication;
the R38 «subphase = minor version» rule is REPLACED by the plan-canon
§13.13). §5 below is the amendment; §2.1/§2.2 read through it. (The
original verbatim authorization core: «Каденс поезда принимаю (Proposed → Accepted): окно
— понедельник»; the FULL verdict text lives at the source — gh#1148,
comment 2026-10-08T08:03:41Z, and the canonical «гейты-0810» record п.4
— the docs-language lint №383 keeps the .md form English-only, the
ADR-0181 §1 authorization-record shape).)
The window rule, the
no-empty-train rule, the security-cut exception and the first-window
candidate (12.10: a `### Security` entry in [Unreleased] → 0.30.1) are
accepted AS WRITTEN below.

**Date:** 2026-10-08
**Naryad:** №654 (issue #1148, Волна 38); the acceptance landed by №663 (issue #1164, Волна 39)
**Depends on:** ADR-0179 (the release criteria v2, release-block §4), ADR-0186 (the 0.30 gate — §4.5/§6.5: the publication is the OWNER's act), docs/release-checklist.md (№509 — the named steps between the evidence and the decision), №614/№583/№646 (the lockstep release shape), №562/№643 (the release-gap gate and the CHANGELOG dictionary)
**Blocks:** the first train window (the first candidate: Monday 2026-10-12)

## 1. Context

The unified audit of 48301708 (§6.4 R3) observed that the release line ran
OUTSIDE any cadence: four releases in four consecutive days —

- 0.28.1 on 2026-10-05,
- 0.28.2 on 2026-10-06,
- 0.29.0 on 2026-10-06,
- 0.30.0 on 2026-10-08 —

each with a different ad-hoc trigger (an emergency cut, a gate
completion, a movement completion, a wave completion). The dispatch
gh#1136 (the №646 flag 5) deferred the train decision until after the
0.30.0 publication; 0.30.0 is PUBLISHED (2026-10-08 06:55Z), so this
window is open now.

The release machinery itself is already mechanical and machine-read:
the lockstep PR (version + `Cargo.lock` ×5 + badge sync + the generated
Version string — the №614/№583/№646 shape), the `--strict` gate read
(`unfreeze_gate.py --gate-target <target> --strict`, exit 0 on the
0.30 record), the release-gap gate (№562 — Security entries in
[Unreleased] never stick outside a release), the CHANGELOG dictionary
(№643), the tag, the build (release.yml — the assets + the SBOM + the
attestations), and the publication as the OWNER's act (ADR-0186 §4.5).
What is missing is a fixed RHYTHM and a rule that keeps the rhythm
honest.

## 2. Decision (the proposal)

### 2.1 The cadence: Monday

The train deploys on MONDAYS. The first candidate window: **2026-10-12**.
The window is a working block of the day, not a minute-exact schedule;
the executor prepares everything machine-side, the owner publishes when
satisfied.

### 2.2 The window (the machine-side steps, in order)

1. **Check [Unreleased]** — is there content? (See 2.4: empty means SKIP,
   not a forced release.)
2. **Class the version number** (see 2.5).
3. **The gate read** — `unfreeze_gate.py --gate-target <target> --strict`
   for the relevant record(s) must exit 0.
4. **No open release-block** (ADR-0179 §4) against the target — an open
   one holds the train (see 2.6).
5. **The lockstep PR** (the №646 shape: version + Cargo.lock ×5 + badge
   sync + the generated Version line + the CHANGELOG date-stamp), CI
   green, squash-merge.
6. **The tag** on the merge commit.
7. **The build** (release.yml — the assets, the SBOM, the attestations).
8. **The publication** — the OWNER's act, always (ADR-0186 §4.5/§6.5;
   the machine never publishes).

### 2.3 The honest-content rule: no empty train

**A train without content does not depart.** If, on the window, the
`[Unreleased]` section carries no Fixed/Added/Security entries (and no
pending owner-act unblocks one), the window is SKIPPED without blame —
the cadence exists to discipline the release process, never to
manufacture releases. An empty release (a version bump with no user-
visible change) is exactly the Goodhart failure this ADR refuses.

### 2.4 The version-number semantics

- **Security / Fixed entries only** → a PATCH cut: `0.30.x`.
- **Added / Changed entries present** → a MINOR cut: `0.31.0`
  (whether the 0.31 gate parameters are fixed by then is a separate
  owner question — the №525 path; the train does not wait for the gate
  fixation to ship Added content, the gate read applies to whatever
  record the number class names).

### 2.5 The release-block hold

An OPEN release-block issue (ADR-0179 §4) against the target holds the
train: the window may pass the mechanical steps and still not depart
until the block closes. The hold is recorded in the dispatch thread of
the wave that owns the block.

### 2.6 The unscheduled security cut stays legal

The 0.28.1-style unscheduled security cut remains a PERMITTED exception
when a defect cannot wait for the next Monday window — with the reason
recorded in the release notes (the №620/№629 precedent: the ADVISORY
form). The train is the default, not a cage.

## 3. The checklist wiring

`docs/release-checklist.md` gains a «The train» section: the same
machine-readable steps as §2.2, as the named items between the evidence
and the owner's decision — the checklist remains the single
release-time SSOT.

## 4. Consequences

- The release rhythm is predictable for consumers (the Камертон-class
  external contour included) and for the wave machine (a wave merged by
  Friday rides the next Monday window, or the one after — no ad-hoc
  pushes).
- The empty-window skip keeps the honesty: the metric here is not
  «releases shipped», it is «releases shipped WITH content».
- The release-block hold ties ADR-0179's §4 into the schedule: a block
  is never outrun by a calendar.
- Nothing in this ADR changes the publication authority: the owner
  publishes, the machine prepares (ADR-0186 §4.5 stands).

## 5. The amendment — the calendar train (2026-11-05, №899)

**Decision source:** the owner's calendar-train decision of 2026-11-04,
recorded in [gh#1793](https://github.com/ShkodnikAI/Metalogos-/issues/1793)
(№899); the plan-canon §13.13 is the registry-side record. The
foundation facts: the R38-era rule «one subphase = one minor version»
produced FOUR minor releases in three days (30.10–01.11) and the 0.37
number skip; the №861-class serve regression (gh#1735 — the
`allow_carrier` loss in cron ticks and routes) was caught only by the
office's own stand AFTER a publication. The train gains a verification
stage BEFORE the publication.

### 5.1 The weekly cycle (replaces §2.1–§2.2 for every week after 2026-11-03)

| Day | Step |
|---|---|
| Mon–Sat | the work; the number of waves does not set a release |
| **Sat 12:00 MSK** | the candidate `vX.Y.0-rc.N` cut from main (the machine: the `unfreeze_gate.py --strict` gate read, the lockstep, the rc tag, the DRAFT prerelease with the assets — release.yml, the rc path) |
| **Sun** | the office-stand verification: the rc pinned on the stand (FOSVED-office-v2), the e2e battery, the `mlog check` baseline; a defect → the fix → `rc.N+1` |
| **Mon** | the final tag on **the same commit**; the publication is the OWNER's act (ADR-0186 §4.5 stands) |

### 5.2 The rules

- **The empty week departs nothing** (§2.3 stands): no content in
  `[Unreleased]` → no rc, no release, no blame.
- **A red Sunday skips the week** — without a Tuesday carry: the defect
  that failed the stand holds the train to the NEXT week's cycle (a
  forced Tuesday publication is exactly the ad-hoc trigger §1 refused).
- **The numbers run consecutively** — a new minor tag equals the
  previous minor + 1; a gap is RED (the machine gate:
  `scripts/ci/release_train_gate.py`, blocking in ci.yml and in the
  release build). The rc numbers run consecutively within a train too
  (`-rc.1`, `-rc.2`, …).
- **The final tag does not rebuild a different commit** — the final
  `vX.Y.0` tag's SHA must equal the SHA of the LAST `vX.Y.0-rc.N`
  candidate of the same train (the commit the stand verified); a
  mismatch is RED. Security and P0 fixes stay patch cuts on ANY day
  (§2.6 stands) — a patch cut does not require an rc.
- **R38 is replaced**: the subphase→minor-version binding is cancelled
  (the plan-canon §13.13). Phase-8 subphases are enumerated in the
  release notes of the weekly train that carries them — the notes
  template block «Вошедшие подфазы» (docs/release-checklist.md). The
  version number classifies the CONTENT (§2.4 stands: Fixed → patch,
  Added/Changed → minor), never a subphase identity.

### 5.3 The machine wiring (this amendment's deliverables)

- `release.yml`: the `-rc.N` tags are FIRST-CLASS — the same
  assets/SBOM/attestations; the rc cut creates a DRAFT prerelease with
  the assets attached (the machine never publishes — the draft sits
  until the owner acts); `BUILD-INFO.txt` records the prerelease
  posture. The final-tag build runs the train gate BEFORE any asset is
  produced.
- `scripts/ci/release_train_gate.py` — the numbering gate (no gaps;
  the rc↔final SHA equality) with the `--self-test` fixtures; blocking
  in ci.yml (push/PR, full tag history via `fetch-depth: 0`) and in the
  release build.
- `docs/release-checklist.md` — the train section carries the calendar
  cycle and the release-notes template blocks («Вошедшие подфазы»,
  «Проверено на стенде офиса: <дата, результат>»).

### 5.4 Consequences of the amendment

- The publication is preceded by a VERIFIED candidate on the consumer's
  own stand — the gh#1735 class (a serve regression published before
  any stand contact) is structurally closed.
- The version line stays monotonic and gapless; the 0.37-class skip
  becomes a machine-caught impossibility for the post-amendment trains.
- The subphase identity moves out of the version namespace: the minor
  number measures the content class, the release notes name the
  subphases.
