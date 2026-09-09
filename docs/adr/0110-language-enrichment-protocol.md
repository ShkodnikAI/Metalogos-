# ADR-0110: Language Enrichment Protocol

> **Status:** Accepted  
> **Date:** 2026-08-22  
> **Decision by:** owner  
> **Related:** ADR-0106/0107/0108 (governance precedent — reject
> without demonstrated need), naryad #90 (composition-first principle),
> naryad #85 (contract-before-code lesson), naryad #92 (security-lint
> classification discipline), ADR-0105 (honest-gap-documentation
> precedent)

## Context

Over the course of one working day, a stable, repeatable practice
emerged for deciding on new language capabilities — from "it can be
composed from existing builtins, no new code needed" (naryad #90) to
"a semantic decision requires prior art and an ADR before code"
(three rejections the day before). The practice was applied
repeatedly, but was never written down as a single process — only
scattered across individual naryads and ADRs.

Formalized at the owner's direct request after reviewing an external
audit's roadmap: some of its proposals (stdlib extension, error
messages) do not require semantic decisions and can be done
independently; others (the type system) require a protocol that had
never been written down explicitly.

## Decision

A five-step protocol, applied to any future language extension:

**Step 0** — if a capability can be assembled by composing existing
builtins in `.mlog`, it is not a language feature, but an
example/pattern.

**Step 1** — split into mechanical (an established pattern, goes
straight to implementation) and semantic (requires prior art + an
ADR before code). Criterion for semantic: the decision is not made
out of general completeness considerations — only for a concrete,
reproducible case.

**Step 2** — contract before code: a `.mlog` example → a golden test
in the same commit → a minimal implementation.

**Step 3** — mandatory security-lint classification for text inputs;
verification of TW/VM parity, or honest documentation of the gap.

**Step 4** — a branch, `git push` to the remote repository, **opening
a PR** (do not leave the work only as a local commit — a naryad is
not considered delivered until the PR exists, regardless of how ready
the code is), real CI on the PR itself, synchronous documentation
updates, checking for name/number collisions before finalizing.

**Who decides:** mechanical — does not require sign-off every time;
semantic — the owner only, after being presented with the options,
with an ADR regardless of the outcome.

## Consequences

- The protocol's text is duplicated in the Project Instructions (the
  "charter", §9) — the one place actually read by every future
  builder-agent session before executing a naryad. This ADR is the
  versioned record of the same decision in the repository, not a
  replacement for the charter.
- Future naryads for new language capabilities are checked against
  this protocol before being written, not after the fact.

## Related

- ADR-0105/0106/0107/0108 — precedents for applying the "not without
  a demonstrated case" criterion
- Naryad #90 — source of the composition-before-new-code principle
- Naryad #85 — source of the "contract in the same commit" principle
- Naryad #92 — source of the security-lint classification discipline
