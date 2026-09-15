# ADR-0154: Label lattice for taint kinds — (conf, integrity, consent-scope)

**Status:** Accepted
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
