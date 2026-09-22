# ADR-0172: Session model — wake/interrupt/duty over a process-global registry

**Status:** Accepted
**Date:** 2026-09-22
**Naryad:** #348 (issue #591; dispatch #598, wave 9 — registry Phase 4 "Always-on, память, забывание")
**Pillar:** session (cross-cutting: interpreter builtins + ledger + profiles)
**Consumers:** №349 (duty-profile compile rule — keys on the `profile duty` flags this ADR lands), №352 (full-duplex barge-in — consumes the typed interruption priorities), №393/№415 (Action Ledger surfaces — the `session.*` record family)

## 1. Context

The session surface was a pillar stub (§16.0-6(б)): `spec!("session_login", 2, "stub"; …)` returned an **empty** `Value::Session` map and `session_logout` was a **no-op** — both flagged mock in the registry. The AST carried the hooks (`OnSessionStart`/`OnSessionEnd`, `src/ast.rs:850-854`; the interpreter fires them once at `run()` entry/exit, `src/interpreter/mod.rs:178`), and `Value::Session` existed as a server-side type — but none of it was a session *model*: no lifecycle, no wake, no interruption semantics, no duty mode, no trail.

Phase 4 of the registry plan ("Always-on") needs that model: the office agent lives in a wake/sleep rhythm (keyword/event/schedule), gets interrupted (higher-priority events preempt running work), and spends most of its life in a *duty* (background) mode where a different security posture applies (№349: private materialization and network sinks become compile errors there). The schedule half already has an SSOT — the native cron (№418: `cron_add` arity 2..5, tz/catch_up/payload; try-code `CRON_JOB_FAILED`) — so scheduling must NOT be re-invented here.

