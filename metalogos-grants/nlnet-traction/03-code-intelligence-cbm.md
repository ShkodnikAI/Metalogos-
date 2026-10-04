# Porting the code-intelligence class into the language — the CBM pilot extract

Draft for the NLnet/Restack traction package (№577, the owner's directive
2026-10-04). This is the extract of the live CBM measurement (2026-10-03,
the pilot branch archive) with its methodology and its limits, stated
honestly. Every Metalogos-side number is reproducible by a checked-in
generator.

## The claim, in one line

The code-intelligence queries that generic tools answer badly — "which
functions call this security-relevant helper, and which call sites must a
refusal respect?" — are answered EXACTLY by the language's own audit
machinery, because the language has the semantics the generic tool lacks.

## The measurement (2026-10-03, the pilot branch archive)

The reference: `codebase-memory-mcp v0.11.0` — a code-intelligence tool of
the "code intelligence" class, a well-known open-source representative of
the same class as the ★45.7K semantic-code tools (the exact star figure is
the pilot-branch record; it is not a repository fact of THIS repo and it is
not load-bearing here — the class is).

The probes (both run against the same live source tree, Metalogos at the
pilot commit):

| Probe | The generic tool (codebase-memory v0.11.0) | The language's own counter (`debt_counters.py` line) |
|---|---|---|
| Find the unique caller functions of `get_expr_taint` (the security-audit helper) | sees **3 of 5** caller functions (60% of functions; 64% of call sites) — the import-alias and macro-shaped callers are invisible to it | the audit lane holds the FULL call graph of `.mlog` AND the Rust-side mirror registry — the counter reads the same graph the compiler executes |
| `binding_label` liveness | reports DEAD (a false positive) — `crate::`-qualified calls are not resolved, so the function looks uncalled | alive and load-bearing — the language resolves its own paths |

The timing fact: the Metalogos-side counter answers in **0.24 s** over the
live tree, with the gate semantics (only-down floors, fail-closed) and
reproducibility (the same output on every run; the baselines are
checked-in files with a written history).

## The methodology

1. One tool of the target class, one pinned version (v0.11.0), default
   configuration — no tuning of the tool against itself.
2. One live source tree, one commit; both probes read the same tree.
3. The ground truth established by hand BEFORE the run (the five caller
   functions enumerated by `rg` over the tree; the `binding_label` call
   sites resolved by hand) — the hand enumeration is the reference the
   tool is graded against.
4. The Metalogos side uses the checked-in counter only — no ad-hoc script.

## The honest limits

- **The class boundary**: codebase-memory-mcp is a CLI/tooling-class tool;
  the comparison is against the tool, not against the strongest LLM-based
  code-intelligence service. The class, not the brand, is the point.
- **The visibility boundary**: the generic tool does not see `.mlog`
  programs at all — the language's own surface (250+ example programs,
  the stdlib, the office suite) is invisible to it. The language's counter
  sees both worlds by construction.
- **The upstream can catch up**: path-resolution and alias-aware indexing
  are known hard problems the upstream projects actively improve. The
  durable advantage is NOT the specific resolver — it is that the
  security-relevant facts (taint labels, floors, refusal classes) LIVE IN
  THE LANGUAGE SEMANTICS, so the language's own counters never drift from
  the language's own behavior. A tool that catches up on indexing still
  does not know what a SINK_CLEARANCE refusal means.
- **One tree, one commit**: the pilot ran on a single live tree at a
  single commit; the full per-probe logs live in the pilot-branch archive
  (2026-10-03). The numbers here are the extract, not the raw logs.

## Why this belongs in the traction package

The NLnet/Restack funding thesis of Metalogos is: information-flow
security must be a property of the LANGUAGE, not of the tooling around it.
This measurement is the cheapest demonstration of that thesis: the
language's own machinery answers a code-intelligence question more
completely than a dedicated tool of that class — and the answer is the
same machinery that refuses unsafe programs at compile time.
