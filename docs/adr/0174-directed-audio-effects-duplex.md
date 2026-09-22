# ADR-0174: Directed audio effects (`listen`/`speak`) and the duplex channel — barge-in over the session priority ladder

**Status:** Accepted
**Date:** 2026-09-22
**Naryad:** #352 (issue #595; dispatch #598, wave 9 — registry Phase 4 "Always-on, память, забывание")
**Pillar:** voice (cross-cutting: effect-trail typing + session priorities + ledger)
**Consumers:** №348 (the session priority ladder this ADR consumes), №324/ADR-0154 §9 (the effect-trail machinery that makes direction conflicts compile errors), the office duplex loops (§10.2 of the plan — the agent that can be interrupted while speaking or listening), ADR-0143–0146 (the voice gates this ADR must not weaken)

## 1. Context

The voice contour is real (№302 skeleton; №334 backend registry; `stt_transcribe`/`omni_ask`, `tts_generate`/`tts_send`/`tts_speak` — mock-first, real-weights PARKED №294), but it is HALF-duplex: listen and speak are each a single undirected `io` effect, simultaneity is a runtime discipline at best, interruption is not typed, and audit events can be lost in the churn. №348 landed the typed priority ladder (`low < normal < high < critical`) on sessions; §10.2 of the office plan needs a barge-in loop: the agent must be interruptible WHILE speaking (user takes the floor) and WHILE listening (higher-priority event), with the interrupted flow ending typed — a value or an interruption error, never a panic — and with no audit event lost.

№324 (ADR-0154 §9) landed the effect trail: a pattern DECLARES its effects (`⟨io, audit⟩`) and the semantic gate enforces factual ⊑ declared — a body that does more than its signature is a compile error. The vocabulary today is `io | audit` — direction-blind.

## 2. Decision Drivers

1. **Simultaneity is a TYPE, not a runtime habit.** "Конфликт направлений без типов — ошибка компиляции": a pattern that uses a direction it did not declare must not compile. The №324 gate already implements exactly this shape of contract — the cheapest correct mechanism is to make the directions VISIBLE to it.
2. **The priority ladder is the session's (№348), not a new one.** Barge-in decisions consume `INTERRUPT_PRIORITIES` ranks; no second vocabulary.
3. **Nothing lost on interruption.** Every transition of a duplex channel — open, start, preempt, stop — is a ledger record; preemption emits the preempt record AND the preempted stream's typed terminal outcome. The acceptance test pins the count differentially.
4. **No new voice machinery.** No codecs, no backends, no wake-word logic (№348 owns wake); the duplex runs on the mock/registry contour (real-weights omni stays PARKED №294 — a loud boundary in the report).
5. **Backward compatibility by construction.** Only DECLARED trails are gated (the №324 design); no existing pattern declares the new words, so the vocabulary extension cannot break the existing corpus.

## 3. Decision

### 3.1 The directed effect words

`Effect::Listen` (word `listen`) and `Effect::Speak` (word `speak`) join the ADR-0154 §9 vocabulary (parse-time shape unchanged — bare words in `⟨…⟩`; semantic-level validation extended). The direction table (the SSOT lives in `semantic.rs` beside the №332 media-producing table):

| Direction | Builtins |
|---|---|
| `listen` (audio in) | `stt_transcribe`, `whisper_transcribe`, `voice_enroll`, `listen_start`, `listen_stop` |
| `speak` (audio out) | `tts_generate`, `tts_send`, `tts_speak`, `speak_start`, `speak_stop` |
| both | `omni_ask` |

The channel-state builtins (`duplex_open`, `duplex_state`) carry NO direction — opening a channel and reading its state are direction-neutral registry operations. `speak_stop`/`listen_stop` are SEPARATE builtins (not a direction-argument one) because the direction must be static for the trail to type it.

### 3.2 The compile contract

The effect words flow through the existing `builtin_effects` → walk → trail gate. Consequences, all pinned by tests:

