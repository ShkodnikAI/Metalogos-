# The next-line decision package (№1015, gh#2097 — волна В108)

Recorded: 2026-11-27. The dispatch: gh#2095 (the owner's directive 26.11:
«Проверь что сделано на репо Металогос и заводи следующую волну нарядов
согласно стратегического плана развития языка Металогос»). Purpose: to
prepare the owner's choice of the next line AFTER the Phase 7 closure
(№1014, gh#2096). Zero product code; no phase is opened or promised here —
the opening is a dispatch act by the owner's word. Every number carries its
source; hypotheses are marked; where the materials do not answer, the
question to the owner is named instead of an invented estimate.

## 1. The Phase 5 state (the GPU-budget gate — what is actually behind it)

**What the line's own materials fixate (ADR-0159, accepted 2026-09-22,
gh#617/№354, wave 10):** the embodied contour is sim-only by a loud
boundary («There is no real hardware»); the type surfaces landed (№355:
DeviceHandle/WorldState/ActionChunk/Pose/Trajectory/GoalPredicate carry
SafetyBounds); the monitor DEFINITION is fixed (SafetyBounds is an
STL-formula monitor, not a table); three stages are explicitly BEHIND the
GPU-budget gate — **№356 SimProve stage A** (the STL monitor
implementation), **№357 G-ActPhys stage B** (the physics-injection
harness), **№358 the injection-proof stage C** (the proof corpus run,
defined by №358's contract, its run behind the gate). The real-weights
gate (№294) is PARKED separately.

**What is already usable without any GPU (from the ADR's own reuse
posture):** the monitor evaluates signal traces whatever produced them —
the corpus can be generated against the stage-C telemetry shape; the
carriers differ, the specification does not. Stage A (№356) is a parser +
evaluator over recorded traces: a CPU-shaped work item. Hypothesis
(marked): stage A is executable sim-only on the existing CI hardware
(no GPU needed); stages B/C depend on the physics/traces generator
scale, which the materials do not quantify.

**The GPU cost picture (public sources, the dates named):**

| source | date | numbers |
|---|---|---|
| [runpod.io/pricing](https://www.runpod.io/pricing) | 12.11.2026 | H100 PCIe $2.89/hr Secure Cloud, from $1.99/hr Community, per-second billing |
| [gpufinder.dev/providers/lambda](https://gpufinder.dev/providers/lambda) | 25.11.2026 | Lambda on-demand: H100 $3.79, A100 $1.99, A100-80GB $3.19 per GPU-hour |
| [spheron.network/blog/runpod-h100-pricing-2026](https://www.spheron.network/blog/runpod-h100-pricing-2026) | 22.05.2026 | RunPod H100 SXM5 ≈ $2.99/hr Secure, ≈ $4.55/hr serverless active time |

Reading (hypothesis, marked): for stage A (no GPU needed) the gate cost is
zero. IF the owner wants stage B/C corpus generation on rented GPUs, a
modest campaign (the order of 100–500 GPU-hours) prices at ~$200–1,500 at
the quoted on-demand rates — but the hours estimate is NOT derivable from
the line's materials; treating it as a number would be an invented budget,
so it is named as a question, not a fact.

**The questions only the owner answers (the gate is their input):**
1. Is stage A (№356, the CPU-shaped STL monitor) allowed to run WITHOUT
   lifting the GPU-budget gate (sim-only stays loud)?
2. If stages B/C are wanted: the trace-corpus scale (hours, hardware
   class) — the line has no recorded estimate to reuse.
3. Does the sim platform choice matter (a name to be pinned: SAPIEN,
   Isaac Sim, MuJoCo-style, or a custom trace generator — the materials
   do not name one)?

## 2. The Phase 6 demand re-scan (Spatial / XR)

**The watchlist (WL-01…WL-07, docs/watchlist/registry.md, №993):** no XR
scenario exists among the seven (rust-anything, TradingAgents, toolhive,
auto-ontology, haystack, PraisonAI, superpowers). The W48 report
(2026-11-23, docs/watchlist/reports/2026-W48.md): only WL-01 Yellow
(rust-anything stars 7868→8398, +6.7%); the rest quiet. Nothing XR-shaped
has entered the watchlist since the В105 resolution.

**The public signals (the market grows — on platforms and content, not on
a language):**
- [imarcgroup.com/spatial-computing-market](https://www.imarcgroup.com/spatial-computing-market) (15.09.2026): spatial computing ≈ USD 172.2B (2025), CAGR 21.47% 2026–2034.
- [hololight.com/news/xr-trends-2026](https://hololight.com/news/xr-trends-2026) (17.11.2026): AI is the fastest-growing segment in Mordor's spatial-computing taxonomy (43.56% CAGR); enterprise streaming, not DSLs.
- [uctoday.com/xr-trends-2026](https://uctoday.com/xr-trends-2026) (n.d.): enterprise XR — training, spatial collaboration, frontline performance at scale.

**Verdict (honest):** the demand for spatial COMPUTING is real and
measurable, but none of the scanned signals translates into demand for a
programming-language contour for XR agents — no library/DSL request, no
watchlist entry, no DOI-sample hit (the DOI scan of В105 remains negative;
nothing since contradicts it). Phase 6 stays without a confirmed demand
signal. Hypothesis (marked): if a spatial line is ever wanted, the
watchlist should first grow an XR scenario — a §16.7 booking decision, an
owner's call.

## 3. The candidate phases (from the watchlist, the edge line, the ledger)

1. **Verifier-first agent actions** (the edge line's own continuation).
   Why: the Phase 7 bought a measured, gated verifier junction (attest/
   e2e/bench lanes, the provenance chain, the recurring bench); agentic
   usage is the growth direction (jetbrains.com/research/2026/08/
   ai-coding-agent-adoption-2, 18.08.2026: ~39% of professional developers
   used an AI coding agent at work in May–July 2026, up from 18% in
   January). What it gives: the verifier junction extended from the
   shipped-binary surface to the serve/agent-action path — the checks the
   core passes become the checks the agent path passes. Dependencies: the
   junction is on main; no external budget.
2. **MCP/interop surface** (WL-03 stacklok/toolhive, quiet — an
   infrastructure watch, not a hot signal). Why: the language already
   serves tools (mlog serve/tool surfaces, the server bin); the MCP
   lifecycle toolkit's own growth makes the interop question concrete.
   What it gives: the language callable from the mainstream agent stacks.
   Dependencies: the server surface; a demand check before any build.
3. **Ontology/knowledge automation** (WL-04 NVIDIA/auto-ontology, quiet).
   Why: the memory lane (the KG persist/recall contour) is in-repo; the
   auto-ontology tooling signal (NVIDIA) shows the direction is
   institutional. What it gives: the KG layer driven by declared schemas
   instead of hand-built graphs. Dependencies: the memory lane's health
   (the §1/§2 findings of #967 are RESOLVED, 08.11 revision).
4. **Skills/methodology composition** (WL-07 obra/superpowers, quiet).
   Why: the repo carries the tiered skill-index machinery
   (examples/skill_index_tiered.mlog, the ADR-0058 line); the composable-
   skills methodology is the watch's subject. What it gives: the language
   as the composition layer for methodology packs. Dependencies: the
   skill-index surface; the watch's own maturity.

## 4. The recommendation draft (форма №1599 — the A/B/C for the owner)

- **A (recommended): close Phase 7 by the owner's verbatim (№1014's draft
  in gh#2084) and open the next line as «Verifier-first agent actions»**
  — the smallest line that converts the finished Phase 7 asset into new
  surface; no external budget; the queue after it (the panic-noise mute,
  the stress-lane speedup) folds into its В1 as hygiene.
- **B: close Phase 7 and revive the embodied contour at stage A only**
  (№356, the STL monitor, CPU-shaped, the GPU gate untouched for B/C).
  Middle cost, revives a parked contour with the strongest formal
  machinery; risk: the sim-platform choice and the stage-B/C scale stay
  unanswered (the questions above).
- **C: close Phase 7 and hold the line-opening** — serve the queue only
  (the SHA256SUMS cross-platform extension needs its provenance review;
  the stress-lane speedup; the panic-noise mute). Minimal cost, zero new
  risk, no new surface.

The decision is the owner's act; this package decides nothing.

## 5. Sources (all opened 26–27.11.2026)

- ADR-0159 (docs/adr/0159-sim-first-stl.md) — the phase-5 slot fixation.
- docs/watchlist/registry.md + docs/watchlist/reports/2026-W48.md.
- #967 (the known-issue ledger; the 08.11 machine revision: 6 RESOLVED,
  §4 PARKED — the legacy process-style declarations, the ignore stays).
- The GPU/XR/agentic URLs in the tables above, with dates.
