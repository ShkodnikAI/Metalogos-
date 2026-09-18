# AGENTS.md — Metalogos for the AI coder agent

> This file is not a replacement for README/REFERENCE.md/ADR. It is the
> entry point to the **methodology**: where the truth lives for each
> class of question, which mistakes to avoid, what "a naryad is done"
> means. Read it first, before any task.

---

## 1. The main rule: code is the source of truth, not prose

Comments, README, ADRs **may diverge from the real code** — not
because someone lied, but because the code changes faster than the
description next to it gets updated. Multiple times per day one sees:
a comment describes a carefully designed policy while the code does
not implement it (example: the comment above `check_secret_leak`
described handling of `http_post`, but `SINK_FUNCTIONS` did not
contain that function at all — naryad #157). Or the reverse: the code
was fixed long ago, while an ADR still describes the old problem as
unsolved.

**Rule:** before acting on a textual description of anything in this
project — **verify with a fact**: `grep` over the real code, a real
run, real CI. Not "README says X" → "I proceed from X", but "README
says X" → "grep confirms X" → "proceed".

## 2. Project identity — do not copy human languages

Metalogos does not reinvent Python/Rust with different syntax. The
seven pillars (`Entity`/`Pattern`/`Flow`/`Memory`/`Rule`/`Learn`/`Adapt`)
are not alternative names for variables/functions/loops but a
different level of primitives, designed for how an AI reasons, not a
human. When adding a new capability — not "how do other languages do
it", but "how should it be done here, if done honestly more elegant".

**Practical consequence for new large branches (example — training
local models, naryads #175/176):** if the result boils down to
"Metalogos calls an external library the same way Python would call
it" — that is a wrapper, not the project identity. The sign of a real
integration is a new capability that uses the already-existing
pillars, not a separate island living next to them. Example: a
trained model as a source of confidence for `Fluid` (`ADR-0006`), not
a separate, standalone API.

## 3. Decision boundary — where to act independently, where to ask

**Act independently, without asking permission for every step:**
engineering-aesthetic decisions — module structure, builtin
signature, file split, document wording. Here engineering taste
decides, not product strategy.

**Ask for an explicit owner decision, do not assume:**
irreversible actions (rewriting git history) and anything that
changes the project identity — revising already-accepted `ADR`s
(`Result`/`Option` — rejected, `ADR-0106`; VM parity — deferred,
`ADR-0105`; a real `adapt` metric — deferred, `ADR-0112`), a new
semantic pillar (not a mechanical extension of an existing one). The
boundary is not distrust of one's own judgment, but that the course
of the project is decided by the owner, not the agent.

## 4. Synchronization — the most frequent own mistake

**Twice in one day (today) an action was taken based on a stale
local checkout**, even though `git reset --hard origin/main` seemed
to have been invoked. The reason — a previous `git fetch`/`clone`
had cached the state BEFORE the needed merge happened, and the
subsequent `reset --hard` pinned exactly that stale state.

**Mandatory sequence before any check:**
```bash
git fetch origin main --force   # exactly --force, exactly main explicit
git log origin/main -1 --format='%H %ci %s'   # look at what actually arrived
git reset --hard origin/main
git log -1 --format='%h %s'     # confirm it matches the previous line
```
If `git fetch` prints `xxxxx..yyyyy main -> origin/main` — it means
the branch REALLY advanced just now, was not already up to date
earlier.

## 5. Where the truth lives — by class of question

| Question | Do NOT look at (may be stale) | Look at (source of truth) |
|---|---|---|
| Does builtin X exist? | REFERENCE.md (coverage ~59%, not 100%) | `grep 'spec!("X"' src/builtins/registry.rs` |
| What is the arity of a builtin? | Prose description | The exact signature in the same `spec!()` |
| Does X work the same in TW and VM? | Claims in README | `tests/crosscheck_backends.rs` — is the example included in the comparison, or explicitly excluded with an ADR reference |
| What does a security check guarantee? | The check_id name | The function itself in `src/audit.rs`, its `Severity`, and whether it is actually wired into `audit_category_a` (does it compile a blocking error) |
| What are the real current language limitations? | Scattered mentions | `docs/threat-model.md` — the single, maintained point |
| Was an architectural decision made and why? | Past discussions/memory | `docs/adr/README.md` (index) → the specific `docs/adr/NNNN-*.md` |
| Are the numbers current (builtins/ADR/examples)? | Never trust static numbers in prose as fact | Recount directly (`ls`, `grep -c`, `wc -l`) — README numbers are synchronized by autotests (`tests/readme_consistency.rs`), but between runs they may lag |
| What does CI actually check? | Job names | `.github/workflows/ci.yml` — what each job actually runs |

## 6. Quick ADR search — do not read all 110 in a row

`docs/adr/README.md` is the index with numbers and topics, but for a
specific question it is faster to:
```bash
grep -rl "keyword" docs/adr/*.md
```
Frequent topics where the decision is already made — do not reopen
without an explicit owner request:
- `Result`/`Option` instead of soft-failure → **rejected**, `ADR-0106`
- A real metric instead of mock `accuracy` for `adapt` → deferred to a
  real case, `ADR-0112`
- VM parity with TW (`match`, `BlockIfElse` expression) → deliberately
  not implemented without real need, `ADR-0105`
- Bytecode format `.mbc` → `bincode` with an explicit size limit,
  `naryad #146`

## 7. Known typical mistakes when working with this code

**False grep results due to a wrong pattern.** Functions/tests in
this project are not always named as one would intuitively expect —
the test naming convention is often `n<NNN>_<description>` or
`naryad_<NNN>_<description>.rs`, not `test_<description>`. An empty
result of `grep 'fn test_'` does not mean "there are no tests" —
first check the real file content (`cat`/`wc -l`), then refine the
pattern.

**`mlog check` without `--root` does not resolve imports between
files** — a multi-file project gives false `undefined` errors. Use
`mlog check <file> --root <entry.mlog>`.

**Do not assume a third-party type implements a trait** (`PartialEq`,
`Debug`, etc.) without checking — at least one naryad today broke
exactly on this (`Value` does not implement `PartialEq`, the test
compared directly).

**A local toolchain in a sandbox may not build the whole project**
(`Cargo.lock`/edition version mismatch) — this does not mean nothing
can be verified: `rustfmt --edition 2021 <file>` and `git diff` work
independently of a full build; real CI is the final source of truth
on compilation.

## 8. The naryad contract — what "done" means

1. Branch → `git push` to the remote repository → **a PR is open**. A
   branch left only locally or only pushed without a PR does not
   count as delivered, regardless of code readiness (`ADR-0110`).
2. Every claimed feature is confirmed by a **real** CI run (not a
   local `cargo test`, when a real run is possible) — blocking jobs
   green on the actual merge commit, not on an intermediate one.
3. If a naryad implies several separate changes — one commit per
   logical step, not a single dump, unless that hurts the task volume
   sharply.
4. The report names explicit assumptions and what remains unresolved —
   it does not present a partial result as a complete one.

## 9. What is not documented centrally anywhere, but is useful to know

- Roughly every few naryads the `BUILTIN_REGISTRY` and its usage
  change structurally (naryad #170 — SSOT refactoring) — when in
  doubt about how builtins are registered, look at
  `src/builtins/mod.rs`; do not rely on memory of the old structure.
- `main` is the single branch of truth; open PRs may lag behind it by
  many commits if other work ran in parallel — compare before merge,
  not only before starting work.

## 10. Documentation language — English only

Owner directive (2026-09-17): **all documentation in this repository
is written in English only.** Every new CHANGELOG entry and every new
document (README sections, `docs/*.md`, ADRs, research reports) must
be authored in technical English; Russian prose in documentation
files is a defect and is caught by the `tests/docs_language_lint.rs`
CI gate. Cyrillic inside code examples and test fixtures (string
literals that are data, identifier-support demos) is content, not
documentation language, and is allowed. The terminology is fixed:
«наряд» → *naryad*, «диспатч» → *dispatch*.

---

*Living document — update when a new class of typical mistake or a
new source of truth is discovered, using the same naryad protocol as
the rest of the documentation.*
