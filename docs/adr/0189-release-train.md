# ADR-0189: The release train — the Monday cadence, the window, and the honest-content rule

**Status:** Proposed — the cadence is ACCEPTED by the owner only on the §-answer (the executor drafts; the owner fixes)
**Date:** 2026-10-08
**Naryad:** №654 (issue #1148, Волна 38)
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
