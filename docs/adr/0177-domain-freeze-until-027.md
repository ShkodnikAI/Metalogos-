# ADR-0177: The domain freeze until 0.27 — no new subsystems, the core first

**Status:** Accepted — the owner's decision 1-A of the strategic gate
(gh#680, the owner comment 5828075063, 2026-09-25: "1-A" with the
exceptions and the unfreeze discipline fixed in the gate thread). This
ADR is the implementation vehicle of that decision (naryad №461, the
FIRST naryad of Wave 17 — every other Wave 17 naryad merges after it).
**Date:** 2026-09-25
**Naryad:** #461 (issue #682; Wave 17, gh#695 — the implementation of the
7 strategic decisions)
**Pillar:** Process — the strategic cycle discipline (the audit 25.09
recommendation "not a single new subsystem per cycle")
**Consumers:** every naryad dispatcher and executor (the template line in
§7); the release gate of 0.27.0 (§6); gh#680 (decision 1-A); №462/№465/
№466/№467/№468 (the unfreeze criteria machinery); PLAN-SUMMARY (the
public line); the AI Act Art. 50 slice (the only feature exception, §5)

## 1. Context

September 2026: ~30k+ lines went into domains (image / video / voice /
OCR / forecast / robotics) across 726 commits and 217 merged PRs, while
the language core did not move: types are still strings, the grammar
carries field-order quirks, and the TW/VM duplicate-name count grew
27 → 48 with the VM as the default production backend of `mlog serve`
since 2026-09-21. All three High findings of the audit 25.09 sit exactly
on the core↔domain stitches (the silent mock-LLM default, `read_file`
in routes, distillation without holdout validation), and the architecture
assessment dropped 5 → 4.5.

The executor-agent mechanics amplify the drift: a conscientious agent
WILL extend a domain when the tasking does not literally say "frozen" —
a freeze without a formal ADR and a line in the tasking template leaks
within a week through the mechanics of task framing (vision v1.1, doubt 2
of decision 1; gate gh#680 §1).

Decision 1-A therefore freezes new domain work for one cycle and points
the entire capacity at the core.

## 2. Decision

From the acceptance of this ADR until the 0.27.0 release (the window,
§6), NO new domain work is started and NO existing domain is extended:

- **Frozen — new subsystems of any kind** (a new builtin family, a new
  feature flag, a new media/perception/embodied surface, a new external
  backend class);
- **Frozen — extensions of the existing domains:** image, video, voice,
  OCR, forecast, robotics (new builtins, new formats, new providers,
  new pipeline stages inside the domain).

**Allowed inside the frozen scope (always):** bug fixes, tests, docs,
refactors that do not grow the surface, and security fixes (§5).

## 3. What the freeze does NOT touch

- The planned canon phases 2–7 (media handles, capability, memory,
  embodied sim-only, spatial, edge) are planned LINES of work, not
  domain extensions — they are unaffected by this freeze.
- Wave 16 (the security wave, gh#681) is unaffected: it fixes the
  existing surface, it opens nothing.
- Wave 17 itself (gh#695) is the core-and-process wave this freeze
  enables; its naryads do not open domains.

## 4. The unfreeze criteria (the owner's reinforcement — explicit in text)

The freeze is lifted when ALL of the following are green:

1. **Types:** `enum Type` stages 0 AND 1 executed (№467 stage 0 — the
   typed signatures in the builtins registry without a runtime switch;
   stage 1 — the checks live) and the CI typing metric grows;
2. **Dedup:** the TW/VM duplicate-name count is at or below the CI
   threshold (№462 pins the threshold at the 48 fact) AND the threshold
   has moved only downward (№465 diff-fuzzer → №466 the per-group
   transfer);
3. **Debt:** the CI debt gate (№468 — ignore/dead_code/duplication
   counters against the fact) is green;
4. **Memory:** the memory office E2E dogfood (office#373, FO-056) is
   green — it already is on 0.24.0 and 0.25.0.

**The right to lift the freeze belongs to the OWNER ONLY.** An executor,
a verifier, or a wave report may RECOMMEND lifting; none may lift.
Lifting is recorded by the owner in the gate thread (gh#680) and, when
it happens, this ADR's Status is updated by a naryad, not silently.

## 5. The exceptions (exhaustive — not extendable by an executor)

1. **Security fixes inside the frozen domains — always allowed.** A
   security defect in image/video/voice/OCR/forecast/robotics is fixed
   on sight (the audit 25.09 lesson: the domain stitches are where the
   High findings live). The fix lands with its tests and does not grow
   the feature surface.
2. **AI Act Art. 50 synthetic-content marking (the C2PA slice,
   ADR-0152/ADR-0166):** the external regulatory deadline (the grace
   ends 02.12.2026) overrides the freeze — the marking path is brought
   to the regulatory minimum and then re-frozen. "Regulatory minimum"
   is the manifest write path for generated media; anything beyond the
   marking contract stays frozen.

No other exceptions exist. An executor that believes an exception is
needed escalates to the owner (a stop-condition comment with
evidence); the executor does not extend the list.

## 6. The window and the release gate

- **Window:** from the acceptance of this ADR until the 0.27.0 release.
- **Release gate:** 0.27.0 is NOT published while any of the §4
  criteria is red. 0.27.0 is the release OF the unfreeze — it is not a
  Wave 17 artifact; it happens when the core work the freeze bought is
  done and verified. Every §4 criterion carries a CI or office check —
  the gate reads evidence, not intentions.

## 7. The tasking-template line (the enforcement mechanics)

Every naryad dispatcher adds to the tasking when a request touches a
frozen area:

> domain X is frozen (ADR-0177) — the naryad is not opened

The line is recorded in the ADR index (docs/adr/README.md) so the
dispatcher meets it while numbering. Opening a domain-extension naryad
without an explicit owner override recorded in the tracker is a process
violation — the verifier rejects the dispatch, not the naryad text.

## 8. Consequences

- The September pattern (~30k domain lines / cycle) stops; the next
  cycle's capacity lands in types, dedup, debt and the core stitches.
- Demo/grant-facing feature velocity dips for one cycle — the accepted
  price of the decision (the owner chose 1-A over the soft quota 1-B).
- The frozen domains keep their shipped guarantees: the freeze stops
  NEW surface, not the maintenance of what exists (§5).
- The unfreeze criteria are themselves Wave 17 deliverables (№462,
  №465–№467, №468) — the wave that executes the freeze also builds the
  instruments that measure its end.
