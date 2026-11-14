# ADR-0154: Label lattice for taint kinds — (conf, integrity, consent-scope)

**Status:** Proposed (amended 2026-11-12 by №949 — §12, the interprocedural contract; the owner's explicit «принято» returns Accepted — канон §13.14 п.6)
**Date:** 2026-09-15
**Naryad:** №322 (issue #416, plan v2 §16.3 / Волна 1)
**Supersedes:** the reserved stub written by №319 (booking note in git history)

## 1. Context

METALogos already has a real, executed taint system: `enum TaintKind { LlmOutput, Secret, UserInput, Sanitized, CanaryLeak }` (`src/audit.rs`), the per-scope `TaintTracker`, and Category-A checks built on it (SECRET_LEAK, HTML_INJECTION, SQL_DYNAMIC, CANARY_LEAK — path-sensitive, №284). Consent, however, lives in a separate subsystem (`consent_ledger`, `src/voice/store.rs`), and the builtins classification of №316 (role × label × reversibility) is a data table without a lattice behind it.

План v2 (§13.3, §16.3) requires a three-component security label — **(conf, integrity, consent-scope)** — as the foundation for Волна 1: statement-level inference (№323), effect-trail (№324), sink-gate + `profile legacy` (№325), redact/declassify (№326), integrity/anti-injection (№327), runtime parity (№328), dogfooding (№329). This ADR fixes the label model, the carrier, and the migration bridge from the legacy kinds.

Prior art (mandatory references per the naryad):

- **DLM / Jif** (Myers et al.): decentralized labels — confidentiality and integrity policies as first-class values combined componentwise; the observation that data flow joins confidentiality while it *meets* integrity.
- **FlowCaml** (Simonet): static information flow in an ML dialect — a lattice of levels with pointwise ordering across a structured label.
- **LIO** (Stefan et al.): label arithmetic in a dynamic IFC kernel — `join` on data combination, `meet` on requirement combination, and privilege/consent as explicit values.

## 2. Model

A label is the product of three components:

### 2.1 Confidentiality axis (`Conf`)

Order: `public < consented < private < poisoned`.

- `public` — world-visible.
- `consented` — releasable under a consent scope; the scope itself lives in the consent component (§2.3). A `consented` value with an empty scope set is not leak-free by this axis alone — the sink side (№325) checks scope coverage; the lattice only carries the data.
- `private` — user data, secrets.
- `poisoned` — **quarantine**, and deliberately NOT an ordinary top. It is absorbing for BOTH join and meet (D1):

```
join(x, poisoned)    = poisoned    for every x
meet(x, poisoned)    = poisoned    for every x
```

Rationale: a poisoned value is a value whose provenance is disqualifying (e.g. a confirmed-compromised channel, `CanaryLeak`). If `meet(poisoned, private) == private` (the pure-order answer), then a single `meet` would be a one-step declassifier — quarantine must not be curable by lattice arithmetic. Poisoned has no legal sinks (today: advisory CANARY_LEAK; structurally from №325: gate).

### 2.2 Integrity axis (`Integrity`)

Order: `untrusted < trusted`. Dual to confidentiality (DLM/LIO):

```
join  (data combination)      = min  — mixing data can only lower integrity
meet  (requirement combination) = max — the strongest requirement wins
```

This is the axis that gives №327 (anti-injection) its footing: user input and LLM output are integrity problems, not secrecy problems.

### 2.3 Consent-scope axis (`ConsentScope`)

The set of consent scopes a value is covered by; empty set = no consent backing. Ties to `consent_ledger` (`src/voice/store.rs`) — the ledger remains the runtime SSOT of *which consents exist* (that wiring is Phase 2, №335); the lattice carries which scopes a *value* is covered by.

```
join (data combination)       = intersection — a value usable in two contexts
                                                keeps only consent both contexts carry
meet (requirement combination) = union       — requirements combine permissively
```

### 2.4 Componentwise combination

`Label.join` / `Label.meet` are componentwise (D2):

```
join(a, b)  = (conf: a.conf.join(b.conf),      integrity: min,   consent: a ∩ b)
meet(a, b)  = (conf: a.conf.meet(b.conf),      integrity: max,   consent: a ∪ b)
```

## 3. Carrier and annotation syntax

- `src/labels.rs` — the lattice: `Conf`, `Integrity`, `ConsentScope`, `Label` with `join`/`meet`/`bottom`, canonical `Display` (`private, untrusted, consent(gdpr)`), and `Label::parse`.
- Carrier in the AST (`src/ast.rs`): `LabelAnn { span, raw }` — the raw annotation text with its source span — attached to `Param.label`, `FieldDecl.label`, `EntityRecordDecl.label`, `EntitySimpleDecl.label`.
- Grammar (`src/grammar.pest`): `type_name label_ann?` where `label_ann = '<' label_spec '>'`, `label_spec = label_part (',' label_part)*`, `label_part = consent(...)` or a bare word. The rule is **strictly additive**: a `<` after a type name was previously a parse error, so no existing program changes meaning.
- **Division of labor**: the grammar guarantees the *shape* (comma list of words / consent parts); semantic analysis (`src/semantic.rs`) validates the *words* via `Label::parse` and reports unknown words / duplicates / missing conf / empty consent lists with the annotation's span. This keeps the word table in one place (`labels.rs`) and makes the semantic validation reachable and testable — no dead code.
- Annotation rules (validated in `Label::parse`): exactly one conf word (`public|consented|private|poisoned`) is REQUIRED — a bare `<untrusted>` must not silently mean `public`, the silent default would be the wrong (loudest) one; at most one integrity word (`trusted|untrusted`, default `trusted`); at most one `consent(scope, ...)` part (default empty). Defaults are only on the permissive side; every downgrade remains an explicit annotation act.
- Parameteric label polymorphism (`∀t. String<t>`) is explicitly deferred — annotations are label *literals* in this phase.

## 4. Rejected alternatives

- **Single numeric lattice** (one integer encoding all three axes): conflates orthogonal concerns; `join` on conf would have to know about consent scopes; quarantine cannot be expressed as an absorbing element without polluting the whole order. Rejected — three components, joined pointwise.
- **Independent dimensions without meet** (only join, gates compare components ad hoc): without `meet` there is no way to combine *requirements*, and the sink-gate (№325) needs exactly that. Rejected — meet is componentwise and total (with quarantine absorbing, D1).
- **Meet curable quarantine** (`meet(poisoned, x) = x`): rejected — see §2.1; it would make `meet` a one-step declassifier.
- **Silent defaults on the restrictive side** (e.g. missing integrity = `untrusted`): would force every annotation to carry all components and would make the lattice pessimistic about literals. Rejected — defaults are permissive (`trusted`, no consent); downgrades are explicit.

## 5. Migration bridge from TaintKind

The five legacy kinds project onto the lattice — **additively**: no kind is removed, no Category-A check changes its behavior, and no existing message is touched (the only acceptable diff direction was "adding"; here it is zero). The table lives in `labels::legacy_taint_label`, keyed by variant name; `src/audit.rs` carries an exhaustiveness unit test pinning enum ↔ table sync.

| TaintKind    | conf     | integrity | consent |
|--------------|----------|-----------|---------|
| `LlmOutput`  | public   | untrusted | —       |
| `Secret`     | private  | trusted   | —       |
| `UserInput`  | public   | untrusted | —       |
| `Sanitized`  | public   | trusted   | —       |
| `CanaryLeak` | poisoned | untrusted | —       |

Rationale: `LlmOutput` is meant for display (no conf concern) but is the HTML_INJECTION vector — integrity, not secrecy. `Secret` is the confidentiality concern; the leak is the sink's problem (SECRET_LEAK). `UserInput` is not secret but untrusted (SQL_DYNAMIC's vector). `Sanitized` passed through `render()`/`escape_html()` restores trust. `CanaryLeak` is a confirmed-compromised channel — the quarantine element (§2.1). The sink-gate (№325) reads this table — it is the contract point between the legacy checks and the lattice.

## 6. Consequences

- Волна 1 can proceed on a fixed model: №323 infers labels through all 10 statement kinds, №324 attaches effect-trails to pattern signatures, №325 gates sinks by the classification of №316 read through this lattice (+ `profile legacy`), №326 adds the only legitimate downward path (redact/declassify), №327 adds the integrity axis to untrusted sources, №328 carries labels to VM `Value`s (LabelJoin/SinkCheck), №329 dogfoods the office contour under the gate.
- `mlog check` (semantic) now reports label-annotation errors with spans — new diagnostics only.
- The annotation syntax is user-visible; REFERENCE/grammar documentation follows the rule count in README (311).
- Parametric label polymorphism and consent-source wiring (ledger ↔ lattice) are Phase 2 (№335) — recorded as loud, intentional boundaries.

## 7. Verification

- `cargo test naryad_322` — annotation syntax (parse + carrier + span), componentwise join/meet incl. poisoned absorbing, semantic loudness (unknown word / missing conf / duplicates with span), legacy projection totality.
- `cargo test` — existing taint/audit suites without degradation.
- grep `todo!`/`unimplemented!`/`SKELETON` in `src/labels.rs` — 0 (also asserted by a test).

## Appendix A — Statement-kind inference contracts (naryad №323)

The statement-level inference (`semantic::infer_pattern_labels`) walks pattern
bodies with an environment `var → Label`. Per-kind contracts — input reading,
output label, side effects, merge rule:

| # | Statement        | Input (reads)                    | Output label                    | Side effects                  | Merge rule |
|---|------------------|----------------------------------|---------------------------------|-------------------------------|------------|
| 1 | LetBinding       | RHS label                        | binds `name` = label(RHS)       | —                             | sequential |
| 2 | Assign           | RHS label                        | binds `name` = label(RHS)       | —                             | sequential (replace — reassignment to a safe value lowers in straight-line code, mirroring TaintTracker untaint; merge points re-add conservatism) |
| 3 | Each             | label(iterable)                  | iterator = label(iterable)      | —                             | body cannot raise the iterator (restored); every other var = join(entry, post-body) |
| 4 | EachWithIndex    | label(iterable)                  | item = label(iterable); index = bottom (a position, not data) | —             | same as Each (both iterator vars restored) |
| 5 | While            | condition ignored (conditions do not taint values) | —             | body assignments              | bounded fixpoint (see decision below); exit env = fixpoint env |
| 6 | IfElseBlock      | condition ignored                | —                               | branch assignments            | componentwise join over ALL branches (then + else-ifs + else); a branch that did not assign contributes the entry label — one-sided assignment is conservative by construction |
| 7 | IfThen           | condition ignored                | —                               | then-assignments              | merge with implicit empty else: join(entry, then) |
| 8 | Return           | label(value)                     | joins the pattern output        | terminates the branch (conservatively: later statements in the block still infer) | sequential |
| 9 | ExprStmt         | label(expr)                      | joins the pattern output        | —                             | sequential |
| 10 | Match           | label(scrutinee)                 | —                               | arm assignments               | join over arms (+ else); the scrutinee's label additionally joins every variable ASSIGNED in any arm — control dependence on the scrutinee (decisions derived from private data taint the outcomes); assignment shape is structural, not label-diff |

Extra kinds beyond the ten mandatory ones (honest completeness):

| Statement            | Contract                                                                                     |
|----------------------|----------------------------------------------------------------------------------------------|
| Break / Continue     | loop control — no label effect, no merge contribution                                        |
| Memorize / Forget    | memory side effects; the payload label does not enter the value flow — persistence gating is №325 (TAINT_PERSISTENCE vocabulary of the leak-suite) |
| Relate               | knowledge-graph edge; same treatment as Memorize/Forget                                      |

**While: fixpoint vs conservative — DECISION.** The naryad required an explicit
choice. Chosen: **bounded fixpoint** — the body is re-inferred until the
environment stabilizes, capped at 8 passes (`LABEL_FIXPOINT_MAX_PASSES`; the
conf lattice has height 4, monotone joins stabilize any loop-carried var→var
chain within that; the cap is a termination guard, deterministic). Rationale:
a single conservative pass under-approximates loop-carried chains
(`a = b; b = env(...)` needs a second pass) — under-approximation on a
security lattice is unsound; the bounded fixpoint IS the exact fixpoint for
this finite lattice, at the price of at most 8 body walks.

**Entry points.** Annotated params → parsed label (№322 carrier); unannotated
params, literals, and unresolved names → `bottom` (the inference is open-world:
the sink-gate №325 reads these labels, it does not trust them).

**Sources/sanitizers.** The audit.rs vocabulary (`env`/`secret`,
`call_llm`/`call_claude`/`call_llm_schema`/`reflex_generate`,
`form_data`/`json_body`/`query_param`/`mcp_call`, `render`/`escape_html`,
`redact`) is projected through the §5 table. redact(mode "secrets"/"all")
maps private → bottom; every other label passes through — quarantine is NOT
curable by redact (ADR-0136 D2).

**Example.** `examples/l1_flow_infer.mlog`: the private label from `env()`
reaches the output through if/else + each without a single annotation.

**Boundaries (loud).** Recursive patterns → №324; label polymorphism →
deferred; media-handle flows → Phase 2; learnable-pattern calls as
LlmOutput-equivalent sources need a program-wide context (audit.rs already
does this at call sites — the per-pattern inference adds it when №325
provides the declarations context).

## 9. Appendix (№324): effect trail in pattern signatures — ⟨io, audit⟩

Implemented in `src/ast.rs` (`Effect`, `EffectSet`, `EffectAnn`, `effects` fields on `PatternDecl` / `ToolMethod` / `LearnablePatternDecl`), `src/grammar.pest` (`effect_trail`/`effect_word`, +2 rules), and `src/semantic.rs` (`check_effect_trails` — validation, interprocedural fixpoint, gate).

### 9.1 Syntax and semantics

`pattern P(x: String) -> String ⟨io, audit⟩ { ... }` — the declared trail sits after the return type. Grammar is shape-only (comma list of bare words in `⟨…⟩`; `⟨⟩` = the zero-effect declaration); semantic validates the WORDS — the closed set `{io, audit}` — with the trail's span (same division of labor as `LabelAnn`, §3). Additive: a `⟨` after a return type was previously a parse error.

- `io` — data crosses the expression boundary. Read from the №316 SSOT: `Source` builtins (ingress) and `Sink` builtins (egress) — no name re-hardcoding.
- `audit` — a persistent, auditable write: a `Sink` builtin whose `Reversibility` is not `Pure` (state/db/file/memory writes, delivery), plus the `memorize`/`forget`/`relate` statements.
- `Pure`/`Lift` call sites carry no effects. A learnable pattern IS an LLM call — `{io}` by construction.

### 9.2 The boundary gate

For every container with a DECLARED trail: factual body effects ⊑ declared, computed with interface semantics — a call to a pattern whose trail is declared contributes that DECLARED contract (not its internals); a call to an unannotated pattern contributes its inferred (fixpoint) effects. Excess (factual \ declared) is a loud compile error listing declared/required/excess — caught at the calling boundary. A pattern WITHOUT a declared trail is ungated: zero behavioral delta for existing programs.

### 9.3 Recursion decision

The effect domain is the 4-element powerset of `{io, audit}`; joins are unions; the propagation is monotone. Therefore the interprocedural fixpoint converges in ≤ 4 passes even for directly and mutually recursive patterns WITHOUT annotations — the dispatcher's No-Go signal ("вывод не сходится на рекурсии → аннотация") does not fire. An explicit trail on a recursive pattern is welcome but not required; it gets gated against the same fixpoint result.

### 9.4 Verification (№324)

- `cargo test naryad_324` — 18 tests: trail syntax on pattern/tool-method/learnable (span, bare type name), empty trail = zero effects, unknown/duplicate words loud, the gate (excess listed; violation at the calling boundary; undeclared callee shares factual effects), legal composition clean, zero delta without trails, №316 mapping (env → io; print → io+audit; upper → ∅), memory statements → audit, direct and mutual recursion convergence, gated recursion, the showcase `examples/l1_effect_sig.mlog`, canonical `format_effect_set`, no-stub grep.

## 10. Appendix (№326): redact/declassify — the only sanctioned downward move, policy as a value

Implemented in `src/builtins/string.rs` (the policy registry `REDACT_POLICIES` + runtime shapes), `src/semantic.rs` (the static label mapping reads the same registry) and `src/audit.rs` (`REDACT_APPLIED` events).

### 10.1 Policy as a value

`redact(value, "<policy>")` — the second argument is a policy VALUE: it names the transformation AND declares the target conf of the result. The built-in registry:

| policy       | target conf | transformation (runtime) |
|--------------|-------------|--------------------------|
| `secrets`    | public      | secret-pattern masking (legacy ADR-0136) |
| `all`        | public      | full masking: secrets + PII + entropy net (legacy) |
| `pii`        | private     | PII-pattern masking — conservative, keeps the source conf (legacy) |
| `pii_strip`  | private     | PII pattern strip — conservative (new) |
| `hash_only`  | private     | one-way SHA-256 fingerprint, full 256-bit hex — destroys the data, keeps comparability, does NOT declassify (amended by №806/B1: was `public` with a 64-bit digest) |
| `hash_keyed` | public      | one-way HMAC-SHA256 under the process-scoped `METALOGOS_REDACT_KEY` — the sanctioned public fingerprint; loud refusal without the key (№806/B1) |
| `truncate`   | private     | keep the first 3 chars, mask the rest (new) |

Conservatism rule: pattern strips and truncations can miss data, so they do NOT declassify (target `private`) — the sink gate keeps blocking their output. The one-way policies (full masking) destroy the data and are the sanctioned path down. Unknown or dynamic (non-literal) policy values pass the input label through — no silent downward moves.

**№806 amendment (the audit-23.10 B1, Wave 66).** The original `hash_only` contract had a hole: a 64-bit UNKEYED SHA-256 prefix of a LOW-ENTROPY secret (a PIN, a short token) is brute-forced offline in seconds, so the `public` label on the digest was a leak channel dressed as hygiene. The amendment: `hash_only` produces the FULL 256-bit hex digest, keeps its comparability AND its integrity restore (the №327 decision lift survives on non-secret data — the error-code branching form), but does NOT declassify anymore. Public comparability of a fingerprint moves to the NEW `hash_keyed` policy: HMAC-SHA256 under the process-scoped `METALOGOS_REDACT_KEY` (an attacker without the key cannot evaluate the HMAC, so low-entropy secrets stay safe to expose as digests); the missing/empty key is a LOUD runtime refusal — never a silent unkeyed fallback. The static label mapping (semantic.rs) reads the same registry: `hash_only` keeps the input conf and restores integrity; `hash_keyed` lands at bottom. The secret-comparison recipe (limitations.md, the UNTRUSTED_DECISION refusal text): `ct_eq(provided, expected)` — constant-time — or the keyed fingerprint on BOTH sides. The static label can no longer diverge from the runtime reality: there is no env-dependent policy word (a one-word env-keyed design was rejected — the check-time and run-time environments could diverge, and the label would lie; №525: an untruthful label is worse than a refusal).

### 10.2 Unconditional audit events

Every `redact()` application — statically enumerated across all statement-bearing containers — records an event: what was processed (container), which policy, which target conf. The event is UNCONDITIONAL: a `Severity::Info` finding in `mlog audit` AND a `[REDACT][audit-event]` stderr line on compile/run. No profile, no env toggle can switch them off — they are the paper trail of the downward move. Under `profile legacy` the events remain (only the №325 clearance verdicts change severity).

### 10.3 Verification (№326)

- `cargo test naryad_326` — the registry (names/targets/lookup, loud unknown words), the downward path, conservatism (pii_strip/truncate keep the label), unconditional events (count + policy/target in message; present under legacy; dynamic → `<dynamic>`), runtime shapes (hash determinism and data destruction, truncation shape), the showcase `examples/l1_redact.mlog`. №806 amendment tests (the same file): the full 64-hex digest shape, the keyed determinism + `[HMAC:` shape, the loud missing/empty-key refusal, `hash_only` no longer passing the №325 gate on a secret, `hash_keyed` passing it.

## 11. Appendix (№767): the read-back pair exemption — the documented boundary + the risk-review record

Added by naryad №767 (gh#1420, Wave 59 — the external audit 21.10 §3 M-1, §6 п.4). The boundary is documented in [`docs/limitations.md`](../limitations.md) (the Static Analysis section); this appendix is the ADR-side record and the risk-review trail. The implementation is NOT modified by this appendix — it is a documentation pass.

### 11.1 The fact

The sink-clearance gate (№325) reads this lattice: a sink requires `public` conf (§2.1 — `poisoned` clears nothing). №729 (gh#1356) added the ONE deliberate narrow exemption — the read-back pair table (`semantic::READBACK_PAIRS`, `src/semantic.rs`): a destructive lookup key originating from the paired Source of the same domain (`cron_list` → `cron_remove`; `mtree_retrieve` → `mtree_forget`) clears the gate at the KEY POSITION only. The rationale: the paired sinks are match-only — the key never persists new data into the state, the data returns to the domain it came from; there is no egress channel by construction. The conservative boundary rides the lattice: the provenance chain must be paired-source-only (field reads and `to_string` preserve the domain, ANY foreign admixture loses the exemption — `readback_domain` answers `None`, fail-closed); the payload positions of multi-arg sinks stay gated; a `poisoned` label clears nothing anywhere (§2.1 — the exemption never overrides quarantine).

### 11.2 The risk-review record (the audit 21.10 §6 п.4 — пункт 10 of the audit methodology)

- **Verdict: ACCEPTED with the standing boundary** (2026-10-22, naryad №767, Wave 59).
- **What was reviewed:** the exemption is position-only (the lookup-key argument alone, `key_arg: 0` in both rows), domain-matched (both ends share one domain — `cron` / `mtree`), provenance-conservative (paired-source-only chains; field reads / `to_string` preserve the domain, every other expression form fails closed), and quarantine-proof (§2.1 — `poisoned` clears nothing anywhere). The audit's own assessment stands: «реализация консервативная — претензий по существу нет».
- **Limitations of the record:** the review covers the table AS IT STANDS (two pairs); the implementation was not re-verified line-by-line by this review — the pinning tests of №729 remain the executable verification.
- **Conditions for revision:** a THIRD pair, or a shape change in the pair recognition (a new carrying expression form in `readback_domain`), or any relaxation of the conservative boundary — each re-opens this review in its own naryad. Every future Category-A exemption follows the same protocol: the limitations row + the risk-review verdict in the same PR (the M-1 lesson: «исправление видимости обнаружило реальные срабатывания, и ответом стало исключение» — the correct answer, but the trace must survive).

## 12. Appendix (№949): the interprocedural contract — pattern summaries, the call-graph fixpoint, the end of the open world

Added by naryad №949 (gh#1916, the total audit 12.11, P0 — ADR BEFORE code; the owner decision 12.11: revising accepted records is permitted). **This appendix is PROPOSED — the owner's explicit «принято» turns it Accepted (канон §13.14 п.6); the implementation naryads are №950 (the static lattice) and №951 (the in-pattern losses).** №950 LANDED on 2026-11-13: the code implements §12.1–§12.6 with the three departures recorded in §12.8, and the status stays Proposed — the stamp is the owner's act, not the implementer's.

### 12.1 The fact (the audit 12.11, machine-verified on the published v0.39.0)

The lattice as accepted is INTRAPROCEDURAL. `expr_label` (`src/semantic.rs`) computes a user-pattern call's label as the JOIN OF ITS ARGUMENTS — the called body is not consulted; an unannotated parameter enters the body as `public` (the «open world», §3.2/Appendix A). The runtime labels (ADR-0156/№328) are a projection of the same static knowledge and are blind at the same boundary. The machine-verified consequence: every probe below PASSES `mlog check` on v0.39.0 while the value leaks at run (the full probe set: `FOSVED-office-v2 → docs/projects/metalogos/audit_2026-11-12/pocs/`, commit `fd384d3`).

### 12.2 The pattern summary (the amendment to §3.2 and Appendix A)

Every user-defined pattern P carries a summary computed from its body, on the FULL lattice — both axes (conf, integrity) and the consent scope:

- **(а) The return label** of P = join( labels of the SOURCES evaluated inside the body ) ⊔ join( labels of the PARAMETERS that reach a return ), where «reaches» is computed through the body's own calls to other patterns (their summaries, recursively).
- **(б) The required clearance per parameter** p = MEET of the clearances of every sink p reaches (sinks inside P's body and inside the patterns P calls, via their summaries). The call site checks the argument's label against (б) — this is the end of the open world: an unannotated parameter is no longer silently `public` at the call boundary.
- **(в) The side sinks** of P reachable from the body's internal sources — each such (source-label, sink) pair is checked at summary time exactly as the intraprocedural gate checks it today (the body keeps refusing its own leaks — the summary never LAUNDERS an internal violation).

The existing annotation rules (§3) are unchanged and EXTENDED: an annotation is a CHECKABLE CLAIM, not trusted knowledge — a declared `String<private>` parameter or a declared return label is verified against the summary, and a mismatch is a loud check error (the annotation that claims less than the body does is the lie the summary catches). A NEW annotation form arrives with №950: the return-label annotation on the pattern signature.

### 12.3 The computation

- A **fixpoint over the call graph** of the program: the lattice is finite (§2), the summaries monotonically over-approximate, so the iteration terminates; the recursion and the cycles take the CONSERVATIVE TOP for the affected component (the cycle's members summarize as «may return anything reachable» — no false negatives by construction).
- **Cross-module calls** use the same summaries: a module's exported patterns summarize once per module hash (the №376 cache form — an unchanged module is never recomputed).
- The analysis budget: measured on the examples corpus + the office `app.mlog`; the accepted threshold is the №376 dispatch bound (+50% analysis time). The measurement lands in the №950 PR.

### 12.4 The diagnostics — no new words

The refusals reuse the EXISTING classes (SECRET_LEAK, SECRET_EGRESS_NETWORK, PII_EGRESS_*, HTML_INJECTION, UNTRUSTED_EXEC_DECISION, SQL_DYNAMIC — the vocabulary is frozen by №385). What changes is the REACH: the message names the call chain from the source to the sink (`Key("a") → http_post(url, …) — SECRET_EGRESS_NETWORK`). No new check_id is introduced by №950/№951.

### 12.5 The transition window (the precedent of the owner decision 27.10 by attestation)

One release window: the NEW interprocedural refusals fire as LOUD WARNINGS while the entry program declares `profile legacy { interproc: warn }`; the default (no key) is the ERROR. After the window the key is REMOVED (a program still carrying it is a check error naming this appendix). The BREAKING row and the `docs/upgrade.md` section land with №950 (the code that introduces the refusals); this appendix fixes the mechanism in advance.

### 12.6 The machine-verified probe → rule table

Every row's check status was re-verified on this tree (the built binary, `mlog check`, `SP=TOPSECRET123`); the file stems are unique across the PoC folders — 1_secret_lane (p*, q*), 2_html_exec_sql_lanes (e*, h*, i*, s*), 3_loop_label_loss (l*): the PASS rows are the gaps this appendix closes; the REFUSE rows are the honest positive side — the existing lanes that already hold.

| Probe | check on v0.39.0 | The rule that rejects it (after №950/№951) |
|---|---|---|
| p01_direct, q01_post_direct, q11, p02_struct, p03_list, p05_transform, p06_json, p09_kv, p10_file, p15_b64, p16_try, p17_mem, p19_format, p20_len_oracle, p21_map, p22_session, p23_template, p24_secret_fn, p25_while, p26_sha, p28_nested_struct, p29_push, p30_respond_err | REFUSE | already caught today — the intraprocedural SECRET_LEAK / persistence / opacity lanes (unchanged) |
| p12_implicit | REFUSE | already caught today (the comparison/oracle lane refuses the shape on v0.39.0 — the audit's «устояло» list) |
| p04_helper_return, q03_post_helper_return, q05_get_url_helper, q06_post_url_helper, q07_writefile_helper, q08_secret_fn_helper, i04 (2_html_exec_sql_lanes) | PASS | (а) — the return label joins the internal sources → SECRET_LEAK / SECRET_EGRESS_NETWORK |
| p18_param, q02_post_helper_param, h03_helper_param | PASS | (б) — the parameter reaches a print/respond/http sink → the call-site clearance check → SECRET_LEAK / HTML_INJECTION |
| q09_two_level, h06_user_input_helper | PASS | (а)+(б) through the CALL-GRAPH FIXPOINT (two levels: the source crosses two pattern boundaries) |
| p07_loop, l03, l04, l08, h04_loop | PASS | the EACH-accumulator contract (№951): the loop body's raises fold back into the loop-exit environment, exactly as the `while` arm already did → PII_EGRESS_OUTPUT / HTML_INJECTION |
| p13_http_get_url, q04_get_url | PASS | the URL line (№951): a source interpolated into the URL/first argument of http_get/http_post → SECRET_EGRESS_NETWORK |
| p11_flow_output | PASS | the FLOW contract (№951): a flow step's return carries the label to the flow output → SECRET_LEAK |
| q10_entity_secret | PASS | the ENTITY contract (№951): a `Secret`-declared entity read back via `to_string` → SECRET_LEAK (the opacity is value-level, not print-site-only) |
| p14_error_msg | PASS | №955 (the runtime error text masks the value; check stays pass) |
| e02_helper, e03_llm_exec_helper | PASS | (б)+(а) — the exec lane joins the summary rules → UNTRUSTED_EXEC_DECISION |
| h02_helper_return | PASS | (а) — the return label joins the internal call_llm → HTML_INJECTION |
| i03 (Gen: let a = call_llm; return a) | PASS | (а) — the same internal-source rule through the let-binding |
| e01_direct, e04_llm_exec_direct, h01_direct, h05_user_input, i01, i02 | REFUSE | already caught today — the intraprocedural UNTRUSTED_EXEC_DECISION / HTML_INJECTION lanes |
| s01_direct, s02_helper_sql, s03_helper_exec | REFUSE | already caught today — the SQL literal gate (shape-based, holds through patterns) |
| l01, l02, l05, l06, l07, l10 | REFUSE | already caught today — the intraprocedural lanes (direct-source join, branch join, while fixpoint) |
| p27_env_or | PASS (static only) | **A machine-verified finding beyond the audit's leak list, with a correction (№951 re-validation):** `env_or` passes `mlog check` as a print argument because the lattice's source vocabulary is hand-written and does not name it. But the RUNTIME backstop DOES refuse it (`[SINK_CLEARANCE_RUNTIME]`, verified by running the probe), so the gap is STATIC-ONLY and not an end-to-end leak — the original note overstated it. The static half closes with the registry-derived source classes of №951 → PII_EGRESS_OUTPUT |

### 12.6a Re-validation of the probe table (№951, 2026-11-13) — two probes were never evidence

The table above was supplied by the dispatcher from the audit of 12.11. Re-running every probe while implementing №951 found **two entries that are not valid programs**, and the honest place for that is here, next to the claim they supported:

- **`p08_match`** does not PARSE. It was written as `match x { "x" => s, _ => "n" }` — arrow arms — while Metalogos spells a match with `then { }` blocks. The harness read «no diagnostic class in the output» as «check passes», so a parse error was counted as a clean pass. It is removed from the evidence base; the match-expression label question it was meant to raise is untested and stays open.
- **`l09`** does not RUN: `each c, i in …` binds the INDEX first and the VALUE second (`src/parser/stmt.rs`), so the probe concatenated a Float index into a String and dies with `[TYPE_MISMATCH]`. Rewritten with the correct order (`each i, c in …`) it IS a leak, and the №951 `each` fix catches it — so the row's RULE stands while the probe that argued for it did not.

The method defect behind both is the dispatcher's, not the language's: a probe harness that classifies by the presence of a diagnostic class treats «invalid program» and «clean program» alike. The corrected harness checks for a parse error first and, for every probe claimed as a leak, requires the leak to be OBSERVED (the secret in the sink's output), not merely the check to be silent. A second, smaller defect showed up in the correction itself: the first re-validation pass looked only at STDOUT and so under-counted the probes whose sink is a file — the honest form checks the sink the probe actually targets.

What the re-validation did NOT shake: of the 29 probes that passed `mlog check` on the published v0.39.0, **14 are confirmed leaking by observation in a sandbox with no network** — 13 with the secret in stdout, plus `q07_writefile_helper`, whose target is a FILE: the re-validation's first pass looked only at stdout and marked it unconfirmed, and the file it had written (`q07.txt`, containing the secret) was then found in the working tree. The remaining seven leak into an HTTP response body or a command — targets the original run captured with its own listener, which this sandbox blocks, so they are unconfirmed HERE rather than refuted. The four pins B1–B4 are refused by name, and the boundary finding itself is unaffected.

### 12.6b The literal-marker rule meets every new gate position (№951)

Twice in one naryad an extension of the sink gate to a new argument position collided with the literal-marker rule of ADR-0161 §3, and the pattern is worth naming once rather than being rediscovered a third time.

The rule reads a BARE STRING LITERAL's wording: «паспорт», «confidential», «internal», «private», «secret» in a literal mean the literal carries personal data or names private infrastructure. It was written for literals that are DATA (a hard-coded passport number) or a SINK'S ADDRESS (the destination is the leak vector). Both collisions came from applying it where a literal is neither:

1. **A prompt.** Gating the arguments of the whole `Role::Source` + `Label::Network` class on confidentiality made the repository's own leak fixture `n449_prompt_egress_chain` refuse on the literal `"summarize the private document"`. Every prompt mentioning a confidentiality word would have followed, in a language whose primary use is asking a model about text. The content family was therefore left out of the request table.
2. **A URL path.** Even with the table narrowed to the address/query family, the project's hardening probe `http_get(".../private/data", {robots: true})` refused statically on the path segment `/private/`, hiding the runtime's typed `[HTTP_ROBOTS_DENIED]`. At a request position a bare literal is now bottom: a URL the author typed is author text, not data that flowed from a source — pin B6 is about a CONCATENATION, where the secret and not the wording supplies the label.

The general statement: **the literal-marker rule is sound for the positions it was written for and misfires wherever a literal is authored text rather than data.** Any future gate on a new argument position has to decide, explicitly, whether a bare literal in that position is data; the default answer for an authored address or prompt is no. The rule itself is unchanged by №951 — only the positions it is consulted at.

### 12.7 What this appendix does NOT promise

- No pointer analysis, no path-sensitivity beyond the existing may-joins, no new label vocabulary (§12.4) — the summaries are a monotone over-approximation on the SAME lattice.
- The consent axis participates in (а)/(б) unchanged (the consent scope joins/ meets per §2.4) — no new consent machinery.
- The runtime second line of defense is ADR-0156 §5 (№952) — the static lattice stays the SSOT of every verdict.

### 12.8 The implementation record (№950, 2026-11-13) — where the code differs from §12.3

The appendix is the CONTRACT; this subsection records what the implementation actually does, including the two places it departs from the §12.3 wording. Recorded rather than silently absorbed: §12.3 is what the owner is asked to accept, and an implementation that quietly differs from it would make the acceptance meaningless.

1. **Reverse topological order for the acyclic part; an ascending fixpoint inside a cycle.** §12.3 prescribes one fixpoint over the whole call graph. The first implementation did exactly that, with an 8-pass cap, and it was wrong in both directions: a call chain DEEPER than the cap came out as an UNDER-approximation (a leak at depth 12 was invisible — the same class of silent gap this appendix exists to close), and the repeated passes cost +272% analysis time on a 200-pattern chain, far outside the §12.3 budget. The code instead summarizes the acyclic part callees-before-callers, which is complete in ONE pass at any depth (pinned at depths 1…20), and solves each cyclic component by an ascending fixpoint from bottom, joining every generation onto the previous one. The conservative TOP of §12.3 is kept only as the fallback for a component that does not stabilize within the cap, and that was forced by a regression: giving TOP to every cycle member made the SELF-HOSTED PARSER (`self-host/parser.mlog`, ~20 mutually recursive patterns) refuse with 23 findings, because every parse function returned «private, untrusted» and every `if`/`while` on a parse result became an UNTRUSTED_DECISION. Recursion is ordinary Metalogos; a rule that makes it unusable is a false positive, not caution. Both halves are pinned: a leak through a recursive pattern refuses, a benign mutual recursion stays clean.

2. **The axes are probed SEPARATELY, and that is a correctness requirement, not a refinement.** A sanitizer inside the callee usually repairs ONE axis: `redact(x, "hash_only")` restores the integrity and keeps the confidentiality; `escape_html` restores the integrity for the HTML lane. Probed with both axes raised at once, such a parameter still «reaches» its sink — the confidentiality half fails — and the call site then refuses an UNTRUSTED (but public) argument the callee had already made safe. That is a false refusal, and it makes the migration this naryad documents («sanitize inside the callee») not work; it was found exactly that way, while migrating the repository's own benchmark fixture. So each parameter carries a LIST of facts, each tagged with the axis that produced it, and the call site re-decides a fact against the argument's label SEEN THROUGH THAT AXIS (the other axis neutralized to its bottom). Pinned three ways: the sanitized flow clears, the unsanitized one refuses, and a SECRET argument into the same sanitized parameter still refuses — `hash_only` repairs the integrity, not the confidentiality.

3. **The summaries reuse the lattice machinery on a probe — no second analyzer.** The reach facts of (а) and (б) are answered by RE-RUNNING `infer_pattern_labels` and `sink_clearance_violations` on the pattern with one parameter annotated with a probe label and the others forced to bottom, differencing against an all-bottom baseline. A cheap screen (every parameter probed at once with `<private, untrusted>`, which fails every clearance class) bounds the per-parameter runs by monotonicity, so a pattern that carries nothing costs two runs in total rather than two per parameter. This is why there is no duplicated source/sink vocabulary anywhere in №950 — the only vocabulary is the lattice's own.

4. **The diagnostic classes that actually fire** differ from two forecasts in the §12.6 table and in the №948 pins: the table predicted `SECRET_LEAK` for the B1/B2 shapes, while the class that fires is the SINK-CLEARANCE class of the lattice gate (`PII_EGRESS_OUTPUT` for `print` of a private value, `SECRET_EGRESS_NETWORK` / `PII_EGRESS_NETWORK` on the network sinks). `SECRET_LEAK` is the separate SYNTACTIC lane (a direct `env()`→sink match) and stays intraprocedural. §12.4 holds as written — the vocabulary is frozen and no new `check_id` was minted; only the forecast of WHICH frozen word fires was off, and the `.expected` pins now record the fired class.

5. **The measured cost** (§12.3's +50% bound): +8.5% analysis time on the six largest `examples/` programs, +3.4% on the 2300-line benchmark workload, +37.2% on the self-hosted parser (the worst case — the cycle fixpoint), and +11.9 / +30.6 / +36.2% on synthetic 50 / 100 / 200-pattern call chains. Inside the bound.

6. **Cross-module summaries are NOT yet cached per module hash** (the §12.3 cache form). The current implementation seeds per analysis entry point and recomputes; the measured cost makes the cache unnecessary at present corpus sizes. Named here as an open item rather than claimed as done — it returns if a real program exceeds the bound. Cross-module REACH itself is covered: `check_program_with_root` resolves the import graph statically (№953) and seeds over the merged declaration list, so a leak through an imported pattern refuses (verified on a two-file probe).

   The snapshot is a thread-local seeded by the analysis entry points, exactly like `html_label::seed_render_raw`, and it carries the same property: an entry point that does NOT seed reads no summaries and therefore behaves exactly as v0.39.0 did — weaker analysis, never a weaker refusal. Every production entry (`semantic::check_program`, `audit_category_a`, `audit_program`, and so the run/compile/serve gates above them) seeds; the only non-seeding callers are tests that call `infer_pattern_labels` directly.

7. **The summarizable unit is wider than the appendix says.** §12.2 speaks of patterns; the implementation also summarizes TOOL METHODS, under the `Tool.method` key a `QualifiedCall` resolves to. They were not in the audit's row list and they carried the identical open world in both halves (verified: `tool T { send(v) { print(v) } }` + `T.send(env("SP"))` passed `mlog check` on v0.39.0). Closing them in the same naryad was not scope creep but the condition for the fix to mean anything — with patterns closed and tool methods open, one `tool { }` wrapper bypasses §12 entirely. Learnable patterns needed nothing: their output already carries the untrusted label.

8. **Three rule families cross the boundary, not one.** §12.2 (б) speaks of «the clearances of the sinks the parameter reaches». The walk also refuses on two rules that are NOT the generic clearance and whose names are not sinks at all: `profile duty { materialization: denied }` fires on `str`/`json_encode` (`Role::Pure`), and the Н455 untrusted-file-path rule fires on `read_file` (`Role::Source`). Both are label-dependent, so both cross the boundary — each re-decided at the call site by ITS OWN rule, under its own class (`DUTY_MATERIALIZATION`, `UNTRUSTED_FILE_PATH`). Propagating them through the clearance rule, as the first implementation did, named the wrong rule — a `str` call reported as a network egress. What does NOT cross: the label-INDEPENDENT refusals (`duty-network-sink`, `irreversible-content`, `video-likeness-no-consent` — identical in the probe's baseline, so the baseline subtraction drops them and the inner site reports them), and `media_save`, whose gate reads caller-context credentials (a consent scope or a bound likeness token, №387) that the probe cannot hold — re-deciding it at the call site would refuse a caller that legitimately holds the credential, so its own site keeps that gate.

9. **What №950 does NOT close**, unchanged from §12.7 and the threat-model table: the intraprocedural losses B5–B8 (№951) and dynamic dispatch beyond the static lattice B12 (№952).
