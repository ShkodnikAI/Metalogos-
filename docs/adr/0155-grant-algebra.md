# ADR-0155: Grant algebra — permissions for irreversible operations

**Status:** Accepted (fills the reserved booking of 2026-09-14; approved direction per owner dispatch #491, wave 3)
**Date:** 2026-09-17
**Naryad:** #389 (issue #483; fills the booking made by naryad #319 — plan v2 §19 originally mapped this slot to naryad #339, renumbered by wave 3)
**Pillar:** cross-cutting (security/action); consumes the label lattice (ADR-0154) and the effect trail (ADR-0154 §9)
**Consumers:** naryads #390 (Grant value + builtins), #391 (lattice bridge), #392 (DenyEvent), #393 (signed ledger), #395 (dogfood run)

## 1. Context

Irreversible actions in Metalogos are today only **prohibited**, never **conditionally allowed**. The `SINK_CLEARANCE` Category-A gate (naryad #325, ADR-0161) denies the schema-destroying SQL literals passed to `db_execute` (DROP TABLE/DATABASE/INDEX, TRUNCATE — the content-gated population; DELETE/ALTER stay in parameterized-CRUD/SQL_DYNAMIC territory) with the specialized class `IRREVERSIBLE_NO_GRANT` (`src/audit.rs`, the leak-suite vocabulary block; the comment there already names the intended direction: "grant algebra is Phase 3"). That single mechanism cannot express the agentic requirement this ADR answers: **"the agent may DELETE — only in the `sessions` table, only with a grant issued by admin, at most five times."** Prohibition is the zero-power point of a capability system; an autonomous office (the wave-3 dogfood target, naryad #395) needs the whole axis from "never" to "always, audited".

Reserved slots and precedents verified on `da262c3e98`:

- `docs/adr/0155-grant-algebra.md` existed as an honest `reserved` booking (naryad #319) — this document fills it.
- The opaque-handle family is the closest language precedent: `Secret` / `Encrypted` / `Hash` are non-displayable, zeroizing, non-serializable values (ADR-0031, ADR-0038), and ADR-0114 fixed the philosophy that stateful security-relevant data lives behind opaque handles, not as plain data. What the language does **not** have yet is true linearity: no variant enforces use-once or metered use. The Grant layer adds it.
- A persistent, TTL-carrying ledger template exists: `consent_ledger` (`src/consent.rs:47–137`) — CREATE TABLE with `kind` / `subject` / `scope` / `ttl_seconds` / `issued_at` / `expires_at`, INSERT + query helpers. Grant state follows the same pattern.
- The sink inventory is an SSOT: the №316 classification (`Role::Sink`) is the only sink list any gate reads. The grant gate must read the same map, never a second hand-written list.
- The effect trail `⟨io, audit⟩` (naryad #324, ADR-0154 §9) already makes "every use is audited" a declarative property of patterns and tool methods.

## 2. Decision Drivers

1. **Agentic delegation needs scoped *allowing*.** A capability must be issuable, attenuable, revocable, and expirable — not a boolean per sink.
2. **Fail-closed default is already law.** `IRREVERSIBLE_NO_GRANT` denies ungranted destructive SQL today; the algebra may only add an allowing path on top, never weaken the default (compat-profile escape hatch ADR-0161 applies to egress *clearance* only, and must never apply here).
3. **Linearity must be enforced by the checker, not by convention.** Precedent: `Secret` opacity is a runtime + audit contract (ADR-0031). A Once-grant needs the stronger, static half: move semantics and use-after-move detection in the semantic pass.
4. **Grant state must be persistent and reconcilable** (quotas survive process restarts; revocation propagates) — the consent-ledger pattern is the template; the *signed* chain is a later wave step (naryad #393), not a redesign.
5. **Audit is the universal tax.** Every granted use of an irreversible action produces an audit event; the effect trail `⟨io, audit⟩` is the static declaration surface for it.
6. **No new trust root.** The position (see §6) is a profile over language-level linear values, not a new token format and not an external policy service.

## 3. Decision

### 3.1 The Grant value

`Grant` is a new opaque `Value` variant (the `Secret` treatment, ADR-0031): it cannot be displayed, printed, logged, converted to `String`, embedded in a response, or hashed into user-visible output. A grant is a capability record:

- `grant_id` — unique, ledger-primary identity;
- `capability` — a (resource, action) pair, structured (e.g. resource = SQL table `sessions`, action = `db_execute.delete`; resource = repo URL, action = `git_push`);
- `class` — one of **Once**, **N(n)**, **Unlimited** (§3.2);
- `scope` — structured attenuation payload (macaroon-style caveats, §6): the narrowing that all descendants inherit;
- `issuer`, `issued_at`, `expires_at` — mandatory TTL (rule 3);
- `remaining` — for N(n) only; authoritative state lives in the **grant ledger**, never in the value (the value is a handle, not the quota).

### 3.2 The three classes

| Class | Linearity | Exhaustion model | Typical sinks |
|---|---|---|---|
| **Once** | statically linear: exactly one use, move semantics; second use on any path = semantic error `GRANT_REUSED` | consumed by use | `git_push` (publication is one-way), one-off destructive migrations (`db_execute` DDL/DROP), single-shot `exec` of a release step |
| **N(n)** | value may be stored and re-read (not linear); the *uses* are metered | runtime quota: the ledger decrements per authorized use; exhaustion = typed error `GRANT_EXHAUSTED` | rate-bound `send_message` (quota), build-loop `exec`/`exec_argv` (n builds), endpoint-scoped destructive SQL (k deletes in `sessions`) |
| **Unlimited** | copyable (`Clone` permitted — the only copyable class) | no quota | long-lived services whose every use is audited (effect trail `⟨io, audit⟩` is mandatory for patterns/tool methods holding Unlimited grants); e.g. a payment `http_post` profiled as an action |

Sink mapping (justification on the №316 SSOT inventory):

- **`db_execute` destructive literals**: the ungranted deny covers the schema-destroying forms (DROP/TRUNCATE); the GRANTED path deliberately treats DELETE and ALTER as destructive too — a granted delete must be scoped and metered even though the ungranted gate is narrower today (the asymmetry is fail-closed in the safe direction). Once for one-off migrations, N(n) for bounded application-side deletes. Unlimited requires an explicit owner-level justification in the program and is loud in the audit report.
- **`exec` / `exec_argv`**: Once (release step) or N(n) (build loop). The independent integrity gate (UNTRUSTED_EXEC_DECISION, naryad #327) is **not** replaced by a grant — both gates must pass (§5).
- **`git_push`**: Once per push — publication is the canonical irreversible act; N(n) only for bounded batch publication.
- **`http_post`**: not inherently irreversible; a grant applies only when the program *profiles* the endpoint as an action (e.g. payments). Then N(n) or Unlimited (audited).
- **`send_message`**: quasi-irreversible (a sent message cannot be unsent) — N(n) is the natural quota shape; Unlimited for bot services with the audit trail.

### 3.3 Linearity rules (1–6)

1. **No copy except Unlimited.** A Once-grant moves (assignment transfers ownership; passing as an argument consumes it). Use after the move — on any path the flow analysis covers — is the compile/audit error `GRANT_REUSED`. Copying N(n)/Unlimited values never copies *power*: N(n) is metered in the ledger, Unlimited is audited per use.
2. **No serialization.** Grant → `String` / `print` / `log` / `respond` / template interpolation is a runtime error (the ADR-0031 opacity contract) and raises the audit event `GRANT_DISPLAY_ATTEMPT`. Rationale: a serializable grant is a bearer token — exactly the leak class the language exists to close.
3. **No outliving the revoking context.** TTL is mandatory (`expires_at`); a use past expiry is `GRANT_EXPIRED`. A grant value may not be stored where it survives the revoking authority's scope (entity fields holding grants are re-validated at use time against the ledger, not against the stale handle).
4. **Subgrant is explicit and attenuating-only.** `subgrant(parent, capability', scope', ttl')` is a builtin operation with: `capability' ⊆ capability`, `scope' ⊆ scope`, `ttl' ≤ ttl` (attenuation only, the macaroons/Biscuit law — §6). Class power may only go down: Unlimited → N(m) → Once. Any widening attempt is `GRANT_ESCALATION` (also the fuzzing oracle for naryad #390).
5. **Revoke is cascading.** `revoke(grant_id)` revokes the grant and, transitively, every subgrant derived from it — the consent-cascade precedent (naryad #335). Revoke events are append-only ledger records; a revoked chain never disappears from the ledger (it becomes a deny reason).
6. **Fail-closed default unchanged.** The absence of a grant at a grant-gated sink is exactly today's `IRREVERSIBLE_NO_GRANT` deny. No compatibility profile weakens this gate (ADR-0161 `legacy` touches egress clearance only — explicit non-goal here). The algebra adds the allowing path; it never removes the denying one.

### 3.4 Ledger state model (pre-signing)

Grant lifecycle events — `issued`, `subgranted`, `used` (class, capability, subject, note), `exhausted`, `expired`, `revoked` — are appended to a **grant ledger** in the `consent_ledger` pattern (`src/consent.rs:47–137`): in-memory SQLite, same column vocabulary plus `parent_id` for the subgrant tree and `class` / `remaining`. The ledger is the SSOT for quota state and revocation; handles are untrusted caches of it. Cryptographic signing (prev-hash + Ed25519), the in-toto/PROV profile, rotation and snapshots are naryad #393's contract and change the *integrity* of this ledger, not its schema-driven semantics.

### 3.5 Typed error surface (for #390–#392)

`GRANT_MISSING` (the audit-class name stays `IRREVERSIBLE_NO_GRANT` for continuity of the leak suite), `GRANT_REUSED`, `GRANT_EXHAUSTED`, `GRANT_EXPIRED`, `GRANT_REVOKED`, `GRANT_ESCALATION`, `GRANT_DISPLAY_ATTEMPT`. Each maps to one DenyEvent reason in naryad #392's vocabulary and carries the failing `grant_id` when one exists.

## 4. Rejected alternatives

- **Bearer tokens / capability strings** (env vars, files, headers): replayable, copyable, serializable — the exact opposite of rules 1–2; they convert a capability system back into a secret-possession system. Rejected.
- **Pure compile-time linear capabilities only** (no ledger): cannot express N(n) quotas, expiry, or cascading revocation — the state has to live somewhere, and the ledger is that somewhere. Rejected as the whole mechanism; retained as the static half (Once).
- **External policy engine** (OPA/Cedar as a runtime service): adds a network dependency to the security core, evaluates against global state the language cannot audit locally, and has no linear accounting. Rejected for the core; Cedar's deny-by-default shape influenced the gate order (§5).
- **ACL matrix per sink** (grant = boolean per builtin): global, non-delegable, no attenuation, no expiry — cannot express "admin's DELETE in `sessions`, five times". Rejected.

## 5. Interface with the label lattice (bridge — naryad #391)

The two systems are **orthogonal axes, composed conjunctively**:

1. **Data gate first, action gate second.** At a grant-gated sink the data gate (label `⊑` maxTaint, naryad #325 machinery) is evaluated before the grant gate. Deny reasons are reported deterministically in this order (DenyEvent carries the first failed gate; #392 owns the vocabulary).
2. **A grant never widens a label.** Holding `exec` with `Unlimited` does not lift `UNTRUSTED_EXEC_DECISION` or `SECRET_TO_EXEC`; integrity `≥ trusted` on the incoming data (naryad #327) remains independently mandatory. Capabilities authorize *actions on resources*; labels classify *data*. No conversion exists between the axes in either direction.
3. **Consent is also orthogonal.** The consent-scope axis (ADR-0154 §2.3) governs personal-data egress; a grant governs irreversible action. The kitchen-camera wave-acceptance flow exercises both on one path — deny with an explained reason and a ledger record, allow with consent and grant and full tracing.
4. **Effect trail is the audit surface.** A pattern/tool method that performs granted irreversible actions declares `⟨io, audit⟩` (naryad #324 machinery); for Unlimited-class holdings the audit component is mandatory, and a declaration of bare `⟨io⟩` under such a holding is a check-path error.

## 6. Prior art — what we take, what we leave

| System | Take | Leave |
|---|---|---|
| **Macaroons** | attenuation-only delegation: caveats can only narrow; the algebra of §3.3 rule 4 is exactly this law | the bearer-token wire form (macaroons are serialized caveats — rule 2 forbids serialization here) |
| **Biscuit** | offline-verifiable attenuation blocks; the principle that attenuation math must be checkable without a network round-trip (our ledger reads are local SQLite, in the consent-ledger pattern) | the token format and the embedded Datalog engine — Metalogos expresses attenuation in its own type/flow system, not in a logic language |
| **UCAN** | capability = (resource, action) delegation chains; the delegation-depth discipline mirrored in rule 4's monotone power decrease | JWT/UCAN wire encoding, did:key PKI, and the "capability = proof of possession" reading (our capabilities are ledger-backed values, not proofs) |
| **Cedar** | deny-by-default evaluation shape; policies that compose conjunctively with data attributes — the §5 gate order follows it | policies as an external engine with global state; non-linear resources; policy language as a second programming surface |

**Position.** This is a **profile over language-level linear values**: `Grant` is a `Value` variant checked by the existing semantic/audit machinery (the ADR-0031/ADR-0114 opaque-handle family plus new static linearity), with state in a local append-only ledger. It is deliberately *not* a new token format, not a wire protocol, and not an external authorization service.

## 7. Migration note

- The existing deny stays: `IRREVERSIBLE_NO_GRANT` (destructive SQL in `db_execute`) continues to fire for every ungranted call — the leak suite and the Stage-4 corpus behavior are unchanged by this ADR (no code in this naryad).
- The algebra is purely additive: naryad #390 introduces `Value::Grant`, the `issue_grant` / `subgrant` / `revoke_grant` builtins and the linear checks; until that merge, no `.mlog` program can even name a grant value.
- No compatibility profile exists or will exist for this gate (rule 6). The migration burden sits on the allowing side only: programs that *want* the new allowance declare grants explicitly.
- The audit vocabulary gains the §3.5 identifiers without renaming the existing `IRREVERSIBLE_NO_GRANT` class (historical continuity of reports and tests).

## 8. Verification

- This naryad is documentation-only: no `.rs`/`.mlog` behavior changes, CI green, `docs/adr/README.md` index updated (0155 → accepted), no stubs (`grep todo!|unimplemented!|SKELETON` over the diff — 0).
- Self-sufficiency check (DoD): naryads #390–#392 can start from this document alone — the class table (§3.2), rules 1–6 (§3.3), ledger schema vocabulary (§3.4), typed error list (§3.5), gate order (§5) and migration constraints (§7) are all fixed here.
- The doc-language gate (`tests/docs_language_lint.rs`, naryad #383) passes on this file: English-only documentation.

## 9. Verification (naryad #390)

The implementation landed in naryad #390 (issue #484) and follows this ADR without deviations from §3:

- **Surface (§3.1-§3.2)**: `Value::Grant` (opaque, non-printable — the `is_nonprintable` family; serde emits a dead `[GRANT]` marker; a deserialized grant refuses every use) + five appended builtins (`grant_issue`, `grant_subgrant`, `grant_revoke`, `grant_use`, `db_execute_with_grant`; registry 442→447, bytecode indices unshifted). The safe default class at issue is Once.
- **Linearity (§3.3)**: rules 1-2 enforced statically (`GRANT_REUSED` compile error: flow walk with branch-intersection merge, move detection `let g2 = g`; exclusive if/else single uses legal) and at runtime (ledger state machine: consumed/revoked/expired/exhausted refuse with typed errors, never panics). Rules 3-6: mandatory TTL (a born-expired grant refuses), attenuation-only subgrant (scope/TTL/power, quota conservation — a Once parent is consumed by the split, an N(n) parent is debited by the child quota), cascading revoke (BFS over the `parent_id` tree), and the ungranted deny untouched (`IRREVERSIBLE_NO_GRANT` — verified by contract test against the №325 leak-suite vocabulary).
- **Ledger (§3.4)**: `src/grants.rs` — in-process SQLite (`grant_state` + append-only `grant_events`), the `consent_ledger` pattern; signing stays with #393.
- **Fuzzing (DoD б)**: `tests/grant_algebra_fuzz.rs` — 4000 deterministic ops (issue/subgrant/use/revoke) differentially checked against an independently coded model of the algebra (scope/class/quota/lifecycle/expiry-horizon); zero divergence, zero amplification.
- **Backend parity**: the granted action runs identically on TW and VM (shared ledger, typed binding — the №381 `convert_params` contract); asserted by contract tests and by the golden example.
- **Examples**: `examples/w2_grant_linear.mlog` (+ `.expected`, both backends) and `examples/w2_grant_linear_reuse.mlog` (+ `.error` naming `GRANT_REUSED`).
- **Documentation**: REFERENCE §4.15.1 + §6 index + §7 classification rows (№316 SSOT); README claims resynced.
