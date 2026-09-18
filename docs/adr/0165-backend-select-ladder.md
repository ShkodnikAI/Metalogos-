# ADR-0165: BackendSelect — the backend ladder and Degraded(t), typed degradation

**Status:** Accepted
**Date:** 2026-09-16
**Naryad:** №336 (issue #463, Волна 2 · Фаза 2)
**Depends on:** №333 (backend registry — classes, SHA-pin, licenses, ADR-0163), №334 (real backends — the SHA-pin path, mock-first call contract), №374 (try → Struct{ok, value, error}, ADR-0142 — the typed-result precedent), №375 (the mock mode is loud, never silent)
**Blocks:** Волна 2 Go/No-Go item 5 («исчерпание лестницы → Degraded(t), не падение и не тихий мок»)

## 1. Context

The №333 registry says what a backend IS; №334 gave three classes real call surfaces. What the language still lacks is SELECTION with fallback: today the absence of a backend either silently degrades to a mock (`METALOGOS_LLM_MOCK` defaults true) or fails loudly. A program that prefers `whisper-turbo` but tolerates another STT backend cannot say so — there is no try-chain and no typed way to report "every rung failed" other than a crash. The spec's ladder (§7.6.3) and device profiles (§11.2) require exactly two things: a priority-ordered try-chain over registry entries, and a typed degradation result when the ladder is exhausted — not a panic, not a silent mock substitution.

## 2. Decision

### 2.1 `backend_select(class, ladder)` — the try-chain (a builtin, not syntax)

`backend_select` is a builtin (`src/builtins/backends.rs`, registry category), callable from both backends through the shared registry dispatch:

```
let sel = backend_select("stt", ["whisper-turbo", "kokoro"])
if sel.ok { print(stt_transcribe(audio, sel.backend)) } else { print("degraded: " + sel.error.message) }
```

- `class` — one of the §7.6 class words: `stt | tts | omni | vision-understanding | llm`. An unknown class word is a loud Err (catchable by `try`, the ADR-0142 shape).
- `ladder` — a non-empty list of REGISTRY backend names (the program-visible identifiers, `find_by_name`), priority order = list order. Duplicates are a runtime error (a rung tried twice is a contract bug, not a fallback).
- Every rung is ATTEMPTED and every attempt is an audit event (№326 posture): a `[BACKEND_SELECT]` stderr line per rung plus the program-visible `attempts` list in the result. No silent skips — the ladder's trace is data.
- Success → `Struct { type_name: "BackendSelected", ok: true, backend, weights_id, mode, attempts }` where `mode` is `"mock"` (deterministic mock-first mode, №334) or `"real"` (weights verified on disk). The selection is metadata: the class callables (`stt_transcribe`, `omni_ask`, …) still execute the call.
- Exhaustion → **Degraded(t)** (§2.2). `backend_select` never panics and never substitutes a mock for a failed rung.

### 2.2 `Degraded(t)` — the typed degradation result

When every rung is unavailable, the result is a typed value, not an error throw and not a downgrade:

```
Struct { type_name: "Degraded", ok: false, class: <t>, attempts: [...], error: Struct { code: "BACKEND_DEGRADED", message } }
```

- `t` = the backend class word the ladder served (the "type" of what degraded). `ok: false` matches the try-struct convention (ADR-0142), so `if sel.ok` is the uniform guard.
- `error.code = "BACKEND_DEGRADED"` — a stable diagnostic code in the ADR-0131/0140 convention (UPPER_SNAKE_CASE, no-reuse). The `message` is prose and may change; the code is the contract.
- A rung attempt is `Struct { backend, status: "selected" | "unavailable", reason }`; `reason` names exactly what was missing (no registry record / class mismatch / weights not fetched — the №334 refusal wording).

### 2.3 Availability and the loud mock boundary

- Mock mode (`METALOGOS_LLM_MOCK` unset/true — the №4 default): every registry rung of the requested class is available through the deterministic mock; the result's `mode` is `"mock"` — visible in the data, never hidden.
- Real mode (`METALOGOS_LLM_MOCK=false/0`): a rung is available only when its weights are fetched and SHA-verified (the №334 contract). Today, with the PARKED boundary (№294 — no weights in this environment), real-mode rungs are unavailable and the ladder honestly exhausts to `Degraded`. A mock NEVER substitutes a rung in real mode — that is the naryad's "мок-фоллбек только в mock-режиме (громко)" rule, tested.

### 2.4 The device profile: build-time ladder verification

`profile device { mode: production | development }` joins the compat-profile family (№325 `legacy`, №333 `licensing` — independent flags, loud validation of unknown words):

- `development` (default, absent profile) — no static ladder constraints.
- `production` — every statically-visible ladder rung must be SHA-pinnable: `ShaPin::PendingNo334` backends are UNVERIFIABLE for a production profile, and a ladder that names one fails COMPILATION (`check_backend_select_ladders` in `semantic.rs`, wired into `check_program`). This is the §11.2 build-time rule with the companion-check precedent: the ladder is data a program declares, so the compiler verifies it against the registry SSOT.
- Static verification of a literal `backend_select("cls", ["a", "b"])` call site covers: unknown class word, unknown rung name (no registry record), rung class ≠ requested class, duplicate rungs. Non-literal ladders are not statically verifiable and stay with the runtime checks (documented, loud at run time).

**Loud boundary:** the full §11.2 device matrix (memory/accelerator tiers per device class) is NOT in the registry yet — the only constraints the tree can honestly verify today are pin verification, class membership, and license class. Tiers land when registry entries carry hardware requirements; this ADR fixes the verification MACHINERY, not the matrix.

### 2.5 Rejected alternatives

- **Throw on exhaustion** (Err instead of a Degraded value): rejected — a degraded-but-running program is the normal operation mode for device ladders; forcing `try` around every selection conflates "the ladder exhausted" (an expected, typed outcome) with "the call was malformed" (an error).
- **Silent mock fallback** (the pre-№336 posture): rejected — the exact failure this naryad removes; a mock must never impersonate a real rung in real mode.
- **Syntax-level `try-chain` construct**: rejected — the ladder is runtime data over a static SSOT; a builtin + a companion check gives the same static guarantees without grammar growth (parser rule count unchanged at 313).

## 3. Consequences

- `backend_select` joins the №316 SSOT classification (Source — it reads registry metadata and returns selection data; internal default label; Pure reversibility — no egress).
- The result structs (`BackendSelected` / `Degraded`) are plain `Value::Struct` — inspectable, printable field-wise (HashMap field order is nondeterministic; print fields, not the whole struct).
- VM parity is free: both backends dispatch through the shared registry handler.
- The attempts list makes every ladder step an audit surface; the stderr events keep the op-log posture of №326.
- `Degraded` joins `TryError` as a named error-shape struct; the code namespace (`runtime`) per ADR-0142.

## 4. Verification

- `tests/naryad_336_backend_ladder.rs`: selection picks the first available rung (mock mode, both backends); exhaustion → `Degraded` with `ok: false`, class preserved, every rung attempted in `attempts`; every rung emits an audit event; real mode never returns a mock (weights missing → unavailable → Degraded); unknown class / duplicate rungs are loud runtime errors; static companion check — unknown rung, class mismatch, and production-profile + `PendingNo334` rungs are compile errors; `examples/w1_degrade.mlog` runs the full cycle on both backends.
- No-stubs discipline (№16.0-D): grep `todo!`/`unimplemented!`/`SKELETON` — 0; CI blocking green on the merge commit.
