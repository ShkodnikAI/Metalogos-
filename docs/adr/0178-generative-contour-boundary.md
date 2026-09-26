# ADR-0178: The generative contour boundary — experimental, scoped, exit-ready

**Status:** Accepted — the owner's decision 5-B of the strategic gate
(gh#680, 2026-09-25: the generative models move OUT of the core
crates/feature boundary with an explicit ADR; the code is NOT thrown
away). This ADR is the implementation vehicle of that decision
(naryad №464, Wave 17, gh#695 — after №463's stop-list).
**Date:** 2026-09-26
**Naryad:** #464 (issue #685)
**Pillar:** Process — the strategic cycle discipline (decision 5-B)
**Consumers:** the №463 CI stop-list gate (the executable mechanism, §4);
№472 (the future home of the contour — the reflex crate extraction, §3);
№456/№457 (the closed preconditions, §5); the dispatchers and executors
of every future "add a generative capability" naryad (§6).

## 1. Context

The audit 25.09 (§5.2) found that the №166 boundary ("generative models
are out of scope for the core") was silently revised without an ADR: a
CPU transformer lives inside the interpreter, it does not compete with
external LLMs on quality nor with llama.cpp/candle on speed or
ecosystem, and each subsystem keeps adding stateful functions to both
execution backends and new surface to `audit.rs`. The №166 limit leaked
precisely because it had no executable mechanism — a prose boundary is
a wish, not a gate.

The vision's doubt (v1.1, decision 5, doubt 2) is valid and is answered
here rather than argued away: comparing a CPU transformer against
llama.cpp is only relevant if the target workload is prose generation.
If the actual workload is small on-device classification and adequacy
scoring, the comparison is irrelevant and the capability has a
legitimate, bounded home. An ADR that fixes "generative models" as an
abstraction fixes nothing; an ADR that fixes THE TARGET WORKLOAD and
THE MECHANISM fixes the drift.

## 2. Decision

The generative contour (the in-tree CPU transformer path behind the
`candle` feature — the model kinds, the embedding/training surfaces and
their builtins, the weights plumbing) is:

1. **EXPERIMENTAL** — not a supported language pillar; every public
   statement about it must carry the experimental mark
   (`docs/limitations.md` §"Generative contour (experimental)" is the
   single wording source);
2. **BOUNDED by workload** — see the explicit scenario list (§3);
3. **EXECUTABLY frozen** — the №463 CI stop-list manifest
   (`scripts/ci/generative_stop_list_gate.py`, 16 files, 9430 LOC
   baseline) refuses any growth of the contour in CI; extending the
   manifest's file list or the LOC baseline REQUIRES a patch to this
   ADR in the same PR (§4);
4. **EXIT-READY** — the contour moves out of the core into its own
   crate/module (the `reflex-gen` module after the reflex crate
   extraction, №472's roadmap, 0.27+) without language-level rework,
   because the feature gate already isolates the build and the stop-list
   already isolates the surface (§3).

## 3. The target workload — explicit scenarios

The contour EXISTS for exactly these scenarios (the vision's doubt 2,
answered):

1. **On-device classification** — small-input categorization where the
   operator cannot or will not ship an external LLM call (air-gapped
   installs, per-request cost control, latency floor);
2. **Adequacy/quality scoring** — small-model scoring of candidate
   outputs (draft filtering, echo/loop detection) where a 100ms CPU
   budget is acceptable;
3. **Small transformation tasks** — short-horizon rewrites
   (normalization, extraction) where the model fits the CPU budget of
   the deployment.

The contour is NOT for:

- **Prose generation** — the quality bar belongs to external LLMs
  (the `llm` contour, ADR-0037 lineage, the real-backend default since
  №454); an in-interpreter CPU transformer competing on generation
  quality is explicitly OUT of scope;
- **Latency-critical or accuracy-critical decision paths** — the
  contour's outputs are advisory inputs (the same posture as the
  forecast ladder: instrumental, no-advisory);
- **Training pipelines as a product** — `reflex_train` remains the
  learning surface; the generative contour is not a model-training
  product.

## 4. The executable mechanism — the №463 stop-list

The CI job `generative-stop-list (blocking)` runs
`scripts/ci/generative_stop_list_gate.py` against the checked-in
manifest (16 files, the 9430 LOC baseline):

- any NEW file in the contour → CI red;
- any LOC growth in the contour → CI red;
- moving the baseline down (deletions, extraction) → green, and
  WELCOME (the gate's threshold moves only down — the №462 convention);
- extending the manifest (new files, a higher baseline) → REQUIRES a
  patch to THIS ADR in the same PR, naming the owner's decision that
  authorizes it. A manifest change without the ADR patch is a rejected
  PR, not a discussion.

## 5. The preconditions for ANY contour expansion

Before any expansion (new capability, new file, new surface) ALL of:

1. №456 CLOSED — holdout validation, fail-closed NaN, the fallback
   barrier (landed, Wave 16);
2. №457 CLOSED — the strict serve context incl. cron/webhook ticks
   (landed, Wave 16);
3. Background training OUT of the interpreter mutex (the Wave 17
   finding 3.11 — not yet landed; blocks expansion, not the frozen
   status quo);
4. The runtime Secret check in the embedding path (the taint layer's
   embedding seam — not yet landed; blocks expansion);
5. The call budget defined (per-tick/per-request invocation limits for
   the contour — not yet landed; blocks expansion).

The unlanded preconditions (3–5) are tracked here; the freeze does not
depend on them — the stop-list holds regardless.

## 6. The removal/expansion rights — the owner only

The freeze lifts, the manifest grows, or the workload list changes ONLY
by the owner's explicit decision recorded as a PATCH TO THIS ADR
(the same discipline as ADR-0177's unfreeze). An executor may propose;
an executor may not expand. The stop-list gate mechanically refuses
what this ADR does not authorize — the prose and the gate say the same
thing, which is the whole lesson of №166.

## 7. Consequences

- The №166 boundary is back, this time with a CI teeth (the stop-list)
  and a fixed workload (§3);
- The audit's 5.2 finding is closed by policy + mechanism, not by
  deleting code (the owner's 5-B: the code is not thrown away);
- The path to 0.27 is clean: the reflex crate extraction (№472's
  roadmap) picks the contour up along with the reflex domain, and the
  core loses the last stateful domain resident;
- `docs/limitations.md` carries the honest experimental wording — the
  absence of a claimed support is the absence of a lie.