Opaque-handle precedent: ADR-0114 (`Value::Grant`, `Value::Secret`, `Value::Likeness*`); ledger side-effect precedent: ADR-0167 §3.4 (the record call site IS the action's own bookkeeping path); profile precedent: ADR-0161 (closed `profile` vocabulary with loud validation).

## 2. Decision Drivers

1. **Real state, not a bigger mock.** The stub returns a map; the model must have live state the operations actually mutate (membership, duty flag, pending wake/interrupt queues) — otherwise №349/№352 would enforce against fiction.
2. **Every transition audited.** A session is a security-relevant actor context: create/wake/interrupt/duty/end transitions land in the Action Ledger (`session.*` family) exactly like grant lifecycle events. Best-effort, per ADR-0167 §2 driver 5: a ledger failure is loud on stderr and never flips the session operation's outcome.
3. **SSOT discipline for scheduling.** Wake-by-schedule must arrive through the existing cron payload-dispatch (№418 D4) calling `session_wake(source: "schedule")`. No new cron surface, no new scheduler; `cron_add` arity 2..5 is not touched.
4. **Typed interruptions, nothing lost.** Preemption is the №352 lever. The contract: a typed priority ladder (`low < normal < high < critical`), `take` returns the highest rank (FIFO within a rank), and BOTH the enqueue and the dequeue are ledger-recorded — preemption therefore loses no audit trail by construction.
5. **The duty profile needs a static carrier.** A compile-time rule (№349) cannot key on runtime-only state. The carrier is the existing closed `profile` mechanism (ADR-0161): `profile duty { materialization: denied; surfaces: local_only }` — declared in the program, validated loudly (unknown words are errors, not silent no-ops), resolved into `ResolvedProfiles` flags. The runtime `session_duty_enter/exit` is the session-side half of the same concept; this ADR lands the carrier, №349 lands the enforcement.
6. **Honest auth boundary.** There is no server user-store in the interpreter: `session_login` does NOT verify credentials. The password argument is accepted (String or Secret) but never checked. This is a documented boundary of the model, not a stub — the state, queues, and ledger trail are real. Server-side auth (the `SessionEntry` middleware, `src/server.rs:459`) remains its own surface.
7. **Value surface stays `Value::Session(HashMap<String,String>)`.** The handle is the printable projection (`id`/`user`/`duty`); the registry is the state. No new Value variant (bytecode/serde churn avoided — the №417 lesson on representation cost).

## 3. Decision

### 3.1 Registry and lifecycle

A process-global registry (`src/session.rs`, `Mutex<HashMap<String, SessionState>>`; std-only, no new dependencies):

| Transition | Surface | Registry effect | Ledger record |
|---|---|---|---|
| create | `session_login(user, password)` | insert `SessionState` | `session.create` |
| duty-enter | `session_duty_enter(s)` | `duty = true` | `session.duty_enter` |
| duty-exit | `session_duty_exit(s)` | `duty = false` | `session.duty_exit` |
| wake | `session_wake(s, source, payload?)` | enqueue `WakeEvent` | `session.wake` |
| wake delivered | `session_poll_wake(s)` | dequeue oldest (FIFO) | `session.wake_delivered` |
| interrupt | `session_interrupt(s, priority, reason?)` | enqueue `InterruptEvent` | `session.interrupt` |
| interrupt taken | `session_take_interrupt(s)` | dequeue highest rank | `session.interrupt_taken` |
| end | `session_logout(s)` | remove entry | `session.end` |

Session ids are `s-` + 16 hex chars of a SHA-256 over `(user, unix-millis, process seq)` — uniqueness is the requirement; the id carries no secret. Unknown/ended sessions refuse with the typed `SESSION_UNKNOWN` error (fail-closed; no implicit recreate).

### 3.2 Wake sources (closed set)

`keyword | event | schedule` — validated at the surface; any other word is a loud error. `schedule` wakes are produced by the №418 cron payload-dispatch calling `session_wake(source: "schedule")`; the language surface itself treats all three sources identically (enqueue + record), so the CI contour (no live cron) exercises the same code path as the serve contour.

### 3.3 Interruption priorities (typed ladder)

`low(0) < normal(1) < high(2) < critical(3)`. `session_take_interrupt` returns the highest-rank pending interrupt; ties resolve FIFO (earliest enqueued within the rank). Both events of a preemption — the arriving interrupt AND the taken one — are ledger records, so "preemption without audit loss" (the №352 duplex invariant) holds by construction. Empty queue → `Unit` (typed, no panic).

### 3.4 Duty profile (static carrier + runtime half)

Static: `profile duty { materialization: denied; surfaces: local_only }` — at least one knob required (a duty declaration with neither would be a silent no-op, and no-ops must be loud). Each knob resolves independently into `ResolvedProfiles::duty_materialization_denied` / `duty_surfaces_local_only`. The knobs' MEANING is owned by №349 (materialization of private handles → compile error; network sinks → compile error); this ADR fixes only the carrier and its closed vocabulary.

Runtime: `session_duty_enter/exit` flip the session-side `duty` flag and record it. The two halves are intentionally independent surfaces (a program may declare the static profile; the running session may enter duty mode dynamically) — №349 wires the compile rule to the static flags.

## 4. Consequences

- **Positive:** the Phase-4 lane has a real session substrate; №349/№352 have their carrier and lever; the ledger trail gives the dogfood (№395-class) consumers a complete session `session.*` record family for free.
- **Costs:** 6 new registry rows (461→467, append-only — .mbc `CallBuiltin` indices stable); one new `src/session.rs` module (~340 lines) and one new `src/builtins/session.rs` (the handlers moved out of `crypto.rs`); classification rows curated (Sink/Source × Internal).
- **Risks:** registry leak on abandoned sessions — acceptable for the interpreter lifetime (server sessions have their own TTL store, `SessionEntry.expires`); the FIFO wake queue is deliberately NOT prioritized (interrupts carry the priority semantics; conflating them would blur the №352 contract).

## 5. Alternatives considered

- **Server-only sessions (reuse `SessionEntry`):** rejected — the language surface must work in the interpreter/CLI contour too (tests, dogfood harnesses), not only behind `mlog serve`.
- **New `Value::SessionId(String)` variant:** rejected — a new variant ripples through serde/bytecode/printing for zero expressive gain; the existing `Value::Session` map projection already satisfies the ADR-0114 opacity rule.
- **Wake-by-polling file/db:** rejected — invents a second scheduling SSOT alongside №418 cron (violates driver 3).

## 6. Honest boundaries

1. **Credentials are NOT verified** — no user-store exists in the interpreter; `session_login` mints a session for any well-typed arguments. The boundary is loud here, in the builtin doc comment, and in the classification rationale.
2. **The duty profile does not enforce anything in this naryad** — the static compile rule is №349. Until it lands, `profile duty` validates and resolves; it does not yet reject anything.
3. **Wake/interrupt delivery is cooperative** — the model queues and records events; it does not preempt a running interpreter frame (that is the №352 duplex surface's job, typed at its signature).
4. **Ledger writes are best-effort** (ADR-0167 §2 driver 5): a journal failure is loud on stderr and never flips the session operation.
