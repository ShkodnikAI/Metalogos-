# ADR-0198: The Error Model — the Loud Failure + `_or` + `try`; `Result<T>`/`Option<T>` in the Type Layer Without New `Value` Variants (the C6 R-b decision, ACCEPTED 2026-10-23 by the owner's verbatim; SUPERSEDES ADR-0106)

- Status: **Accepted** (2026-10-23 — the owner's verbatim in #1475,
  the decision line quoted in §2; the ADR-0193 §7 form — the decision
  as submitted is recorded, the machine adds no riders)
- Date: 2026-10-23 (the decision); 2026-10-24 (the ADR record, №792)
- Deciders: the OWNER (the C6 decision); the machine formats the record
- Naryad: №792 (issue #1515, Волна 63)
- Source: the audit-23.10 C6 (issue #1475) — the deliberation lives in
  the private strategy canon
- **Supersedes:** [ADR-0106](0106-option-result-rejected.md)
  (Rejected `Option`/`Result`, 2026-08-21) — the 0106 file carries the
  superseded note (the same-PR edit); the AGENTS.md §3/§6 lines that
  cited ADR-0106 are updated (the same-PR edit)
- Scope: a MODEL record — no grammar change, no new `Value` variants,
  no signature change in this naryad; the STATIC check "checked before
  use" rides #1505 (the registry queue)

## §1. Context — why ADR-0106's premises expired

ADR-0106 (Rejected `Option`/`Result`, 2026-08-21) argued from the
soft-failure model: the language "fails softly" (the silent Unit, the
silent 0.0), so the sum-type error channel had nothing to carry. The
audit-23.10 re-read found the examples no longer true: the language
became LOUD where 0106 assumed soft — `to_float` on an unsupported
input is a loud coded error (`src/builtins/math.rs:79`), `read_file`
missing the path is a loud `[IO_ERROR]` (`src/builtins/io.rs:605`),
and the deliberate quietness lives in the EXPLICIT `_or` twins (№481/
№514 — the opt-in softness, the loudness default inverted). The `try`
construction already returns the structured result
`Struct { ok, value, error }` (ADR-0142, №374 — the 0106 text itself
writes `error: Option<Struct>`), and the signatures registry carries
the `|Unit` shapes. The model 0106 rejected does not exist anymore;
the rejection's vocabulary stayed while its referent changed.

The boundary evidence, one line each (the audit's re-read, verified
against `main` at the decision date):

- **The loudness is the DEFAULT, not the exception.** The builtins
  refuse with the position-0 coded stamps ([IO_ERROR], [DB_ERROR],
  [PRECISION_LOSS] — the №787 addition of this very wave), and the
  `try{}` construction catches them through the
  `ORIGIN_STAMPED_CODES` whitelist — the machine-classified, typed
  channel the soft-failure world never had.
- **The quietness is EXPLICIT and grep-able.** The `_or` twins
  (`to_float_or`, the №481/№514 family) are the opt-in softness — the
  name itself declares the fallback. Nothing else may fail softly:
  the №584/№581/№600 lineage, the honest-boundary dictionary (№604)
  and the wave-62 A-epic all GUARD the loudness as a security
  property.
- **The structured result already exists.** `try` returns
  `Struct { ok: Bool, value: Any, error: (code, message) }` — the
  sum-type CHANNEL is shipped; what 0106 rejected was the TYPE
  VOCABULARY around it, and the vocabulary is what R-b now accepts —
  as the spellings of the existing shapes.

## §2. The decision (the owner's verbatim, #1475, 2026-10-23)

> **Решение владельца (23.10): рекомендации приняты** — *R-b: ADR-0106
> пересматривается; полные сумм-типы не вводятся.*

The FULL verbatim lives in #1475 (the owner comment
2026-10-23T06:46Z — permanent, public); the ADR quotes the R-b line
(the docs-language budget — the lint is the honest machine). The full
verbatim also fixes: **Int I-c lands NOW** (the loud precision-loss
refusal — №787 of this wave, `12003e5`); **Int I-b (the full `Int(i64)`
type) is DEFERRED** to the gateway-pilot results; the static check
"the try result and T|Unit checked before use" rides #1505.

Interpretation (the record-keeping reading, the ADR-0195 §7 form): the
R-b decision fixes FOUR things — (1) the error MODEL is "the loud
failure with a code (ADR-0131/0194) + the explicit quietness through
the `_or` twins + `try` for the handling"; (2) in the TYPE layer:
`Result<T>` is the type of the `try` result; `Option<T>` is the
spelling of `T|Unit` — the ALIASES of the existing shapes, not new
machinery; (3) **no new `Value` variants — the grammar of values and
the 526 signatures do not change** (the decision names it verbatim);
(4) the full sum types (`Ok`/`Err`/`Some`/`None`, `?`) are NOT
introduced — the R-b line says so, and the C6 verdict confirms.

## §3. What lands where (the machine furniture)

| Piece | Where | State |
|---|---|---|
| The loud `[PRECISION_LOSS]` boundary (the I-c Int piece) | №787 (gh#1510) | LANDED (`12003e5`, this wave) |
| The error-model record (this ADR) | №792 | LANDED (this naryad) |
| The static check: the try result and `T\|Unit` verified before use | #1505 | queued (the registry, the P2 tail) |
| The `Result<T>`/`Option<T>` spellings in the docs/registry vocabulary | the weekly docs + the natural doc cycles | no code change — the ALIAS form |
| The full sum types (`Ok/Err/Some/None`, `?`) | NOT INTRODUCED | the decision boundary |

## §4. What this changes in the ADR lineage

1. **ADR-0106 → Superseded** (by this ADR) — the 0106 file carries the
   note with the link; its "not without demonstrated need" PRINCIPLE
   survives (the need arrived and is documented here — the principle
   was never the rejection, the soft-failure PREMISE was).
2. **AGENTS.md §3/§6** — the two lines citing "Result/Option —
   rejected, ADR-0106" are re-pointed to this ADR (the same-PR edit;
   the CLAUDE.md bridge copy rides byte-identical — the №763 gate).
3. **No CI change** — the error model is already loud (the gates
   guard it); the static use-check (#1505) carries its own gate when
   it lands.

## §5. The links

- The decision: #1475 (the C6 owner comment, 2026-10-23T06:46Z).
- The landing naryads: #1503 (I-c — №787, LANDED), #1504 (this ADR),
  #1505 (the static check — queued).
- The lineage: ADR-0106 (superseded), ADR-0131 (the diagnostic codes),
  ADR-0142 (the try result struct), ADR-0194 (the loud-fail contract),
  the №481/№514 `_or` twins (the explicit quietness), №787 (the
  precision boundary — the model in action).
