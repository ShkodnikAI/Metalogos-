# REALITY.md — fact-check of assets and a working estimate of P0 readiness

> **Status: SSOT for P0 readiness.** This page is the single source of
> truth on the question "what of what is claimed in plan v2 actually
> exists, and what share of P0 readiness has been reached". Created by
> naryad #318 (P0/docs, issue #405, wave 0, step 0.1 per §13.2 of plan
> v2). Updated or withdrawn only by a new fact-check under the same
> naryad protocol — edits outside the protocol are forbidden: every
> number here is either reproducible by a command or marked UNVERIFIED.
>
> **Snapshots.** Plan v2 snapshot — `fc59e9e` (2026-09-14 13:56 +0300,
> merge #396). Current main at the time of the check — `1876fdf`. All
> commands of section 1 are reproducible on the snapshot **without a
> checkout** — via `git show fc59e9e:<path>`; wherever the main anchor
> has shifted, this is stated explicitly and the new anchor is recorded
> (section 5).
>
> **Honesty.** Plan v2 is ABSENT from the repository — absence check:
> `git ls-tree -r --name-only HEAD | grep -iE 'plan'` returns only
> `docs/refactoring-split-plan.md` (a different document) and false
> matches on `openplanter`. The text of §2 of the plan is available only
> as a quotation in the body of issue #405. Everything that requires the
> full text of the plan (the decomposition weights in section 3) is
> marked UNVERIFIED and is a working reconstruction, to be reconciled
> once plan v2 appears in the repo.

---

## 0. Verdict summary

| # | Anchor of plan v2 §2 | Verdict | Snapshot `fc59e9e` | Main `1876fdf` |
|---|---|---|---|---|
| 1 | `TaintKind` — audit.rs:119, "5 advisory kinds" | **CONFIRMED** | audit.rs:119, 5 variants | no shift (119) |
| 2 | `TaintTracker` — audit.rs:142, per-scope HashMap | **CONFIRMED** | audit.rs:141–142 | no shift (141–142) |
| 3 | Category-A gate `MODEL_WEIGHTS_UNSAFE` — audit.rs:1607–1757 | **CONFIRMED** (anchor shifted) | 1607–1757 | **1660–1810** (+53) |
| 4 | `Statement` — ast.rs:1282, "10 kinds" | **PARTIAL** | ast.rs:1282, **15** variants | no shift |
| 5 | "84 VM instructions" — bytecode.rs:14 | **PHANTOM** | **47** variants | no shift (47) |
| 6 | `semantic.rs` — 3328 lines | **CONFIRMED** | 3328 | 3328 |
| 7 | 420 builtins — registry.rs:38 | **CONFIRMED** count / **PARTIAL** line | 420 `spec!(`; declaration on line **44** | **421** (+video_extend, #309) |
| 8 | "143 ADRs" | **PARTIAL** | 142 ADRs + index README = 143 files | **145** ADRs (+0151/0152/0153) |
| 9 | "214 examples" | **CONFIRMED** (canonical basis) | 214 top-level `*.mlog` | 214 (273 recursively) |
| 10 | zeroize — Cargo.toml:51 | **CONFIRMED** | line 51 | no shift |
| 11 | `consent_ledger` — voice/store.rs:31 | **CONFIRMED** | store.rs:31 | no shift |
| 12 | Provenance — vision/provenance.rs (#241) | **CONFIRMED** | 430 lines | 464 (#320) |
| 13 | MCP client — builtins/mcp.rs (#268) | **CONFIRMED** | 613 lines | 613 |

Total: 9 CONFIRMED (1 of them with a shifted anchor), 3 PARTIAL, 1
PHANTOM. Not a single anchor proved to be wholly fabricated — the only
gross error of plan v2 is the number of VM instructions (item 1.5).

---

## 1. Line-by-line verification of assets

### 1.1. `TaintKind` — audit.rs:119, "5 advisory kinds" — CONFIRMED

Command (snapshot):
```bash
git show fc59e9e:src/audit.rs | sed -n '119p'; git show fc59e9e:src/audit.rs | awk '/^enum TaintKind/,/^}/' | grep -cE '^\s{4}[A-Z][A-Za-z]*,?\s*$'
```
Actual output: `119: enum TaintKind {` — exactly line 119; the number of
variants is **5**. Full list: `LlmOutput`, `Secret`, `UserInput`,
`Sanitized`, `CanaryLeak` (#284). On main — no shift. Clarification to
the plan's wording: the kinds themselves are label carriers, not
"advisory kinds"; it is the **consuming check** of the label that is
advisory or blocking (see section 2 and the check_id dictionary: 21
identifiers in audit.rs on main).

### 1.2. `TaintTracker` — audit.rs:142, per-scope HashMap — CONFIRMED

Command (snapshot):
```bash
git show fc59e9e:src/audit.rs | sed -n '141,142p'
```
Actual output:
```text
struct TaintTracker {
    tainted: HashMap<String, TaintKind>,
```
Line 142 is exactly the field `tainted: HashMap<String, TaintKind>`
(the struct declaration is 141). "Per-scope" is confirmed by the doc
comment above the struct and by the three-method API (`taint` /
`get_taint` / `untaint`); `#[derive(Clone)]` — for the path-sensitive
fork in `check_canary_leak` (#284). On main — no shift.

### 1.3. Category-A gate `MODEL_WEIGHTS_UNSAFE` — audit.rs:1607–1757 — CONFIRMED (anchor shifted)

Command (snapshot):
```bash
git show fc59e9e:src/audit.rs | grep -n 'MODEL_WEIGHTS_UNSAFE' | head -3
```
Actual output: `1607` — the section header
`// ── Check: MODEL_WEIGHTS_UNSAFE + VISION_POLICY_MISSING`, `1757` —
`check_id: "MODEL_WEIGHTS_UNSAFE"` (Severity::Error, Category A —
statically visible violations at `vision_fetch_weights(url, ...)`).
The range 1607–1757 is confirmed as the "gate section" on the snapshot.
**On main the anchor has shifted: the section is now 1660–1810** (the
template comment
`// MODEL_WEIGHTS_UNSAFE / VISION_UNSIGNED_EXPORT template (1607–1757)`
in main's code preserves the historical coordinates). New anchor:
**1660**.

### 1.4. `Statement` — ast.rs:1282, "10 kinds" — PARTIAL

Commands:
```bash
git show fc59e9e:src/ast.rs | sed -n '1282p'
sed -n '1282,1400p' src/ast.rs | awk '/pub enum Statement/{f=1;next} f&&/^\}/{exit} f' | grep -cE '^\s{4}[A-Z][A-Za-z0-9]*'
```
Actual output: `1282: pub enum Statement {` — the position is exact **on
both snapshots**; the full variant count is **15**, not 10: `LetBinding`,
`Assign`, `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`,
`Return`, `ExprStmt`, `Match`, `Break`, `Continue`, `Memorize`, `Forget`,
`Relate`.

The verdict is PARTIAL, not PHANTOM: the number "10" is reproducible with
the basis "15 minus the Memory variants (Memorize/Forget/Relate) minus
loop-control (Break/Continue)" = 10, but this basis is not stated in the
plan and matches none of the project's counters. We note in passing the
discrepancies found in the README — three places with three different
Statement counters, none covered by consistency tests: the architecture
diagram ("12 Statement", alongside "29 Declaration" / "15 Expr" against
the actual 33/14), the AST table ("12 Statement") — all were fixed in
the same naryad to the verified 33/14/15.

### 1.5. "84 VM instructions" — bytecode.rs:14 — PHANTOM

Commands:
```bash
git show fc59e9e:src/bytecode.rs | sed -n '14p'
git show fc59e9e:src/bytecode.rs | awk '/pub enum Instruction/,/^\}/' | grep -E '^\s{4}[A-Z][A-Za-z0-9]*' | grep -v '//' | wc -l
grep -rn '84 инс\|84 instr\|84 instructions' README.md docs/ src/
```
Actual output: `14: pub enum Instruction {` — the position is exact; the
number of variants is **47** on the snapshot and on main; the string
"84 instructions" occurs **nowhere** in the repository. The README itself
agrees with reality: "bytecode VM (47 instructions; experimental …,
ADR-0105/ADR-0141)". Verdict PHANTOM: no counting basis (enum variants,
opcodes, instructions with operands) yields 84. The number 84 in §2 is a
plan error, most likely a carry-over from another revision.

### 1.6. `semantic.rs` — 3328 lines — CONFIRMED

Command (snapshot):
```bash
git show fc59e9e:src/semantic.rs | wc -l
```
Actual output: `3328` — exactly. On main — 3328 (unchanged).

### 1.7. 420 builtins — registry.rs:38 — CONFIRMED (count) / PARTIAL (line)

Commands (snapshot):
```bash
git show fc59e9e:src/builtins/registry.rs | sed -n '38p'
git show fc59e9e:src/builtins/registry.rs | grep -c 'spec!('
```
Actual output: line 38 holds the tail of a `use` import (`};`), not the
registry; the declaration `pub const BUILTIN_REGISTRY` is on line **44**.
The count of `spec!(` — **420** on the snapshot — is confirmed exactly.
The numeric part of the anchor is correct; the line coordinate is
inaccurate (38 → 44). **On main — 421** (+`video_extend`, naryad #309);
the README counter is synchronized by an autotest
(`readme_total_builtins_match_reality`).

### 1.8. "143 ADRs" — PARTIAL

Commands (snapshot):
```bash
git show fc59e9e --stat >/dev/null; git ls-tree -r --name-only fc59e9e docs/adr/ | grep -c '\.md$'
git ls-tree -r --name-only fc59e9e docs/adr/ | grep '\.md$' | grep -vcE '/[0-9]{4}-[^/]+\.md$'
```
Actual output: the `.md` files in `docs/adr/` on the snapshot total
**143**; of them 142 are files of the `NNNN-*.md` format, 1 is the index
`README.md`. The project's canonical counter (`real_adr_count()` in
`tests/readme_consistency.rs`) excludes the README, i.e. on the canonical
basis the snapshot has **142 ADRs**. "143" is reproducible only with the
basis `ls docs/adr/*.md | wc -l` (index included). On main: **145** ADRs
(+ADR-0151 #309, +ADR-0152 #320, +ADR-0153 #412) + index = 146 files;
the README claim is synchronized by an autotest.

### 1.9. "214 examples" — CONFIRMED (canonical basis)

Commands (snapshot):
```bash
git ls-tree --name-only fc59e9e examples/ | grep -c '\.mlog$'
git ls-tree -r --name-only fc59e9e examples/ | grep -c '\.mlog$'
```
Actual output: **214** top-level `*.mlog` — exactly the plan's number;
recursively — 229 (with subdirectories). The project's canonical basis
is top-level: that is how `real_example_count()` in
`tests/readme_consistency.rs` counts (non-recursive `fs::read_dir`), and
214 is exactly what the README claims ("214 .mlog programs (golden
corpus)"). Verdict CONFIRMED; on main the top level is the same 214
(273 recursively: naryad #317 added the `examples/leak/` corpus,
excluded from the golden loop by construction — `golden.rs` scans
non-recursively).

### 1.10. zeroize — Cargo.toml:51 — CONFIRMED

Command (snapshot):
```bash
git show fc59e9e:Cargo.toml | sed -n '51p'
```
Actual output: `zeroize = "1"` — exactly line 51 (the "Phase 7.3: Real
encryption" block, `argon2 = "0.6"` nearby). On main — no shift
(`grep -n 'zeroize' Cargo.toml` → `51:zeroize = "1"`).

### 1.11. `consent_ledger` — voice/store.rs:31 — CONFIRMED

Command (snapshot):
```bash
git show fc59e9e:src/voice/store.rs | sed -n '31p'
```
Actual output: `/// CREATE TABLE consent_ledger (` — exactly line 31
(a schema doc comment). The real schema lives in code: `CREATE TABLE IF
NOT EXISTS consent_ledger` (store.rs:61 on main) + the API
`record_consent` (:127), `has_consent_record` (:143), `consent_count`
(:157) + the test `consent_ledger` (:225). On main — line 31 unshifted.

### 1.12. Provenance — vision/provenance.rs (#241) — CONFIRMED

Command (snapshot):
```bash
git show fc59e9e:src/vision/provenance.rs | wc -l
git show fc59e9e:src/vision/provenance.rs | grep -n 'pub fn' | head -8
```
Actual output: **430 lines**; public API: `sha256_hex`, `prompt_hash`,
`verify_sha_pin`, `manifest_sidecar_json`, `weights_tree_sha256`,
`model_hash32`, `embed_lsb_watermark`, `detect_lsb_watermark` — the full
provenance contour (SHA-pinning of weights, sidecar manifest, LSB
watermark). On main — 464 lines (naryad #320 added the manifest's
synthetic fields, Art 50).

### 1.13. MCP client — builtins/mcp.rs (#268) — CONFIRMED

Command (snapshot):
```bash
git show fc59e9e:src/builtins/mcp.rs | wc -l
```
Actual output: **613 lines**, unchanged on main. The content is
confirmed by the module doc block (taint contract: the result of
`mcp_call` is `TaintKind::UserInput`) and by the README (stdio
transport, hand-rolled JSON-RPC, exec gate, allowlist
`METALOGOS_MCP_ALLOWLIST`, ADR-0132).

---

## 2. What makes taint an effect — and what is missing

Context: the taint mechanics of Metalogos are **a detector and gates on
specific patterns**, not an abstract interpreter with effects. This is
not an implementation defect but an exact boundary: below, every "not
ready" item from the plan is confirmed by an absence-proof command and
tied to the place where it is to appear. All commands run on `1876fdf`
from the repository root and are reproducible on any checkout where the
effects do not yet exist.

**2.1. Label lattice — NO.**
```bash
grep -rni 'lattice' src/        # output: empty (0 occurrences)
```
`TaintKind` is a flat `enum` with no ordering, no least upper bound, and
no absorption operator. Where it will appear: `src/audit.rs`, the
section `// ── Taint tracking for data-flow analysis ──` (lines 115–435
on main).

**2.2. Inference over all statement kinds — NO (9 of 15).**
```bash
sed -n '2404,2983p' src/audit.rs | grep -c 'Statement::Match\|Statement::Break\|Statement::Continue'
# output: 0
```
The interprocedural MVP `TAINT_INTERP` (#292, section 2404–2983) handles
9 kinds: `LetBinding`, `Assign`, `ExprStmt`, `Return`, `Each`,
`EachWithIndex`, `While`, `IfElseBlock`, `IfThen`. Not covered by the
engine: `Match`, `Break`, `Continue` (0 occurrences in the section) and
the Memory variants `Memorize`/`Forget`/`Relate` (they are handled by
separate pattern checks `TAINT_PERSISTENCE`, not by the interp engine).
Where it will appear: the same `TAINT_INTERP` section — extending the
match arms for the missing kinds.

**2.3. Join at merges — NO.**
```bash
grep -n 'fn join\|taint_join\|merge_taint\|widen' src/audit.rs   # output: empty
```
The only branching-sensitivity mechanism is the path-sensitive fork of a
tracker clone in `check_canary_leak` (#284, `#[derive(Clone)]` on
`TaintTracker`): state flows into two branches and **does not merge**
back. Where it will appear: the merge point of after-branches in
`TAINT_INTERP` and in the #284 fork.

**2.4. Effect traces — NO.**
```bash
grep -n 'effect' src/audit.rs    # output: empty
```
The only occurrence of the word in an adjacent zone is a historical
comment in `semantic.rs:2901` about the lifetime of a binding within a
block; the code has no effect concepts (write/read/network/
irreversibility as attributes of expressions). Where it will appear: a
new section in `src/audit.rs` next to the taint engine, or a
`src/effects.rs` module later tied to the builtin classification
(`src/builtins_classification.rs`, naryad #316: Role/Lift/Sink are
already enumerated — 205 non-Pure out of 421).

**2.5. Exhaustive matching over labels — NO (as the central contract).**
```bash
grep -n 'match kind' -A 8 src/audit.rs   # output: empty
```
The rules are scattered across point matches: `get_expr_taint` implements
"sanitizer wins" (`render`/`escape_html` → `Sanitized`) and "first
unsanitized argument"; the section checks match specific kinds (`Secret`,
`UserInput`, `CanaryLeak`). There is no central exhaustive match that
the compiler would force to extend when a new label kind is added — a
new kind can be added "invisibly" to part of the checks. Where it will
appear: in the taint section of audit.rs as a single
transformation/absorption function accompanying the lattice (2.1).

**2.6. Affinity — NO.**
```bash
grep -rni 'affinity' src/        # output: empty
```
The code has neither data-to-flow/actor binding nor affine "one-shot
consumption" types. Where it will appear: after effects (2.4) — as a
consumption restriction at the semantics level (`src/semantic.rs`) with
a gate in `audit.rs`.

The existing part (so that this section does not read as "there is no
engine at all"): labels are placed on sources
(`call_llm`/`env`/`form_data`/`mcp_call`), removed by sanitizers
(`render`/`escape_html`; `redact` removes `Secret` but does NOT remove
`CanaryLeak`, the ADR-0136 D2 template), propagated through
`Ident`/`FnCall`/`BinaryOp`, and feed 21 check_ids, of which the
blocking ones (Severity::Error) are included in `audit_category_a` /
`audit_program` and fail compilation at `mlog check`. The dictionary of
the corpus classes that "must not compile" is `tests/run_leak_suite.rs`
(#317); the gate completeness measured today is 11/28 scenarios caught
(39%), the remaining 17 are the lattice contract of #325.

---

## 3. Recomputed P0 readiness: **26%** (the working figure of Phase 1)

> **UNVERIFIED caveat.** The subsystem weights are a reconstruction from
> the list "labels / capability / backend registry / ledger / memory"
> (issue #405 body, referencing §2 of plan v2); the plan itself is
> absent from the repo, so the weights are unverifiable and are adopted
> as working values until the plan appears in the repo. The readiness of
> each subsystem, by contrast, rests only on the code facts of sections
> 1–2. The sum is 25.85 ≈ **26%**, within the target band of ~25% ± 5pp
> set by issue #405.

| Subsystem | Weight (UNVERIFIED) | Readiness | Contribution | Code basis for readiness |
|---|---|---|---|---|
| Labels (taint) | 30% | 55% | 16.5pp | `TaintKind` 5 kinds, per-scope tracker, propagation, sanitizers (ADR-0136), path-sensitive canary (#284), interp MVP (#292, depth 2), 21 check_ids, the blocking ones in Category-A; **missing**: lattice, join, full inference over kinds, effects, exhaustive matching, affinity (section 2) |
| Capability model | 20% | 0% | 0 | `grep -rni 'capability' src/` → empty; the handle template — ADR-0114, not applied |
| Backend registry | 15% | 10% | 1.5pp | No unified registry/router (`BACKEND_REGISTRY` — 0 occurrences); what is real are per-pillar registries: `VOICE_REGISTRY` (voice/mod.rs:119), `VIDEO_REGISTRY` (video/mod.rs:249), the vision allowlist `MLOG_VISION_WEIGHTS_ALLOWLIST`, `BUILTIN_REGISTRY` (421) — but these are content registries, not compute backends |
| Ledger | 15% | 35% | 5.25pp | `consent_ledger` is real (voice/store.rs:61/127/143/157 + test :225); subprocess audit log (README, MCP gates); **missing**: a universal action ledger, tamper-resistance, grant mechanics for irreversible operations (the `IRREVERSIBLE_NO_GRANT` class is planned only, #325) |
| Memory | 20% | 13% | 2.6pp | Real local infrastructure: memory_store.rs (1621 lines), memory_graph.rs (949), embeddings.rs (655), BM25+vector rank fusion, kv_/mem_ builtins; **missing**: the plan v2 memory P0 contract (its scope is unknown without the plan — UNVERIFIED); `recall` is a registry spec string without a handler (`spec!("recall", 0, "stub")`, registry.rs:248) |
| **Total** | **100%** | — | **25.85 ≈ 26%** | |

**Why not ~60% (v1): five reasons for the discrepancy.**

1. **Different definitions of readiness.** v1 measured functional width —
   "code written" (421 builtins, three media pillars, MCP, VM). The
   working definition of P0 is "contract closed": gate + registry +
   ledger + provable behavior. Width ≠ readiness: not one of the 421
   builtins carries a capability attribute — of which there are none.
2. **Empty subsystems invisible to v1.** Capability — 0 occurrences in
   `src/`; a unified backend registry — 0; taint join/lattice/effects —
   0. The combined weight of these holes in the decomposition is 35+
   percentage points at zero contribution.
3. **Advisory ≠ gate.** audit.rs has 28 occurrences of `Severity::Warning`
   — detectors, not launch blockers. v1 counted them as coverage; for P0
   only the blocking part counts (`Severity::Error`, included in
   `audit_category_a`).
4. **Indicators of width-without-readiness.** `spec!("recall", 0, "stub")`
   is a registry string without a handler; the VM is experimental
   (ADR-0105/0141, full-language constructs do not compile to bytecode),
   and the "84 instructions" from §2 are PHANTOM (actually 47, item 1.5).
5. **Limits of the taint engine.** Intraprocedural depth is bounded
   (`TAINT_NESTING_MAX_DEPTH=3`), interprocedural — 2
   (`TAINT_INTERP_MAX_DEPTH=2`, warnings `INTERP_DEPTH_LIMIT`);
   persistent taint is file/module scope only (limitations.md);
   v1 read the presence of `TaintKind` as "taint ready".

**Decision (adopted as the working one for Phase 1, item (b) of the
DoD):** the figure **26%** with the decomposition in the table above is
the planning basis of Phase 1. Recomputation — upon completion of each
wave, under the same naryad protocol, with edits to this page only.

---

## 4. Precedent assets to rely on

| Asset | What it provides as a template | Where |
|---|---|---|
| ADR-0114 (reflex-opaque-handle) | "Opaque handle": the real object exposes only an identifier outward — a direct template for the capability model (the subsystem at 0%) | `docs/adr/0114-reflex-opaque-handle.md` |
| ADR-0125 + #241 | Category-A static gate: a statically visible violation → `Severity::Error` at the call site; the 1607–1757 section template is already replicated (VISION_UNSIGNED_EXPORT, VISION_UNSIGNED_EXPORT_RAW, MEDIA_SYNTHETIC_UNMARKED #320) | `docs/adr/0125-vision-provenance-gates.md`; `tests/naryad_241_vision_gates.rs` |
| ADR-0136 + #274 | A sanitizer with exact taint semantics: masking ≠ sanitization (`redact` removes Secret, does not remove CanaryLeak) | `docs/adr/0136-redact-taint-sanitizer.md`; `tests/naryad_274_redact.rs` |
| #284 | Path-sensitive taint: forking a tracker clone in the then-branch — the only existing branching mechanism; the base for a future join | `tests/naryad_284_canary.rs` |
| #261 + #130 | A layered network gate (allowlist + SSRF-guard + pinning) — applies to any new Source builtin | `tests/naryad_261_ssrf_pack.rs`; `tests/naryad_130_ssrf_guard.rs` |
| #300 | A consent gate over the ledger (`has_consent_record`) — the template for grant mechanics of irreversible operations (#325) | `tests/naryad_300_voice_gate.rs` |

All files exist on `1876fdf` (verified by `ls docs/adr/ | grep -E
'^0114|^0125|^0136'` and `ls tests/ | grep -E '241|274|284|261|130|300'`).

---

## 5. Divergences from §2 of v2 — recording the new anchors

| Anchor | §2 v2 | Snapshot `fc59e9e` | Main `1876fdf` | Comment |
|---|---|---|---|---|
| MODEL_WEIGHTS_UNSAFE | audit.rs:1607–1757 | 1607–1757 ✓ | **1660–1810** | shift +53 (sections from #309/#317 higher in the file); new anchor — 1660 |
| builtins | 420 (registry.rs:38) | 420 ✓ (declaration on 44) | **421** | +video_extend (#309); the plan's anchor line is inaccurate |
| ADR | 143 | **142** ADRs (+README = 143 files) | **145** (+README = 146) | "143" is reproducible only with the index README; the canonical basis is 142 |
| Statement | "10 kinds" (ast.rs:1282) | **15** variants | 15 | "10" is basis-dependent; the README said 12 — corrected to 15 |
| VM instructions | "84" (bytecode.rs:14) | **47** | 47 | PHANTOM; the README agrees (47) |
| Examples | 214 | 214 ✓ (top-level) | 214 (273 recursively) | the plan's basis matched the canonical one |
| semantic.rs | 3328 | 3328 ✓ | 3328 | no shift |
| TaintKind/TaintTracker | 119 / 142 | 119 / 141–142 ✓ | 119 / 141–142 | no shift |
| zeroize / consent_ledger | Cargo.toml:51 / store.rs:31 | ✓ / ✓ | ✓ / ✓ | no shift |
| provenance.rs | #241 | 430 lines ✓ | **464** | +synthetic Art 50 (#320) |
| mcp.rs | #268 | 613 lines ✓ | 613 | no shift |
| audit.rs (context) | — | 4258 lines | **4577** | growth from #309/#316/#320/#412 |

Summary: the factual layer of plan v2 is accurate on file positions and
most numbers; the erroneous numbers are "84 instructions" (PHANTOM) and
the basis-dependent "10 Statement kinds" / "143 ADRs"; all anchors that
shifted on main are recorded in this table and in section 0.
