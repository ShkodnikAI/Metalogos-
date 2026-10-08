# ADR-0188: The Template Semantics Composition — the §7 lane plan: the compile-time port re-priced, the phased recommendation (the OWNER's choice slot)

- Status: **Accepted — option (б)** (the owner's verdict, 2026-10-08;
  the verbatim authorization core: «ADR-0188 принимаю по варианту (б)»;
  the FULL verdict text: gh#1123, the owner's comment, and the canonical
  «гейты-0810» record — the docs-language lint №383 keeps this .md
  English-only, the ADR-0181 §1 authorization-record shape. The
  implementation: the (б) rules — the compile-time refusal of the
  implicit String→Html coercion (the assignment/argument positions) and
  of the unknown template names — landed by №662 (gh#1163, Волна 39,
  the SHARED semantic checker: TW/VM identically, the parity postulate);
  the pipe stays runtime-absent (№115 open); the FULL port (а) — later)
- Date: 2026-10-08
- Deciders: the executor (the draft); the OWNER fixes the plan (the
  verdict — 2026-10-08, gh#1123)
- Naryad: №640 (gh#1123, Wave 36); the (б) implementation: №662 (gh#1163, Волна 39)
- Predecessors: [ADR-0187 — the memory-phase plan (the draft-first form)](0187-memory-phase-plan.md),
  ADR-0073 (the JIT/parity discipline — the parity-postulate shape),
  ADR-0122 #208 / №250 (the compile-time template registration),
  №617 (the static OpaqueConcat + variable-scope walk), №114 (the opaque
  Html coerce), №115 (the pipe syntax, the unknown-template refusal),
  №635 (the vm_golden corpus sweep), the Known-Issue ledger gh#967 §7

## §1. Context

The Known-Issue ledger gh#967 §7 (template semantics) carries three
honest `#[ignore]` findings in `tests/template_integration.rs` — the
machine reads 3 (`grep -c "#\[ignore"` → 3, verified on main bb60d11 at
the draft date):

1. **The pipe syntax** `{{ var | safe }}` is not implemented (№115) —
   the template expression grammar has no pipe stage; composition that
   needs an explicit escape override refuses at parse time
   (`template_safe_pipe_not_implemented`).
2. **The opaque Html coercion is a runtime fact, not a checker fact**
   (№114) — `entity page: Html = "<div>" + "hello" + "</div>"` passes
   the semantic check (the operands are Strings; the String→Html
   assignment coerces silently) and only the runtime guard refuses the
   opaque concatenation (`check_html_from_string_error` expects the
   CHECKER to refuse — it stays ignored until the №114 coerce work).
3. **The unknown-template detection is a runtime error**
   (`builtin_render` refuses at request time), not a semantic-checker
   refusal (№115) — `check_server_render_unknown_template` expects
   `check_program` to refuse `render("Unknown", ...)` and stays
   ignored.

The runtime contour is WHOLE: the templates are registered at COMPILE
time into `GLOBAL_TEMPLATES` (№250, ADR-0122 #208 — the №115 TW/VM-parity
channel; the VM serve path renders from the same registry), the
vm_golden corpus sweeps fully green (№635/gh#1103: 189/189). The static
contour has the №617 leeway already: the OpaqueConcat check refuses a
`+` whose operand carries a DECLARED opaque type (the entity/param/
let-chain fact) — the static twin of the runtime guard, "the same
refusal shifted left", with the runtime staying the backstop for the
conditionally-bound cases. The §7 gap is precisely the REST of the
distance: the implicit String→Html coercion (finding 2), the
unknown-name resolution (finding 3), and the pipe grammar (finding 1).

This ADR re-prices the options over those facts and proposes a phased
recommendation in the №624 draft-first shape: **the draft binds
nobody** — while `owner_fixed: false`, no realization naryad may cite
this ADR as its mandate; the realization naryads are born AFTER the
OWNER fixes the plan (the ADR-0186 §5 owner-only path is the shape).

## §2. The options (the prices; the parity postulate per option)

### Option (а) — the FULL compile-time port

The checker refuses: (1) the implicit String→Html coercion at
assignment/argument positions (№114); (2) `render("Unknown", ...)`
against the template names collected from the program's AST
declarations (the checker walks the same declarations the compiler
registers — №250's channel makes the name set well-defined); (3) the
template expressions gain the pipe grammar `{{ expr | stage }}` with
check-time typing of the stages (№115).

**Price:** the parser work (the pipe grammar in template expressions —
the largest single item; the template body mini-language grows a stage
list), the checker work (an assignment-position coercion rule + the
AST name collection + the stage typing), the diagnostics movement (the
failures move left — every program that silently coerced or hit the
runtime refusal now fails at check time; the honest direction, but the
corpus and the book pages must be re-synced), the drift risk (a
check-time model of the template semantics can drift from the render
engine — the №250 idempotent-registration single-source discipline must
extend to whatever the checker models), and the docs (the book's
template chapter, REFERENCE). The vm_golden corpus must stay fully
green (№635's 189/189 is the regression floor).

**Parity postulate (ADR-0073 discipline):** the semantic checker is the
SHARED front door — a refusal computed there is backend-neutral by
construction (the TW and the VM consume the same check verdict before
any execution); the render-time engine stays backend-shared through
№250's registration channel. The honest gap: the diff fuzzer has no
template group today — the postulate is pinned by the corpus + the
check-level tests, not by a generative lane; opening a fuzzer template
group is part of the option's price if the OWNER demands the generative
pin.

### Option (б) — the PARTIAL port (the security-weighted pair, no pipe)

The checker refuses (1) the implicit String→Html coercion and (2) the
unknown-template names — the two findings where the late (runtime)
failure is the SECURITY exposure (an injection-shaped value reaches the
page; a missing template surfaces as a request-time 500). The pipe
syntax stays runtime-absent (№115 stays open as a documented gap).

**Price:** no parser work at all; the checker work is the AST name
collection (finding 3 — bounded, the declarations are in the checked
AST) plus the coercion rule (finding 2 — the №114 design question
narrowed to assignment/argument positions). The diagnostics movement
and the doc re-sync still apply, but strictly smaller than (а). The
drift risk is minimal — the checker models NAMES and COERCIONS, not the
render semantics.

**Parity postulate:** identical shape to (а) — the refusals live in the
shared checker; no generative-lane change demanded.

### Option (в) — the runtime status-quo with a documented boundary

Nothing moves; the book documents the honest boundary ("the template
semantics refuse at runtime; the checker does not model templates") and
the §7 ignore trio stays as the recorded known-issue surface.

**Price:** near zero now; the exposure stays — the late failures are
exactly the failures the static contour exists to catch (the №617
posture: "the static refusal fires earlier on BOTH backends, the
runtime stays the backstop"); option (в) leaves the §7 lane permanently
open and the ledger finding permanently live.

**Parity postulate:** vacuous — nothing moves, both backends keep the
identical runtime guard.

## §3. The phased recommendation (a draft, NOT a decision)

**The executor's recommendation:** phase 1 = option (б) (the
security-weighted pair — the highest honest value per unit of checker
complexity, no parser work), phase 2 = the pipe grammar with check-time
typing (completing (а)); option (в) is the fallback if the OWNER
re-prices the checker work above the value. The recommendation
deliberately leaves the choice — including (в) — open to the OWNER.

- The phase 1 realization lifts the §7 ignores 2 and 3 (each ignore
  becomes its compile-time refusal test, green on both backends);
- the phase 2 realization lifts the ignore 1;
- the ledger §7 lane closes ONLY when all three lift (the №639 revision
  records the ADR path: Accepted → realization → the lifts).

## §4. The draft acceptance floors (X-3, owner_fixed: false)

The plan carries NO fact keys and NO gate record (№525: a fact without
a checker is forbidden — the floors are fixed by the OWNER together
with the plan). The draft floors:

- the realization PRs lift the §7 ignores ONLY with their refusal tests
  green on BOTH backends (the live fact today: the trio is ignored —
  any floor demands movement);
- the vm_golden corpus stays fully green (the №635 floor);
- no new known-divergence pins (№503); the diagnostics movement is
  documented in the same PR that moves it (the corpus/book re-sync is
  part of the realization, not a follow-up).

## §5. References

- the Known-Issue ledger gh#967 §7 (the three findings — this ADR's
  object; the §7 ignores are NOT lifted by this naryad — they lift by
  the realization AFTER the OWNER accepts);
- №114 (the opaque Html coerce), №115 (the pipe syntax; the
  unknown-template refusal), №250/ADR-0122 #208 (the compile-time
  registration — the parity channel), №617 (the static leeway already
  in place), №635 (the vm_golden 189/189);
- ADR-0073 (the parity-postulate discipline), ADR-0187 (the draft-first
  form this ADR follows), №639 (the ledger revision that records the
  path), №623/№637 (the wave-36 typed context — unrelated to this
  lane's scope).