- a pattern calling `tts_speak` and declaring `⟨io, audit⟩` FAILS to compile (excess `speak`);
- a listen-only pattern (`⟨io, audit, listen⟩`) that also calls a speak builtin FAILS (excess `speak`) — the direction conflict is a compile error;
- a DUPLEX pattern declares BOTH (`⟨io, audit, listen, speak⟩`) — the signature itself is the duplex type; composition through calls keeps the №324 semantics (a caller must cover what its callees do).

### 3.3 The duplex channel (runtime)

A process-global registry (`src/duplex.rs`, the session.rs template; std-only, not feature-gated — the CI contour exercises the same code path as the serve contour):

- `duplex_open(session, priority?)` binds a channel to ONE live №348 session (unknown/ended session → typed `SESSION_UNKNOWN`, fail-closed, no implicit recreate); the priority word (default `normal`) is validated against the closed ladder; the handle is an opaque `Value::Duplex` map (`id`/`session` — the printable projection; the registry is the state; **appended LAST** — bincode variant indices of the existing variants stay stable, the №250/№264/№350 rule).
- Each direction holds AT MOST ONE active stream (`Stream { id, rank, outcome }`). A start on a direction that already has an active stream refuses (`DUPLEX_BUSY` — one flow per direction).
- **Barge-in**: a start on the OPPOSITE direction carries the incoming stream's priority rank; if `incoming ≥ active` (equal wins — barge-in is the point of duplex), the opposite stream is PREEMPTED: its outcome becomes the typed `InterruptedBy { by, priority }` (never a panic, never a silent drop) and the incoming stream starts. If `incoming < active`, the start REFUSES (`DUPLEX_PREEMPT_DENIED` — fail-closed; the caller may retry with a higher priority).
- Streams end typed: `Active` (running), `Completed` (`stop`), `InterruptedBy` (preempted). There is no way to observe a half-torn stream: every outcome is a constructor of the same type.
- `speak_stop`/`listen_stop` end the active stream of their direction (idle direction → typed `DUPLEX_IDLE`); `duplex_state` returns the projection of both streams plus their outcomes (the introspection surface for tests and operators).

### 3.4 Audit completeness (the acceptance invariant)

Every transition is a ledger record over the `duplex.*` family (the №393/№415 surfaces; best-effort per ADR-0167 §2 driver 5 — a ledger failure is loud on stderr and never flips the outcome): `duplex.open`, `duplex.speak_start`, `duplex.listen_start`, `duplex.preempt`, `duplex.stop`. Preemption emits TWO records — the preempt decision AND the preempted stream's terminal outcome (`outcome=interrupted`) — so "прерывание НЕ теряет события аудита" holds BY CONSTRUCTION; the differential ledger-count test pins it against an independent transition model.

### 3.5 The voice gates stand

ADR-0145 gates (consent/egress on the real STT/TTS surfaces) are untouched; the duplex layer adds NO egress of its own — the ledger carries a text digest only (the №415 args_hash posture), never the spoken text. Real-weights omni stays PARKED (№294): the duplex semantics are registry-contour semantics, and the report says so loudly.

## 4. Consequences

- The office duplex loop becomes expressible AND checkable: the signature tells the direction story, direction conflicts are compile errors, and barge-in is a typed priority decision — not a race discipline.
- The effect vocabulary grows for the first time since №324 — the two new words are domain-scoped (audio) and ride the existing gate with zero grammar change and zero declared-trail breakage.
- Untouched by decision: ADR-0145 gates, the №316 classification SSOT (role × label × reversibility — direction is a semantic-side table, not a new classification axis), the wake contour (№348), REFERENCE narrative (№425), the №398 thresholds.
- Honest boundaries: the streams are registry state on the mock contour (no real audio device, no real DSP); `omni_ask` keeps its own both-directions contract (a pre-existing surface — its declared-trail users must now declare both words); the duplex channel is single-process (the session it binds is single-process too).
