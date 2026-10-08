# ADR-0190: metalogos-examples — the showcase charter (three apps with e2e, for the language consumer)

**Status:** Accepted — the owner's §-answers, 2026-10-08 (the verbatim
answer map: «1 — отдельный публичный репозиторий; 2 —
metalogos-examples; 3 — как в основном репо; 4 — в NLnet-заявке»; the
FULL verdict text lives at the source — gh#1149, comment
2026-10-08T08:03:43Z, and the canonical «гейты-0810» record п.5 — the
docs-language lint №383 keeps the .md form English-only, the ADR-0181
§1 authorization-record shape). The placement option **(а)** (a
separate public repository) is FIXED by the answer (1) — the owner
creates the repository (the OWNER's act, the boundary §4 keeps); the
executor's (б) recommendation is superseded, and the showcase
implementation rides a wave AFTER the repository exists.

**Date:** 2026-10-08
**Naryad:** №655 (issue #1149, Волна 38); the acceptance landed by №663 (issue #1164, Волна 39)
**Depends on:** ADR-0182 (the docs-only contour precedent — contour + §-answers → Accepted → the implementation wave), the serve-e2e contour (`scripts/ci/serve_e2e_inventory.txt` + run_test_server, both-backend), mlogpkg (the №567 workspace member), the LLM mock contour (`METALOGOS_LLM_MOCK` — the штатный test circuit), the Камертон-class consumer (the В30 external contour; the Charter §1 directive: web, bots, agents)
**Blocks:** the showcase implementation wave (after the §-answers)

## 1. Context

The repository holds a large example library — `examples/` (497 files,
the golden sidecars, the GFI #951 line) — but it is written for the
COMPILER DEVELOPER: the files exercise constructs, edge cases and
regressions, and they assume the reader is reading the tree. There is no
place that answers the language CONSUMER's first question: «what can I
build with this?» — a small, curated, runnable set of applications with
the honest end-to-end proof.

The unified audit of 48301708 (§6.4 R3, the third element) proposes a
showcase: **three applications with e2e**, the target consumer being the
Камертон-class external contour. This ADR is the charter — the same
docs-only contour shape as ADR-0182: the contour, the open §-questions,
the owner's answers → Accepted → the implementation wave.

**The honest stock-taking.** The showcase does NOT need new language
features and does NOT lift the domain freeze (ADR-0177): the three
candidate apps below are buildable on the 0.30.0 surface as it stands —
serve + memory/recall (the stateful e2e contour is `done` in the
№509 inventory), the mlogpkg bot surface (poll/hook), and the LLM call +
tools surface (the mock circuit makes the e2e deterministic without the
network).

## 2. The three showcase applications

### 2.1 App A — the stateful serve application

A small web service with state: `memory { persist }` accumulation +
`recall` reads + the routes serving them, run through the
`run_test_server` e2e harness on BOTH backends (the restart-persistence
leg included — the shape the №520/№522 serve-e2e tests already pin for
the language, here retold as a USER story).

- **Volume:** one directory, one `.mlog` program (≤ ~120 lines), one
  e2e script; the bounded subset: memory/recall/kv + serve routes + the
  persistence env (`METALOGOS_MEMORY_DB`, `METALOGOS_MEMORY_MASTER`).
- **e2e oracle:** the recorded request/response arc (persist → query →
  restart → query) with the expected outputs; both backends must answer
  identically on the shared subset.
- **CI contour:** the blocking conformance-style run on the PR; the
  long restart/stress legs in the nightly.

### 2.2 App B — the bot application (mlogpkg)

A chat-bot skeleton on `mlogpkg`: the poll/hook intake + a small dialog
state machine in mlog + the external API client call — the external
world is REPLACED by a deterministic stub (the mock circuit), so the e2e
runs offline and green in CI.

- **Volume:** one directory; the bounded subset: mlogpkg surfaces +
  hook/poll + one outbound client call behind the stub.
- **e2e oracle:** the scripted dialog (in → state → reply) with the
  expected transcript.
- **CI contour:** the blocking run on the PR; nothing network-bound
  enters blocking at all (the honesty boundary — no flaky external
  dependencies in the showcase).

### 2.3 App C — the small LLM agent

A transparent agent loop: one model call + a tool call + the visible
action trace («what the agent did» printed). Runs under
`METALOGOS_LLM_MOCK` — the deterministic canned-model circuit; no live
provider in the showcase CI.

- **Volume:** one directory; the bounded subset: the llm call family +
  one tool + the trace surface.
- **e2e oracle:** the expected transcript of actions and the final
  answer under the fixed mock script, identical on both backends.
- **CI contour:** the blocking run on the PR; a live-provider demo is
  OUT of scope for CI (documented as a manual local run).

### 2.4 The showcase honesty rule: parity as the display requirement

**An application in the showcase runs on BOTH backends (TW/VM) on its
bounded subset — and the e2e must prove it.** The №629/№651 lineage is
the motivation: a cross-backend divergence found in a showcase app is
the loudest possible finding; the showcase must never demo a surface
that the two backends disagree on. The bounded subset of each app is
chosen exactly so that this parity holds TODAY (the honest boundaries —
§16.0-3(в)); widening an app's subset is a small PR with the both-
backend proof.

## 3. The placement: two options (the §-question)

- **(а) A separate public repository `metalogos-examples`.** Pros: an
  independent release rhythm, a clean consumer-facing surface, no
  example-driven pressure on the core CI budget; the core repo stays
  compiler-first. Cons: a second CI to maintain; the sync question (the
  showcase breaks when the core changes — needs a pinned core version
  or a scheduled cross-repo run).
- **(б) In-tree `showcase/` (or `examples-apps/`) directory.** Pros: one
  CI, one release rhythm, the showcase CI rides the existing blocking
  machinery immediately; no new repo to create (no extra owner act).
  Cons: the consumer-facing surface sits inside a developer-heavy
  repository; the core CI budget carries the showcase.

**The executor's recommendation:** (б) in-tree first — the parity rule
(§2.4) is enforced for free by the existing blocking machinery, and the
separate repository can be spun off LATER as a pure copy once the
content stabilizes. The §-question for the owner: the placement AND the
three app candidates (accept / swap / reorder).

## 4. What this ADR does NOT do

- It does NOT create the repository/directory, write the apps, or move
  any `examples/` file — the creation is the OWNER's act; the
  implementation is a wave after the §-answers (the ADR-0182 shape).
- It does NOT touch the GFI #951 line (the golden sidecars for the
  existing `examples/` library) — the developer-facing library keeps
  its own track (№561: the conveyor does not naryad it).
- It does NOT introduce new language constructs or builtins: an app
  that would need one is out of scope until the construct exists.
