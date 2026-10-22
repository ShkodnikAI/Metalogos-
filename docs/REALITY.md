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

---

## 6. Recheck on main `b05d36c` (naryad №414, wave 5, 2026-09-20)

> **Snapshot of this check:** main `b05d36c9f9aaa1f70114f6c854f1157717b0792c`
> (2026-09-20, merge #564 — naryad №413). The methodology is UNCHANGED from
> section 3: the same subsystem decomposition, the same (UNVERIFIED) weights,
> readiness per code facts only. Every command below was executed on this
> snapshot from the repository root; its verbatim output is recorded.

### 6.1. What landed since `1876fdf` (the section-3 snapshot)

Waves 3 / 4 / 4.5 / 5: the grant algebra №389–№393 (ADR-0155 — the capability
model), the consent runtime №397, the LikenessToken №387 (ADR-0149), the label
lattice + runtime labels №322–№328 (ADR-0154/0156), the backend ladder №336
(ADR-0165), MCP HTTP/SSE №394/№401 (ADR-0168), OCR №407 and video-understanding
№408 (per-shard HF pins), taint layer 2 №405 (ADR-0170), the architecture
contracts №382, the stable try-codes №385/№413 (ADR-0169), the Stage 5
evidence + two re-gates №404/№410 (ADR-0141 Addenda 3–5), the footprint
compression №409, SMFS №282, release 0.20.1 №411.

### 6.2. Re-verification of the section-2 "missing" list (the taint engine)

| Item (section 2) | Verdict on `b05d36c` | Proof command + verbatim output |
|---|---|---|
| 2.1 Label lattice — was NO | **YES** — a two-axis lattice (`Conf` × `Integrity`) with LUB/absorption `join` operators landed (№322/№328, ADR-0154/0156): `src/labels.rs` | `ls src/labels.rs && grep -c "pub enum\|pub struct" src/labels.rs` → `src/labels.rs` / `6` |
| 2.3 Join at merges — was NO | **YES (runtime labels)** — `join` is a lattice operator consumed by the runtime `LabelJoin` instruction on BOTH backends | `grep -n "pub fn join" src/labels.rs` → `src/labels.rs:89` / `src/labels.rs:151` |
| 2.2 Full inference over statement kinds | PARTIAL — the static interp engine covers the statement set it covers; `Match`/`Break`/`Continue` still outside `TAINT_INTERP` | `grep -c 'check_id: "' src/audit.rs` → `43` (was 21 on `1876fdf`) |
| 2.4 Effects module — was NO | **still NO** — no `EffectKind`/effects module; the effect trail `⟨io, audit⟩` is declarative (ADR-0154 §9) | `grep -rn "EffectKind\|effect_trace" src/ --include="*.rs"` → empty |
| 2.5 Exhaustive matching — was NO | PARTIAL — a central `match kind` exists in the deny path (`audit.rs:4357`); the transformation/absorption contract is the lattice's `join`, not yet a single exhaustive sweep of every check | `grep -n "match kind" src/audit.rs` → `4357:    match kind {` |
| 2.6 Affinity — was NO | **still NO** (only "sqlite affinity" column-affinity comments — not the affine-types concept) | `grep -rni affinity src/` → 2 comment hits in `vm.rs` (2249, 2371) |

New since the last check: **taint layer 2** — persistence-surface taint with
prefix bindings (№405, ADR-0170): `TAINT_PERSISTENCE`/`TAINT_PASSTHROUGH`
(`src/audit.rs:5,2140`).

### 6.3. The subsystem facts that moved the readiness

| Fact | Proof command + verbatim output |
|---|---|
| The capability model EXISTS (was 0%): the grant algebra with an opaque linear handle | `grep -n "pub struct GrantHandle" src/grants.rs` → `src/grants.rs:111: pub struct GrantHandle {`; `grep -c "GRANT_" src/grants.rs` → `19` |
| A unified compute-backend registry EXISTS (was content-registries only) | `grep -n "pub const BACKEND_REGISTRY" src/backends.rs` → `src/backends.rs:143` |
| The signed action ledger EXISTS (was consent-ledger only) | `ls src/ledger.rs` → `src/ledger.rs`; `grep -n "verify_file" src/ledger.rs` → `src/ledger.rs:691: pub fn verify_file(` |
| Linearity enforcement (static Once-linearity flow walk) | `grep -n "GRANT_CONSUMING_CALLS" src/audit.rs` → `src/audit.rs:5199` |
| The consent runtime is language surface (№335/№397) | `sed -n '1p' src/builtins/consent.rs` → `// ── Наряд №335 (spec §7.2 v2): consent grant/revoke + quarantine sink ──` |
| Memory: the plan-v2 contract is still unknown, `recall` is STILL a registry stub | `grep -n '"recall"' src/builtins/registry.rs` → `295:    spec!("recall", 0, "stub"),` |
| Width counters moved | VM instructions: `awk '/pub enum Instruction/,/^\}/' src/bytecode.rs \| grep -cE '^\s{4}[A-Z]'` → `54` (was 47); builtins → `460` (was 421); ADRs → `162` (was 145); top-level examples → `247` (was 214) |

### 6.4. Recomputed P0 readiness: **68%** (working figure after wave 5)

Same decomposition and weights as section 3 (still UNVERIFIED — plan v2 is
still absent from the repository); readiness per the code facts of 6.2–6.3.

| Subsystem | Weight (UNVERIFIED) | Readiness `1876fdf` | Readiness `b05d36c` | Contribution | Basis for the new readiness |
|---|---|---|---|---|---|
| Labels (taint) | 30% | 55% | **75%** | 22.5pp | the lattice + join landed (2.1/2.3 YES), taint layer 2 (№405), 43 check_ids, the blocking ones in Category-A; **missing**: effects module, affinity, full static-engine inference/join (2.2/2.4/2.5 PARTIAL), points-to/fixpoint (parked P2-1) |
| Capability model | 20% | 0% | **85%** | 17pp | the grant algebra end-to-end (opaque linear `Value::Grant`, scope/TTL/quota, subgrant attenuation, static linearity + runtime gates, ledger integration), consent runtime, LikenessToken; **missing**: grant-gating covers destructive SQL only — exec/network irreversible ops are deny-only, not every builtin's capability attribute is expressed through grants |
| Backend registry | 15% | 10% | **85%** | 12.75pp | unified `BACKEND_REGISTRY` + ladder + `Degraded(t)` (№336/ADR-0165), per-shard HF pins for OCR/video-understanding (№407/№408); **deduction**: real-weights execution is PARKED by hardware (№294) — the registry/pins/loader are real, the execution proof is not |
| Ledger | 15% | 35% | **90%** | 13.5pp | Action Ledger v1 — signed Ed25519 chain + `mlog ledger verify` (№393/ADR-0167), consent ledger, subprocess audit, deny events journal; **missing**: the runtime `ledger_verify()` hook (parked P2-2 — verification is CLI-only) |
| Memory | 20% | 13% | **13%** | 2.6pp | facts unchanged: real local infrastructure (memory_store/graph/embeddings, BM25+vector fusion), `recall` still a stub, the plan-v2 memory contract still unknown (plan absent) |
| **Total** | **100%** | **25.85 ≈ 26%** | — | **68.35 ≈ 68%** | |

**Honest reading.** The 26% → 68% move is real and code-proven (the three
zero/near-zero subsystems of the 2026-09-19 audit — capability, registry,
ledger — are now the best-covered ones), but it is NOT a P0-green claim:
the weights remain UNVERIFIED without plan v2, the memory subsystem did not
move, and the remaining taint gaps (effects, affinity, full static inference)
are exactly the parked P2 items. Recomputation at the next wave boundary,
same protocol.

### 6.5. Wave 9 recount (naryad №425): **76%** (main @ `5ed4ced6`, 2026-09-22)

Same decomposition and weights as §6.4 (still UNVERIFIED — plan v2 is still
absent from the repository); readiness per the code facts only, the №414
protocol (the same UNVERIFIED weights, no P0-green claims without a contract).

| Subsystem | Weight (UNVERIFIED) | №414 `b05d36c` | Wave 9 `5ed4ced6` | Contribution | Basis for the new readiness |
|---|---|---|---|---|---|
| Labels (taint) | 30% | 75% | **75%** (unchanged) | 22.5pp | no taint work in Wave 9 — the parked P2 set (effects, affinity, full static inference) is unchanged |
| Capability model | 20% | 85% | **88%** | 17.6pp | grant-gating extended BEYOND destructive SQL: the memory cascade forget is a granted, previewable, ledgered delete capability (№351/ADR-0173 §3.4 — check_active → scope `memory:forget:<container>` → the retained VETO → apply → grant_use → the post-success `irreversible.memory_forget` record); proof: `grep -n "pub fn forget_cascade" src/memory_typed.rs`; **still missing**: exec/network irreversible ops remain deny-only, not every builtin carries a capability attribute |
| Backend registry | 15% | 85% | **85%** (unchanged) | 12.75pp | no compute-registry work in Wave 9 (the tick context, №426/ADR-0175, is serve infrastructure — it un-blocks the office dogfood path but does not move the registry row) |
| Ledger | 15% | 90% | **95%** | 14.25pp | the parked P2-2 residual CLOSED: the runtime `ledger_verify()` hook (№415, 0.21.0 — `ledger_verify(source, expect_head, expect_key) -> LedgerVerdict` + `mlog ledger verify --json`); proof: `grep -n "pub fn ledger_verify" src/ledger.rs`; the record surface extended (`memory.*`, `duplex.*`, `session.*` families, №348/№350/№351/№352); **missing**: nothing structural — the out-of-band anchor discipline stays user-side (ADR-0167 §7) |
| Memory | 20% | 13% | **45%** | 9pp | the FIRST move of the subsystem: the Phase-8 "real DB" debt CLOSED (№351/ADR-0173 §3.5 — the persistent rusqlite store behind `METALOGOS_MEMORY_DB`, additive-only DDL per ADR-0060, a TRUE 3-process restart test); typed `Memory<K>` with consent-gated private storage and AES-256-GCM at-rest (№350); the derived-from graph + cascading forgetting with the retained VETO (ADR-0173 §3.3, fuzz-pinned P1/P2/P3); the session model (№348/ADR-0172) and the duplex barge-in (№352/ADR-0174) as the actor surfaces; proof: `grep -n "METALOGOS_MEMORY_DB" src/memory_typed.rs`, `ls src/session.rs src/duplex.rs`; **still missing**: `recall` remains a registry stub (`grep -n '"recall"' src/builtins/registry.rs`), the plan-v2 memory contract is still unknown (plan absent), the FTS5-recall lane is not integrated with the typed lane, decay/boost are legacy-lane only |
| **Total** | **100%** | **68.35 ≈ 68%** | — | **76.1 ≈ 76%** | |

**Honest reading.** The 68% → 76% move is real and code-proven — the memory
subsystem moved for the first time in four recount rounds (13% → 45%: the
persistence, the typed layer, the grant-gated forgetting are landings, not
promises), and the ledger's last structural gap (the runtime verification
hook) closed. It is still NOT a P0-green claim: the weights remain UNVERIFIED
without plan v2; `recall` is still a stub; the taint gaps (effects, affinity,
full static inference) are exactly the parked P2 set; the capability model's
remaining holes (exec/network deny-only, per-builtin capability attributes)
are named, not hand-waved. Recomputation at the next wave boundary, same
protocol.

### 6.6. Wave 10 recount (naryad №434): **76%** (main @ `b18fb254`, 2026-09-23)

Same decomposition and weights as §6.4/§6.5 (still UNVERIFIED — plan v2 is
still absent from the repository); readiness per the code facts only, no
P0-green claims without a contract. The honest headline: **the total does
not move** — Wave 10's substance is (a) the START of Phase 5 «Embodied,
sim-only» (№354/№355), which has NO row in this decomposition at all, and
(b) enforcement hardening + integration contracts on surfaces that were
already counted.

| Subsystem | Weight (UNVERIFIED) | Wave 9 `5ed4ced6` | Wave 10 `b18fb254` | Contribution | Basis for the readiness (recounted by proof commands) |
|---|---|---|---|---|---|
| Labels (taint) | 30% | 75% | **75%** (unchanged) | 22.5pp | the audio consent gate (№428) is ENFORCEMENT on a surface, not lattice work: the runtime `consent::active_grant_for` check + the ledgered refusal + the try-stamp; proof: `grep -n "fn require_audio_consent" src/duplex.rs`, `grep -n "CODE_AUDIO_CONSENT_REQUIRED" src/interpreter/values.rs`; the static contour still does not model `audio.speak`/`audio.listen` grants (limitations.md, the №428 row); the parked P2 set (effects, affinity, full static inference) unchanged |
| Capability model | 20% | 88% | **88%** (unchanged) | 17.6pp | the duty/session origin stamps (№430) harden the session contract's observability (position-0 `SESSION_UNKNOWN`/`SESSION_CONTRACT`, ADR-0172 Implemented); proof: `grep -n "CODE_SESSION_CONTRACT" src/interpreter/values.rs`; no NEW grant-gated capability landed — exec/network irreversible ops remain deny-only |
| Backend registry | 15% | 85% | **85%** (unchanged) | 12.75pp | the `embodied-sim` class + two in-tree sim records (№355) are registry GROWTH, not compute-backend readiness (no weights, nothing to pin — the honest `PendingNo334`); proof: `grep -n "EmbodiedSim" src/backends.rs`; the ladder class word `embodied-sim` parses; the compute rows (STT/omni/vision/video/OCR) unchanged |
| Ledger | 15% | 95% | **95%** (unchanged) | 14.25pp | the `embodied.*` record family joins `memory.*`/`duplex.*`/`session.*` (№355 — device_open/bounds_attach/world_state/chunk_make|denied/proof_seal/proof_verify/world_state_denied); proof: `grep -rn "embodied\." src/embodied.rs | head`; the verification hook (№415) unchanged — the out-of-band anchor discipline stays user-side (ADR-0167 §7) |
| Memory | 20% | 45% | **45%** (unchanged) | 9pp | the office-path integration contract (№429) PINS the end-to-end scenario (session_login → consent-granted private memory → provenance put → audited read → cascade preview → the Once-grant spend) — `tests/naryad_429_memory_office_path.rs`; the duty stamps (№430) harden the session surface; the "still missing" list is INTACT: `recall` remains a registry stub (`grep -n '"recall"' src/builtins/registry.rs`), the plan-v2 memory contract is still unknown, the FTS5-recall lane is not integrated with the typed lane |
| **Total** | **100%** | **76.1 ≈ 76%** | — | **76.1 ≈ 76%** | |

**The Phase-5 note (out of the decomposition).** Wave 10 lands the START of
registry §16.7 (В5): ADR-0159 (№354 — SafetyBounds as an STL-formula
monitor, carrier-independent semantics) and the embodied type surfaces
(№355 — seven opaque handles, the no-unmonitored-action refusal
`EMBODIED_UNBOUNDED`, the signed Pending-by-construction Proof, the
WorldState private-materialization refusal, the `embodied-sim` registry
profile). The decomposition above has NO embodied row — plan v2 §2 is
absent and the row set was reconstructed for the Phase-1..4 subsystems;
inventing a weight now would be a fabrication. The contour is recorded,
not scored; the monitor/stages are behind the GPU-budget gate (№356–№361,
owner decision) and will enter a recount only when the row set has a
plan-v2 basis.

**Honest reading.** 76% → 76% is the honest outcome: a wave whose code is
(a) a new contour's TYPES (scored nowhere without plan v2), (b) gates and
stamps on already-counted surfaces, and (c) an integration CONTRACT, does
not shorten any "still missing" list — and readiness here tracks exactly
that. The wave's real deliverable for the office path is the №429 contract
+ the №430 stamps (the dogfood №395 loop closes cleanly end to end), and
for Phase 5 — the carrier-independent semantics the gated stages will
reuse. P0 items unchanged: `recall` stub, plan v2 absent, taint P2 set.
Recomputation at the next wave boundary, same protocol.

### 6.7. Wave 11 recount (naryad №438): **76%** (main @ `d2295ce`, 2026-09-23)

Same decomposition and weights as §6.4–§6.6 (still UNVERIFIED — plan v2 is
still absent from the repository); readiness per the code facts only. The
honest headline: **the total does not move** — Wave 11's substance is (a)
office reliability INFRASTRUCTURE (№435: the waker moved into the public
repo, GH-hosted schedule; the paired FO-051 demoted the dead self-hosted
schedule in the office repo), which has NO row in this decomposition at
all, and (b) EVIDENCE hardening on already-counted surfaces (№436/№437:
the red/green example line + the mutation harnesses for the №354/№355
embodied refusals and the №428 audio consent gate — the audit 2026-09-23
P1-1/P1-2 live remains). The builtin registry does not grow: 494 before,
494 after.

| Subsystem | Weight (UNVERIFIED) | Wave 10 `b18fb254` | Wave 11 `d2295ce` | Contribution | Basis for the readiness (recounted by proof commands) |
|---|---|---|---|---|---|
| Labels (taint) | 30% | 75% | **75%** (unchanged) | 22.5pp | the dogfood examples (№436/№437) OBSERVE the runtime gates, the static contour is untouched: `audio.speak`/`audio.listen` grants and the WorldState opacity remain runtime facts (limitations.md L103/L104 unchanged); the parked P2 set (effects, affinity, full static inference) unchanged |
| Capability model | 20% | 88% | **88%** (unchanged) | 17.6pp | no NEW grant-gated capability landed: the waker (№435) is repo infrastructure (a workflow pinging `/health`), not a language capability; the consent dogfood (№437) exercises the EXISTING №335 grant contour (`consent_grant`/`consent_revoke`); proof: `grep -c 'spec!(' src/builtins/registry.rs` = 494, unchanged |
| Backend registry | 15% | 85% | **85%** (unchanged) | 12.75pp | no compute-backend work: the wave adds examples/tests/workflow files only; the `embodied-sim` registry records (№355) unchanged; the TimesFM/forecast ladder (№440) is Wave-12 BACKLOG, not landed |
| Ledger | 15% | 95% | **95%** (unchanged) | 14.25pp | no new record family: the №437 example OBSERVES the existing `duplex.*` records (`duplex.speak_denied`/`duplex.listen_denied`/`duplex.speak_start`/`duplex.stop`) via `ledger_count` deltas; proof: `grep -rn "duplex\." src/duplex.rs | head`; the verification hook (№415) unchanged |
| Memory | 20% | 45% | **45%** (unchanged) | 9pp | no memory work in Wave 11; the "still missing" list is INTACT: `recall` remains a registry stub (`grep -n '"recall"' src/builtins/registry.rs`), the plan-v2 memory contract is still unknown, the FTS5-recall lane is not integrated with the typed lane |
| **Total** | **100%** | **76.1 ≈ 76%** | — | **76.1 ≈ 76%** | |

**Honest reading.** 76% → 76% is the honest outcome: a wave whose code is
(a) repo/office infrastructure (the office's native cron got its alarm
clock back — the waker now runs where the runners are alive), (b) example
and mutation EVIDENCE for refusals that were already counted with their
surfaces, and (c) a doc sync, does not shorten any "still missing" list —
and readiness here tracks exactly that. The wave's real deliverable is
operational: the audit 2026-09-23 P1 items are closed by evidence (P1-1 →
№436, P1-2 → №437), and the office voice-path adaptation contract is
written down where the office will read it — in the example header.
P0 items unchanged: `recall` stub, plan v2 absent, taint P2 set.
Recomputation at the next wave boundary, same protocol.

### 6.8. Waves 12–13 recount (naryad №443): **80%** (main @ `8c6256fa`, 2026-09-24)

Same decomposition and weights as §6.4–§6.7 (still UNVERIFIED — plan v2 is
still absent from the repository); readiness per the code facts only, the
№414 protocol (the same UNVERIFIED weights, no P0-green claims without a
contract). The honest headline: **the total moves 76% → 80%** — Wave 12
landed the first compute-backend ladder class with real in-tree execution
(№440/№441), and Wave 13's №442 turned `recall` into the real front door of
memory, closing two of the four named gaps of the Memory row. The builtin
registry grows 494 → 499; the static contour (unique audit check_ids) is
unchanged at 33.

| Subsystem | Weight (UNVERIFIED) | Wave 11 `d2295ce` | Waves 12–13 `8c6256fa` | Contribution | Basis for the readiness (recounted by proof commands) |
|---|---|---|---|---|---|
| Labels (taint) | 30% | 75% | **75%** (unchanged) | 22.5pp | the new gates are RUNTIME origin-stamps on new surfaces, not lattice/static work: `FORECAST_TAINTED` (№440, the forecast export sink-gate) and `MEMORY_RECALL_CONSENT_REQUIRED` (№442, the recall consent refusal); proof: `grep -n "CODE_FORECAST_TAINTED\|CODE_MEMORY_RECALL_CONSENT_REQUIRED" src/interpreter/values.rs` → `values.rs:555` / `values.rs:563`; the static contour is unchanged — the unique `check_id` count is 33 on both `d2295ce` and `8c6256fa` (sorted-unique diff: empty); the parked P2 set (effects, affinity, full static inference, fixpoint) unchanged |
| Capability model | 20% | 88% | **88%** (unchanged) | 17.6pp | no NEW grant-gated capability class: the №442 recall gate rides the EXISTING №413 fail-closed convention (the runtime consent check with scope `memory:<container>`); the №440 forecast gate is taint-based, not grant-based; proof: `grep -c 'spec!(' src/builtins/registry.rs` → `499` (registry growth, not capability growth); **still missing**: exec/network irreversible ops remain deny-only, not every builtin carries a capability attribute |
| Backend registry | 15% | 85% | **88%** | 13.2pp | the FIRST ladder class with real in-tree execution: the `timeseries` class (№440, `grep -n "timeseries" src/backends.rs` → `backends.rs:80/97/305`) with the ladder `timesfm-2.5 → statsforecast → seasonal_naive` (proof: `grep -n "TIMESERIES_LADDER" src/forecast.rs` → `forecast.rs:56`): `seasonal_naive` is the built-in deterministic leg (`forecast.rs:461`), `statsforecast` the pure-software Apache-2.0 rung (backends.rs:323 — no weights artifact exists, nothing to fetch or pin), timesfm-2.5 the Apache-2.0 weights pin (3.0 pinned-never); the ladder EXECUTES in-tree (the w12 example's real quantiles at the seasonal_naive rung, CI-proven); №442 wires hybrid retrieval engines (FTS5 BM25 + cosine RRF on SqliteStore) as real software backends feeding the typed lane; **deduction**: the TOP rungs' real-weights execution remains hardware-gated (P2-2 unchanged) |
| Ledger | 15% | 95% | **95%** (unchanged) | 14.25pp | two new record FAMILIES — `forecast.*` (№440: series_make/run/denied/pull_denied, hash-bearing details; proof: `grep -n "forecast\.run\|forecast\.denied" src/forecast.rs` → `forecast.rs:36-43,715`) and `memory.recall`/`memory.recall.denied` (№442; proof: `grep -n 'crate::ledger::record("memory.recall"' src/memory_typed.rs` → `memory_typed.rs:1456/1470`) — record-surface growth, not structural (the runtime verify hook №415 closed the last structural gap in §6.5); the verification hook unchanged — the out-of-band anchor discipline stays user-side (ADR-0167 §7) |
| Memory | 20% | 45% | **60%** | 12pp | two of the four named gaps CLOSED: (1) `recall` is the real front door — proof: `grep -n 'spec!("recall"' src/builtins/registry.rs` → `registry.rs:320: spec!("recall", 1, 2, "memory"; builtin_recall)` (the stub spec is gone; zero stub-spec on the name); the consent gate is fail-closed (№413: gated content never read — key-list metadata only; the refusal IS a `memory.recall.denied` record); typed hits carry the `[MEM]` provenance suffix; every call records `memory.recall {query hash, containers, hits, consent fact}`; (2) the FTS5-recall lane is INTEGRATED with the typed lane — hybrid FTS5 BM25 + cosine RRF on SqliteStore, the all-entries scan on InMemoryStore, the activation semantics preserved (sim × priority × decay), the external store contract regression-pinned (m4 golden, DoD, crosscheck); **still missing**: decay/boost remain legacy-lane only (`grep -n "decay" src/memory_typed.rs` → empty; the Wave-14 candidate per the dispatch), the plan-v2 memory contract is still unknown (plan absent) |
| **Total** | **100%** | **76.1 ≈ 76%** | — | **79.55 ≈ 80%** |

**Honest reading.** The 76% → 80% move is real and code-proven — the Memory
subsystem moves for the first time since wave 9 (45% → 60%: the front door
of memory is a real, consent-gated, ledgered, provenance-bearing handler and
the store lane's hybrid engines are wired into the typed lane — landings,
not promises), and the backend registry's ladder class executes in-tree for
the first time (the deterministic and pure-software rungs run in CI; the
neural rung stays behind its pin). It is still NOT a P0-green claim: the
weights remain UNVERIFIED without plan v2; the taint P2 set (effects,
affinity, full static inference, fixpoint) is unchanged; decay/boost on the
typed lane remain open (the Wave-14 candidate per the dispatch); the
plan-v2 memory contract is still unknown; the top ladder rungs' real-weights
execution stays hardware-gated (P2-2). Recomputation at the next wave
boundary, same protocol.

### 6.9. Wave 14 recount, part 1 (naryad №446): **82%** (main @ `3dae2e51`, 2026-09-24)

Same decomposition and weights as §6.4–§6.8 (still UNVERIFIED — plan v2 is
still absent from the repository); readiness per the code facts only, the
№414 protocol (the same UNVERIFIED weights, no P0-green claims without a
contract). The honest headline: **the total moves 80% → 82%** — №445 closed
the THIRD of the four named gaps of the Memory row (decay/boost are no
longer legacy-lane only; the second front door — `forget` — is a real
handler; the canon `retain(memory, ttl)` exists; the auto-forgetting sweep
lifted the №280 "v2" deferral). The builtin registry grows 499 → 500; the
stub-spec rows shrink 24 → 23 (`forget` left the stub set); the static
check_id vocabulary grows 33 → 35.

| Subsystem | Weight (UNVERIFIED) | Waves 12–13 `8c6256fa` | Wave 14 `3dae2e51` | Contribution | Basis for the readiness (recounted by proof commands) |
|---|---|---|---|---|---|
| Labels (taint) | 30% | 75% | **75%** (unchanged) | 22.5pp | the new gates are RUNTIME origin-stamps on the forgetting surface, not lattice/static work: `MEMORY_FORGET_CONSENT_REQUIRED` (№445, the forget consent refusal) and `MEMORY_POISONED` (№445, the quarantine sink-gate); proof: `grep -n "CODE_MEMORY_FORGET_CONSENT_REQUIRED\|CODE_MEMORY_POISONED" src/interpreter/values.rs` → `values.rs:568/573` (both whitelisted for `try`, values.rs:624-625); the static vocabulary grows 33 → 35 with two SURFACE companions (argument-literal validation, the №442 RECALL template) — `FORGET_DRYRUN_INVALID` + `RETAIN_TTL_INVALID` (`grep -n "FORGET_DRYRUN_INVALID\|RETAIN_TTL_INVALID" src/audit.rs` → the `check_forget_surface` mappings) — companion growth, not inference work; the parked P2 set (effects, affinity, full static inference, fixpoint) unchanged |
| Capability model | 20% | 88% | **88%** (unchanged) | 17.6pp | no NEW grant-gated capability class: the forget front door rides the EXISTING №351 class (the scope `memory:forget:<container>` since ADR-0173 §3.4; the enforcement order adds the consent rung and the dry_run rung to the SAME ladder — check_active → scope → plan → retained-VETO → apply → grant_use, the grant consumed on SUCCESS only); proof: `grep -c 'spec!(' src/builtins/registry.rs` → `500` (registry growth, not capability growth), `grep -n "pub fn forget_front" src/memory_typed.rs` → the shared engine; **still missing**: exec/network irreversible ops remain deny-only, not every builtin carries a capability attribute |
| Backend registry | 15% | 88% | **88%** (unchanged) | 13.2pp | no backend-registry work in №445 (the forgetting memory touches no compute ladders); the `timeseries` class and the hybrid retrieval engines are unchanged from §6.8 |
| Ledger | 15% | 95% | **95%** (unchanged) | 14.25pp | record-surface growth, not structural: the `memory.forget` / `memory.forget.denied` family (№445; proof: `grep -n 'crate::ledger::record("memory.forget' src/memory_typed.rs` → `memory_typed.rs:1901/1924`), the ttl/retain family in the `memory.*` convention (`memory.retain_ttl`, `memory.ttl_expired` — the `ledger_memory_event` formatter, memory_typed.rs:283), and the `irreversible.memory_forget` record reused on the front door (memory_typed.rs:2092); the denied-recall symmetry holds — a refused forget leaves the same audit trail a granted one does (`memory.forget.denied` at every refusal rung) |
| Memory | 20% | 60% | **70%** | 14pp | the THIRD of the four named gaps CLOSED: (3) decay/boost are in the typed lane — proof: `grep -c "decay" src/memory_typed.rs` → `37` (the №443 proof command `grep -n "decay" src/memory_typed.rs` → empty is now STALE — the exact anchor of the gap); the ACT-R activation product (base × priority × exp(−decay_rate × full days stale); day-granularity keeps the №442 scores byte-exact) orders the typed recall; every access boosts (last_access refresh, persisted); the SECOND front door: `forget(handle, key, grant, dry_run?)` is a real handler — proof: `grep -n 'spec!("forget"' src/builtins/registry.rs` → `registry.rs:377: spec!("forget", 3, 4, "memory"; builtin_forget)` (the №443 stub anchor `registry.rs:366: spec!("forget", 0, "stub")` is gone; zero stub-spec on the name); the canon retain(memory, ttl): `grep -n 'spec!("memory_retain_ttl"' src/builtins/registry.rs` → `registry.rs:1011` (appended at the end — the .mbc index contract; registry 499 → 500; the stub-spec set 24 → 23); the auto-forgetting sweep lifted the №280 "v2" deferral (expired entries auto-forgotten on the read/keys/recall paths, `memory.ttl_expired` records); the §10.3 quarantine: the forget cascade poisons the derived closure and CLOSES the sinks (MEMORY_POISONED on read/export, the recall lane skips the quarantine; consent revocation fires the same cascade — the №442 fail-closed gate evidence stays intact, a re-grant never resurrects); the activation/quarantine attributes persist (the additive side table — `grep -n "memtyped_entry_attrs" src/memory_typed.rs`; no ALTER, ADR-0060); **still missing**: the plan-v2 memory contract is still unknown (plan absent) — the ONLY remaining named gap of the row, a decision/SSOT gap, not a code gap |
| **Total** | **100%** | **79.55 ≈ 80%** | — | **81.55 ≈ 82%** |

**Honest reading.** The 80% → 82% move is real and code-proven — the Memory
row's CODE-side named gaps are now exhausted (three of four closed; the
fourth — the plan-v2 memory contract — is a decision gap the code cannot
close: the plan is absent from the repository and the weights stay
UNVERIFIED until it lands). The forgetting memory is a landing, not a
promise: the grant ladder, the poison quarantine, the ttl sweep and the
activation ranking are CI-pinned by 13 contract tests and mutation-verified
(≥2/2, protocol №382). It is still NOT a P0-green claim: the taint P2 set
(effects, affinity, full static inference, fixpoint) is unchanged; the
capability model's exec/network deny-only holes are unchanged; the top
ladder rungs' real-weights execution stays hardware-gated (P2-2); find and
inspect remain stubs (the registry's remaining 23 stub rows are named,
out-of-scope surfaces). Recomputation at the next wave boundary, same
protocol.

### 6.10. Wave 15 recount, part 1 (naryad №450): **85%** (main @ `2b062513`, 2026-09-25)

Same decomposition and weights as §6.4–§6.9 (still UNVERIFIED — plan v2 is
still absent from the repository); readiness per the code facts only, the
№414 protocol. The honest headline: **the total moves 82% → 85%** — the two
Wave-15 code naryads closed TWO of the four named gaps of the Labels row.
№448 closed "full static-engine inference" (REALITY §2.2: the interp contour
now covers ALL 15 statement kinds — `Match`/`Break`/`Continue` + the Memory
variants `Memorize`/`Forget`/`Relate`; the merge points use the lattice
join). №449 closed "effects" (REALITY §2.4: the effect attributes
{read, write, network, irreversible} derived from the №316 SSOT, the effect
traces in the audit output, the network-axis escalation including the №316
DUAL prompt-egress of `call_llm`).

Arithmetic (the same granularity the Memory row of §6.9 used): the Labels
row stood at 75% with 25pp missing across FOUR named gaps (the №446 row:
effects, affinity, full static inference, fixpoint) ≈ 6.25pp each; two gaps
closed → +12.5pp; the exact 87.5 rounds DOWN → **87%** for the row
(30% weight): 26.1pp (was 22.5pp) → the total 81.55 + 3.6 = 85.15 ≈ **85%**
(rounded down; no upward pressure).

| Subsystem | Weight (UNVERIFIED) | Wave 14 `3dae2e51` | Wave 15 `2b062513` | Contribution | Basis for the readiness (recounted by proof commands) |
|---|---|---|---|---|---|
| Labels (taint) | 30% | 75% | **87%** | 26.1pp | the two named gaps closed with proof: §2.2 — `sed -n '/── Наряд №292 (P0, security): TAINT_INTERP/,/CANARY_LEAK/p' src/audit.rs | grep -c "Statement::Match\\|Statement::Break\\|Statement::Continue\\|Statement::Memorize\\|Statement::Forget\\|Statement::Relate"` → **10** (the proof was 0 on every prior recount — the flat collector fell into the `_ => {}` arm); the `return`-inside-match-arm summary union and the state-aware walker replace it; the merge points join through the lattice — `grep -c "state.join_into" src/audit.rs` → **10** merge sites (`Label::join`, the static twin of the runtime 2.3 operator — no new lattice); §2.4 — `grep -n "pub fn builtin_effects\|pub struct EffectSet" src/audit.rs` → the effects section (`git show 3dae2e51:src/audit.rs | grep -c effect` → **0** — the gap was real); the SSOT coverage: `tests/naryad_449_effects.rs::a1_every_registered_builtin_has_effect_output` (source-level `spec!` parse, feature-independent — a registry builtin without a №316 row fails CI); the leak corpus grew 40 → 44 negatives / 22 → 26 positives — every new negative compiled clean before its naryad (probes on `5df35ff9`: the break-carry, the write_file-chain-in-arm, the key-less statement-form memorize; on `c7ffa112`: the prompt-egress chain); mutation contracts VERIFIED — 4/4 (№448: the walker arm-merge, the summary union, the break/continue exit collection) + 3/3 (№449: the irreversible SSOT mapping, the network escalation × unit + corpus); **still missing**: affinity (§2.6 — the canon risk registry requires a preliminary ADR decision), exhaustive matching as the central contract (§2.5 — PARTIAL: the deny-path `match kind` exists at `audit.rs:4357`-era shape, no exhaustive sweep), points-to/fixpoint (P2-1 park) |
| Capability model | 20% | 88% | **88%** (unchanged) | 17.6pp | no grant work in №448/№449 — the static layer only: `grep -c 'spec!("' src/builtins/registry.rs` → `500` (no registry growth, no new capability attribute); **still missing**: exec/network irreversible ops remain deny-only, not every builtin carries a capability attribute |
| Backend registry | 15% | 88% | **88%** (unchanged) | 13.2pp | no backend-registry work in №448/№449 (the audit effects touch no compute ladders); the `timeseries` class and the hybrid retrieval engines are unchanged from §6.8 |
| Ledger | 15% | 95% | **95%** (unchanged) | 14.25pp | no ledger work: the №449 effect traces are AUDIT-OUTPUT only — the ledger is not duplicated (the №449 boundary: статика↔рантайм связь documented in limitations.md, the ADR-0167 append-only discipline untouched) |
| Memory | 20% | 70% | **70%** (unchanged) | 14pp | no memory work in №448/№449: the Memory statement variants participate in the STATIC inference (the interp contour) — zero runtime change; proof: the runtime gates are byte-identical — `grep -n "CODE_MEMORY_RECALL_CONSENT_REQUIRED\\|CODE_MEMORY_FORGET_CONSENT_REQUIRED" src/interpreter/values.rs` → `values.rs:563/568` (the same anchors the §6.9 row cites); the plan-v2 memory contract is still unknown (plan absent) — the decision/SSOT gap stays |
| **Total** | **100%** | **81.55 ≈ 82%** | — | **85.15 ≈ 85%** |

**Honest reading.** The 82% → 85% move is real and code-proven: the Labels
row's named-gap set shrinks from four to two (affinity + points-to/fixpoint,
both parked behind explicit gates — an ADR decision and the P2-1 park; the
§2.5 exhaustive-sweep residual is named inside the row). The static contour
now sees every statement kind the language can express (15/15) and attaches
effect semantics to every classified builtin — the corpus grew with
negatives that compiled clean for weeks (the break-carry, the arm-hidden
write_file chain, the key-less memory write, the prompt-egress chain). It is
still NOT a P0-green claim: the capability exec/network deny-only holes are
unchanged; the plan-v2 memory contract is still a decision gap; the weights
stay UNVERIFIED until the plan lands; the top ladder rungs stay
hardware-gated; the registry's 23 stub rows remain named out-of-scope
surfaces. Recomputation at the next wave boundary, same protocol.

### 6.11. Wave 17 recount (naryad №473): **85%** (main @ `7411996`, 2026-09-26)

Protocol: №414/№318 — the same UNVERIFIED plan-v2 weights, no P0-green
claims without a proof pass. The honest headline: **the total stays 85% —
and that is the honest result of a strategic wave.** Wave 17 executed the
owner's seven strategic decisions (gate gh#680): the freeze, the CI gates,
the type-system stage 0, the media isolation, the process machinery. These
are infrastructure/process/typing-metadata moves — the four weighted
functional rows (the compute ladders, the taint coverage, the ledger, the
memory contract) saw no code changes to credit or debit. Recounting them
"up" would be inflation; recounting them "down" would be false — the
`--no-default-features` core build and the core→media isolation are new
verifiable facts, but they sit OUTSIDE the P0 functional weights (they
harden the shipability, not the feature readiness).

**The new wave counters (the checked-in facts, each reproducible):**

| Counter | Value | Command / source | Movement rule |
|---|---|---|---|
| TW/VM duplicated builtin names (№462) | **35** (baseline was 60) | `python3 scripts/ci/count_duplicated_names.py --gate scripts/ci/tw_vm_dup_names_baseline.txt` | only down; the №466 groups 1–4 moved it 60→56→49→42→35; the media/vision cluster (20 names) leaves with the 0.27 split (№472 roadmap), 15 other-cluster names wait for their groups — the transfer naryad №466 stays open into В18 |
| Typed-signature share (№467) | **49/500 = 9.80%** (the stage-0 start) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | only up |
| `#[ignore]` debt (№468) | **85** (of which with TODO: 49) | `python3 scripts/ci/debt_counters.py --list ignore` | only down |
| `dead_code` (№468) | **37** | `python3 scripts/ci/debt_counters.py --list dead_code` | only down |

**The Wave 17 gate inventory (all in main, all blocking unless stated):**

| Gate | Naryad | Status |
|---|---|---|
| The domain freeze (ADR-0177) — no new subsystems/domains until 0.27; 0.27.0 is the release of freeze-lifting and is gated on the ADR's criteria | №461 | **in force**; the 0.27.0 tag is NOT published |
| The generative stop-list + LOC growth gate | №463 | blocking, green |
| The TW/VM dup-names threshold | №462 | blocking, green (35/35) |
| The typed-signature share floor | №467 | blocking, green (980 bp) |
| The debt gate (ignore/dead_code) | №468 | blocking, green (85/49/37) |
| The physical core→media ban (the handle/registry tier only) | №472 | blocking, green (33/33 references allowed) |
| The no-default-features core build | №472 | blocking, green |
| The naryad-class domain-quota counter | №470 | the dispatcher's composition tool; the W17 run: **PASS — 0 domain naryads of 14** |
| The risk-based review surface | №469 | **advisory** (non-blocking) until a separate owner decision |

The CI blocking-jobs count: **27** (the badge corrected in №472 — the
waves-13–17 gates had not been reflected).

**The release state:** 0.26.0 was published in Wave 16 (№460, tag
`v0.26.0` on `01bec90`); 0.26.1 closes Wave 17 (this naryad — docs and
the version bump only, per the naryad boundary). **0.27.0 is the release
of freeze-lifting — the tag is NOT published until the ADR-0177 unfreeze
criteria are green** (the release gate of №461).

### 6.12. Wave 18 recount (naryad №476): **85%** (main @ `cd6a14e`, 2026-09-27)

Protocol: №414/№318 — the same UNVERIFIED plan-v2 weights, no P0-green
claims without a proof pass. The honest headline: **the total stays 85%**
— Wave 18 was the unfreeze path (the ADR-0177 §4 criteria finish), it
moved the gates and the core infrastructure, not the functional weights.

**The Wave 18 deliverables (all merged, all blocking-CI green):**

- **№466 completed** (gh#687; the groups 5–7 of the dispatch #745 map):
  the audit-ledger five (`deny_event`/`deny_reason`/`event_count`/
  `event_sum`/`events_since` → `src/audit_ops.rs`, PR #746), the recipe
  pair (`recipe_save`/`recipe_search` → `src/recipe_ops.rs`, PR #747),
  and the server/runtime eight (`exec`/`find`/`fit_to_budget`/`inspect`/
  `json_body`/`require`/`resolve_skill_index`/`server_path_param` →
  `src/runtime_ops.rs`, PR #748) left both backends into the shared live
  modules. The per-backend divergences are preserved verbatim (the deny
  accessor, the recipe search lanes, the find stores, the skill-index
  shapes); every PR carried the №465 diff-fuzzer before/after proof.
- **№474 executed** (gh#742; PR #749): the enum Type **stage 1** — the
  let-type inference, WARN-ONLY (the №467 canon). The one warn rule (the
  type conflict on a `let mut` reassignment) lands in
  `AnalysisResult.warnings` with the `[stage1 types]` prefix; the errors
  are structurally untouched; the C4 acyclicity inventory moved
  deliberately with the module (SCC-2 documented in the same PR).
- **№475 executed** (gh#743): the FO-056 memory-office E2E dogfood
  evidence refreshed on 0.26.1 (16/16 + pytest 3/3 + the 32-record
  ledger) — recorded in the gate thread gh#680 (`n475: fo056-evidence`).

**The Wave 18 counters (the same commands, the same movement rules):**

| Counter | Value | Movement |
|---|---|---|
| TW/VM duplicated builtin names (№462) | **20** (35 after В17; 60 at the start) | only down: 60→56→49→42→35→30→28→20; **the remaining 20 are exactly the media/vision cluster** — it leaves with the 0.27 crate split (№472 roadmap) |
| Typed-signature share (№467) | **49/500 = 9.80%** (unchanged — stage 1 adds the CHECKS, not the typed rows) | only up; the share moves when later waves type more registry rows |
| `#[ignore]` debt (№468) | **85** (TODO: 49) | only down (unchanged — the wave added no debt) |
| `dead_code` (№468) | **37** | only down (unchanged) |

**The ADR-0177 §4 unfreeze criteria — the state after Wave 18 (the gate
reads evidence, not intentions):**

| Criterion | State on `cd6a14e` | Verdict |
|---|---|---|
| §4.1 Types: enum Type stages 0 AND 1 | stage 0 (№467, `df7dde3`), stage 1 (№474, `cd6a14e`); the CI share metric green (980 bp floor) | **GREEN** |
| §4.2 Dedup: count ≤ threshold, only down | 20/20, the history 60→…→20 is one-way | **GREEN** (the media/vision residue rides the 0.27 split — the recorded boundary) |
| §4.3 Debt: the CI debt gate | 85/49/37 at the thresholds, exit 0 | **GREEN** |
| §4.4 Memory: the office E2E dogfood | FO-056 on 0.26.1: 16/16 + 3/3 (gh#680, `n475: fo056-evidence`) | **GREEN** |

**The lift decision is the OWNER'S alone** (ADR-0177 §4: "the right to
lift the freeze belongs to the OWNER ONLY"). Every criterion now carries
its evidence; the lift is recorded by the owner in gh#680 and the ADR's
Status is updated by a naryad, not silently. **0.27.0 remains
unpublished** until that explicit lift.

**The release state:** 0.26.2 closes Wave 18 (this naryad — docs, the
version bump and the release only, per the naryad boundary). The
non-blocking `coverage` job flagged the new `semantic_types.rs` lines on
PR #749 (the inference paths are covered by the dedicated test file; the
coverage job is advisory — recorded per the conveyor discipline).

### 6.13. Wave 27 recount (naryad №576): the counters (main @ `1c79fc2`, 2026-10-04)

Protocol: №414/№318 — the same machines, no hand-written numbers. The
honest headline: this is a COUNTER recount, not a P0-readiness protocol
recount — the subsystem weights of section 3 stay as of the №476
recount (§6.12); Wave 27 was the release-pipeline repair + the typing
step + the two ledger lanes (the doc facts below), no functional-weight
claim is made by a docs naryad. The wave-fact documentation lives in
the public digest (docs/PLAN-SUMMARY.md — the wave rows 25–27, the
current-wave section reads Wave 27); this section pins the machine
counters the waves 19–27 moved.

**The Wave 27 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **186/509 = 3654 bp** (the №573 diagram package: 20 rows `"String"` + the tokens completion `"Struct"`; THE 0.29 DRAFT GOAL 3500 bp REACHED — ADR-0181, the gate still blocks nothing until wired into blocking CI) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | only up: 998→1222→1420→2110→2702→3195→3241→3654 |
| Precise typed share (№560) | **104/509 = 2043 bp** (the same 20 `"String"` rows are precise; `"Struct"` rides typed-but-coarse) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_precise_baseline.txt --precise` | only up: 1637→1650→2043 |
| `#[ignore]` debt (№468) | **17** (TODO: 0) — the gh#967 ledger lanes closed: §2 (№574, 5 tests), §1 (№575, 4 tests); the floor re-locked down in the same PRs | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | only down: 85→58→52→40→26→21→17 |
| `dead_code` (№468) | **33** (unchanged) | same gate | only down (unchanged) |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | only down (0 since В19) |
| Registered mirrors (№502/№564) | **6** (the html_label/vm doc-lineage lines; unchanged) | `python3 scripts/ci/mirror_counter.py` | only down (unchanged) |
| Workspace members (№567) | **5** (`.`, `metalogos-server`, `mlogpkg`, `mlog-lsp`, `metalogos-reflex`) | `grep -A20 '^\[workspace\]' Cargo.toml` | the №567 split fact |
| The 0.29 gate parameters (ADR-0181) | **OWNER-FIXED 2026-10-04** (`owner_fixed: true` — the №570 draft accepted verbatim; the typed-share cost line's arithmetic flag recorded in PR #987: ~+260 sigs = 8350 bp ≠ 3500 bp, the correct cost ≈ +14 — overdelivered by №573) | `head -20 scripts/ci/gate_029_goals.txt` | the owner's gate |

**The release state:** 0.28.0 published (tag on `3576887`, the binary +
the CycloneDX SBOM + Sigstore). The release-pipeline regression the В27
dispatch exposed (№567's bin move broke Build-and-Release while the
per-commit test-CI stayed 45/45 green) is closed by №572 — the release
workflows follow the bin and the `release_bin_guard.py` blocking job
re-runs the release-critical paths on every PR. The [Unreleased]
CHANGELOG carries the wave-27 entries; the next release cut folds them
per the №550 procedure.

### 6.14. Wave 28 recount (naryad №579): the counters (main @ `d5af542`, 2026-10-05)

Protocol: №414/№318/№576 — the same machines, no hand-written numbers.
The honest headline: В28 was the pre-M1 delivery line + the domain-line
reopening CONTOUR (a docs-only ADR) — the counters moved NOTHING, and
that is the wave's honest fact: no ignore was lifted, no signature was
typed, no mirror appeared; the floors hold exactly where В27 left them.
The §3 subsystem weights stay as of the №476 recount (§6.12).

**The Wave 28 counters (every value from its machine, the command
reproduces it — all unchanged from §6.13):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **186/509 = 3654 bp** (the 0.29 owner-fixed goal 3500 bp stays exceeded — ADR-0181 §3) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | unchanged (В28 carried no typing naryad) |
| Precise typed share (№560) | **104/509 = 2043 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_precise_baseline.txt --precise` | unchanged |
| `#[ignore]` debt (№468) | **17** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **33** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py` | unchanged |
| ADR count | **174** (the generated README row; `gen_metrics.py --check` green — №578's ADR-0182 is the 174th) | `python3 scripts/gen_metrics.py --check` | 173 → 174 (the ADR-0182 landing) |

**The Wave 28 facts (the wave's own work, not counters):**

- The pre-M1 delivery (№577, gh#983): the external-reporting sync
  landed — the metrics one-pager
  (the fresh snapshot, every number generator-read on the day), the CBM
  pilot extract (the 2026-10-03 measurement), the M1 checklist. The
  publication stays with the owner.
- The domain line reopened BY CONTOUR (№578, gh#991): ADR-0182
  (`docs/adr/0182-media-handles-backend-registry.md`, **Proposed**) —
  the honest stock-taking (the unified media layer №331, the backend
  registry №333 and the label lattice №322 are LANDED and built on),
  the gap named (the five media-taking backends accept raw strings —
  the ADR-0114 opacity discipline stops at the backend input; the
  registry has no capability axis), the Wave-29 map (Image → Audio →
  VideoFrame), the open questions to the owner (§7 of the ADR — the
  gate: the implementation naryads are NOT issued without the answers).
  Zero src edits — the diff is the ADR + the index (+ the generated
  README metrics row the docs-metrics gate demanded).
- The owner's gates executed in В28's frame: the Фаза-2 opening
  (decision 3Б, machine-recorded in `gate_029_goals.txt`, gh#979), the
  coverage floor 76% in gate mode (gh#980 — the advisory→blocking flip
  fires itself after two stable waves, no naryad needed), the office
  tails gh#989 (the typed-share cost arithmetic corrected, the
  owner-fixed values untouched; ADR-0179 → IMPLEMENTED) and gh#990
  (the FO-056 recount — the scenario pure-language again, gh#981
  closed).

**The release state:** 0.28.1 PUBLISHED (2026-10-05) — the owner's
gate №549 executed: the tag v0.28.1 on `d5af542`, the release
workflow green (run 37263503704), the four assets attached (binary,
SBOM, BUILD-INFO, SHA256SUMS), the Security announcement in the
release notes (the X-1 (High) guard bypass and the X-2 (Medium)
leftover-local leak, both fixed 2026-10-04). The
`fact_open_high_server` trajectory (№580) closed at **0** — the
release-block carrier #997 closed on the live tag per its own
condition; the syncing PR lands the machine records (both goals
files, the [0.28.1] CHANGELOG date). Both v2 gate reads (0.28
ADR-0179, 0.29 ADR-0181) are GREEN.

### 6.15. Wave 29 recount (naryad №589): the counters (main @ `73480aa`, 2026-10-05)

Protocol: №414/№318/№576/№579 — the same machines, no hand-written
numbers. The honest headline: В29 was the SECURITY wave per the audit
d63cc1d + the release line (0.28.1) + the process hardening (the X-5
squash-body rule, the honest-boundary protocol, the 0.29-gate wiring) +
the domain line's first IMPLEMENTATION step (the №599 Image bridge) —
the src moved (№584's VM respond-terminality lowering, №585's fuzzer
lane, №599's media bridge) while the registry and the floors stayed
exactly where В28 left them: no new builtin names (the №599 bridge is a
capability extension of the existing two builtins), no debt lifted, no
mirror added. The §3 subsystem weights stay as of the №476 recount
(§6.12).

**The Wave 29 counters (every value from its machine, the command
reproduces it — all unchanged from §6.14):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **186/509 = 3654 bp** (the 0.29 owner-fixed goal 3500 bp stays exceeded; the gate is WIRED into blocking CI by №597 — ADR-0181 §6, the bare CI run reads the 0.29 targets) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | unchanged |
| Precise typed share (№560) | **104/509 = 2043 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_precise_baseline.txt --precise` | unchanged |
| `#[ignore]` debt (№468) | **17** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **33** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py` | unchanged |
| ADR count | **174** (the generated README row; `gen_metrics.py --check` green) | `python3 scripts/gen_metrics.py --check` | unchanged (the В29 frame's ADR-0182 landed in В28's recount) |
| The release state | **v0.28.1 PUBLISHED** — the tag, the green release run (37263503704), the four assets, the Security announcement; `fact_open_high_server` = **0** (the #997 carrier closed on the live tag) | `scripts/ci/unfreeze_gate.py --office-tests pass` (§4 + v2) | the release-block lifted |

**The Wave 29 facts (the wave's own work, not counters):**

- The security wave per the audit d63cc1d: №581 (the RESPOND_NOT_TERMINAL
  compiler refusal) and №582 (the PushUnit route-epilogue) closed the
  X-1/X-2 classes in code; №584 finished the X-1 line — the respond
  terminality lowers on the TW early-answer surface, the VM parity is
  contract-tested, and the RESPOND_NOT_TERMINAL gate retired to an
  ADVISORY (the lint can never block a serve deploy again); №585 built
  the route-body differential fuzzer lane (the generator, the HTTP
  oracle, the seed corpus) — the serve contracts are now
  differential-tested, not snapshot-tested.
- The process hardening: №587 (X-5) — a squash body describes only its
  own naryad (the body check + the history audit: 152 naryad commits, 14
  true chained-PR instances, 0 false positives); №588 — the
  honest-boundary protocol (a marker in the diff obliges a
  limitations.md row in the same PR; the advisory CI job + the
  maintainers.md rule + the template item); №589 — this sync.
- The release line: №580/№583 prepared and closed the 0.28.1 contour
  (the version lockstep, the Security section first; the
  `fact_open_high_server` trajectory 1 → 2 → 1 → **0**, the goals files
  synced, the REALITY release state landed); the owner's gate №549
  executed — v0.28.1 PUBLISHED 2026-10-05.
- The owner package «Принимаю все твои рекомендации, выполняй»
  (2026-10-05) rode in the В29 frame: №597 wired the 0.29 gate into
  blocking CI (the DEFAULT `--gate-target 0.29`; both v2 gate reads
  GREEN); №598 fixed the ADR-0182 §7 answers (the ADR → Accepted, the
  implementation line opens); №599 landed the Image bridge —
  `vision_understand`/`ocr_extract` accept `Media(Image)` through the
  sanctioned read path with the label-join (the ADR-0182 §3.3 step 1 of
  3; the String overload forms untouched — the №493 posture).
- The OPEN line (honest): №586 (X-4 — the branch-protection audit job)
  stays BLOCKED by the owner's repo secret for the protection API — the
  DoD «зелёная на живой защите» is unreachable without it; the wave's
  only unfinished naryad, waiting on the owner, not on engineering.


### 6.16. Wave 30 recount (naryad №596): the counters (main @ `75438fa`, 2026-10-05)

Protocol: №414/№318/№576/№579/№589 — the same machines, no hand-written
numbers. The honest headline: В30 was the LANGUAGE ENRICHMENT wave for
the Камертон consumer (gh#1022) — the registry grew by SEVEN typed
builtins across the wave (the spectral contour, the UTC calendar
arithmetic, the Box–Muller normal sampler), the typed floors moved for
the first time since В27, the owner gate №593 was resolved by
delegation (the variant B, ADR-0183 Accepted), and the differential
family gained its third axis (the tick↔route parity, tests-only). The
§3 subsystem weights stay as of the №476 recount (§6.12).

**The Wave 30 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **194/516 = 3759 bp** (the 0.29 owner-fixed goal 3500 bp stays exceeded) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | **moved** (3654 → 3759 bp, +105 bp across the wave: the №591/№595/№590 rows) |
| Precise typed share (№560) | **110/516 = 2131 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_precise_baseline.txt --precise` | **moved** (2043 → 2131 bp) |
| `#[ignore]` debt (№468) | **17** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **33** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py` | unchanged |
| ADR count | **175** (the generated README row; `gen_metrics.py --check` green) | `python3 scripts/gen_metrics.py --check` | **moved** (+1: the wave's ADR-0183) |
| The release state | **v0.28.1 PUBLISHED** (the В29 fact, unchanged); `fact_open_high_server` = **0** | `scripts/ci/unfreeze_gate.py --office-tests pass` (§4 + v2 0.28/0.29, both GREEN) | unchanged |

**The Wave 30 facts (the wave's own work, not counters):**

- The language surface (each recorded by its own CHANGELOG entry):
  №591 — the spectral contour (`lomb_scargle`, `spectral_peak`; the
  Lomb–Scargle periodogram for unevenly sampled series, the degraded-loud
  struct, the [SPECTRAL_INPUT] gate); №592 — the per-call HTTP deadline
  taxonomy ([HTTP_TIMEOUT]/[HTTP_CONNECT]/[HTTP_STATUS]/[HTTP_TIMEOUT_RANGE],
  the mid-response break un-laundered); №595 — the UTC calendar
  arithmetic (`now_unix`, `date_parse_iso`, `date_diff_days`,
  `date_format_iso`, the [DATE_INVALID] gate, the explicit №316 Source
  override for the wall clock); №590 — the Box–Muller normal sampler
  over the shared PRNG (the [NORMAL_SAMPLE_STDDEV] gate, the bit-exact
  mutation pin). Registry 509 → 516, append-only (the .mbc contract).
- The process/owner lines: №593 — the migration-rollback boundary
  (ADR-0183 Accepted, VARIANT B: the schema evolution stays
  ADDITIVE-ONLY; the owner gate resolved by the delegated decision,
  the verbatim record in the ADR §4); №590's tail repair — the
  classification generator's OVERRIDES table carries the №565/№526 rows
  again, `gen_classification.py` RUNS on the main tree for the first
  time since №565 (the drift is repaired, `--check`-equivalent tests
  green); №594 — the tick↔route parity of a pure function (the sample
  program byte-identical across the tick and BOTH route backends on 100
  runs against the independent GoldenReplication, plus the deterministic
  tick↔route diff-fuzzer arm — the THIRD differential axis beside №465
  and №585; tests only, zero production fixes).
- The merge-order mechanics (the dispatch's rule, executed): the wave
  merged №591 → №592 → №595 → №593 → №589-adjacent → №590 last — the
  №590 PR was re-based onto the merged wave (the spectral-drop fix
  commit skipped: on the rebased main the spectral module belongs to
  №591), and the cumulative baselines were re-recorded by the merge
  that landed last (the №595 procedure).
- The limitations rows landed in their own PRs per the №588 protocol:
  the Spectral Contour section (№591) and the Schema Evolution Boundary
  section (№593) — verified present by this sync (nothing new to add:
  the wave's honest-boundary markers were carried same-PR, as the
  protocol demands).
- The OPEN line (honest, carried from В29): №586 (X-4 — the
  branch-protection audit job) stays BLOCKED by the owner's repo secret
  for the protection API — waiting on the owner, not on engineering.
  The owner gates ahead of the next wave: the В30 acceptance (the
  Камертон wave 1 start), the №586 secret, the string-form deprecation
  (the 0.30 line), the repo rename Metalogos- → Metalogos.

### 6.17. Wave 31 corrective record (naryad №602): the security release 0.28.2 prep (main @ `f85cfa5`, 2026-10-06)

Protocol: №414/№318/№576/№579/№589/№596 — the same machines, no hand-written
numbers. The honest headline: В31 is the CORRECTIVE wave (the unified audit
of `25b375e` / v0.28.1, the dispatch gh#1052) — its P0 line closed the audit
§3 Y-1 regression: №584 had retired the respond-terminality gate for ALL
forms at once, including the ONE form that never worked (a bare respond*
NESTED under a top-level block-form if/else branch — the audit's guard
bypass, the protected code executed for a non-admin under a style warning).
№600 restored the fail-closed refusal (the blocking `RESPOND_SWALLOWED`
error, the SSOT `RespondPosition` predicate shared by the semantic walk and
the №584 lowering); №601 rewired the swallow test contract and pinned the
audit §3 scenario end-to-end; №586 landed the X-4 branch-protection audit
job (the first read fixated the real divergences — the owner's admin
toggles, gh#1000). The execution-honesty finding: the №601-mandated test
exposed a PRE-EXISTING TW/VM divergence on the migration path itself (the
TW branch walk discarded an explicit `return respond(...)` carried by a
nested statement while the VM answered it — broken since №584); repaired in
the same PR, the honest-boundary marker carried per the №588 protocol.

**The Wave 31 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **194/516 = 3759 bp** (the 0.29 owner-fixed goal 3500 bp stays exceeded) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | unchanged (the wave is corrective — no language surface moved) |
| Precise typed share (№560) | **110/516 = 2131 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_precise_baseline.txt --precise` | unchanged |
| `#[ignore]` debt (№468) | **17** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **33** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py` | unchanged |
| ADR count | **175** (the generated README row; `gen_metrics.py --check` green) | `python3 scripts/gen_metrics.py --check` | unchanged |
| The release state | **0.28.2 PUBLISHED** (2026-10-06) — the owner's act: the tag v0.28.2 on `c694b6d8` (the version lockstep 0.28.2 — the lockstep PR #1058 landed the step the prep PR #1055 missed, the №583 pattern), the release workflow green (run 37412777202), the four assets verified (binary, SBOM, BUILD-INFO, SHA256SUMS — the SBOM describes `mlog 0.28.2`, checked on the downloaded artifact), the Security announcement in the release notes. The `fact_open_high_server` trajectory closed at **0** — the carrier #1041 closed on the live tag per its own condition, the release naryad #1043 closed with it; the syncing PR lands the machine records (both goals files) | `python3 scripts/ci/sync_gate_facts.py` (the API count of open release-block issues = 0, verified 2026-10-06); the goals files carry the honest 0 | **moved** (1 → 0, the live tag closed the carrier) |

**The Wave 31 facts (the wave's own work, not counters):**

- №600 (gh#1041, P0, release-block): the blocking `RESPOND_SWALLOWED`
  refusal — a bare respond* nested under a top-level block-form if/else
  branch blocks run/serve again on BOTH backends; the stable code stamps
  the №523 refusal first; the migration is one word (`return
  respond(...)`); the false-positive scan (363 .mlog files) — zero new
  refusals; the predicate mutation-verified at both transition points.
- №601 (gh#1042, P0): the swallow test contract rewired (the startup
  refusal with the stable code through the PRODUCTION serve entry — the
  test harness skips the gate by design); the audit §3 guard scenario
  negative on both backends + the migrated `return respond(...)` variant
  answering 403-before-the-protected-code / 200-with-the-marker; the
  №584 surfaces stay green.
- The execution-honesty repair (carried in №600's PR): the TW serve lane's
  top-level branch walk propagated an explicit nested `return` NOW (it was
  silently discarded since №584 — the flattening of
  `eval_statements_with_mutability`); the interpreter gained the
  ControlFlow-preserving `eval_nested_statement`; plain tail values stay
  discarded (the etalon; the depth-≥2 semantics fate — the owner gate
  №603).
- №586 (gh#1000, P2): the X-4 branch-protection audit job (the live
  protection read via the API, compared to the maintainers.md checklist,
  fail-closed; the weekly workflow + the read-only secret) — the FIRST
  read (2026-10-05) fixated the real divergences: `enforce_admins: false`,
  `strict: false`, `required_pull_request_reviews: null`, three
  documented checks not required; the owner's admin toggles, dissected in
  gh#1000.
- The fact discipline (the №580 precedent, executed): the release-block
  carrier #1041 stays OPEN across the fix merge (Part of, not Closes —
  the #997 lesson); the goals files carry `fact_open_high_server: 1`
  honestly; the closure is №602's act on the live v0.28.2 tag.
- The owner gates ahead: the v0.28.2 publication (§6.5), the depth-≥2
  semantics (№603), the precise-goal share (№605), №503/memory_forget
  (№609), the repo rename.

### 6.18. Wave 32 release record (naryad №614): the release 0.29.0 prep (main @ `5e28412`, 2026-10-06)

Protocol: №414/№318/№576/№579/№589/№596 — the same machines, no hand-written
numbers. The honest headline: 0.29.0 is the GATE-movement release — the 0.29
v2 gate closed GREEN (№613: 47 verified PRECISE rows, 2131 → 3042 bp ≥ the
owner-fixed 3000; the honest RED record #1069 preceded the movement), the
two semantics fixes ride it (№612 the loop semantics, №611 the schema
references), and the two owner decisions are archived as ADRs (ADR-0184
variant Б, ADR-0185 postponed). The owner's publication verdict landed in
the chat verbatim: «Делай релизный наряд 0.29.0» (2026-10-06) — the
executor runs the mechanical chain, the publication itself is the owner's
act per §6.5 (executed by the owner's PAT on the chat verdict, the v0.28.2
precedent).

**The Wave 32 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **241/516 = 4670 bp** (the 0.29 owner-fixed goal 3500 bp stays exceeded) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | **moved** (194/516 = 3759 → 241/516 = 4670, №613 — the 47 verified rows) |
| Precise typed share (№560) | **157/516 = 3042 bp** (the X-3 goal 3000 OWNER-FIXED, ADR-0181 §3.1 — MET) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_precise_baseline.txt --precise` | **moved** (110/516 = 2131 → 157/516 = 3042, №613) |
| `#[ignore]` debt (№468) | **17** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **33** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py` | unchanged |
| ADR count | **177** (the generated README row; `gen_metrics.py --check` green) | `python3 scripts/gen_metrics.py --check` | **moved** (+2: ADR-0184, ADR-0185 — the owner decisions) |
| The release state | **0.29.0 PUBLISHED** (2026-10-06) — the owner's act on the verbatim chat verdict («Делай релизный наряд 0.29.0»): the tag v0.29.0 on `4be0f729` (the version lockstep 0.29.0 in ONE PR — the №583 pattern: workspace Cargo.toml + Cargo.lock ×5 members + the README badge (badge_sync.py) + the generated Version row and SSOT blocks (gen_metrics.py)), the release notes per the v0.28.2 shape (the v2 gate GREEN table, the decisions, the assets block), the release workflow green (run 37477763021), the four assets verified on the downloaded artifacts (binary `mlog 0.29.0`, SBOM = `metalogos 0.29.0`, BUILD-INFO — rustc 1.99.0 on ubuntu-24.04, SHA256SUMS — all digests OK), the two Sigstore attestations present (the SLSA provenance + the SBOM bound to the binary); the syncing PR lands this record (the №583/PR #1059 pattern) | `python3 scripts/ci/unfreeze_gate.py --office-tests pass --gate-target 0.29` (the fresh run at the release cut: §4 GREEN, v2 GREEN — typed 4670/3500, precise 3042/3000, open High 0, quorum 0/8, the serve-e2e inventory done) | **moved** (the gate movement is the release's own work) |

**The Wave 32 facts (the wave's own work, not counters):**

- №605 (gh#1046, P1): the X-3 precise-share goal in the 0.29 gate —
  `goal_precise_share_bp: 3000` OWNER-FIXED (ADR-0181 §3.1; the chat
  authorization «1-Б, 2-3000, 3-отложить и добивай остальное», the marker
  `w31: precise-goal`, comment 6010562145); the coarse goal stays the
  floor; the honest RED record (#1069: the v2 verdict NOT MET on precisely
  this parameter, 2131 bp) landed BEFORE the movement — the ratchet reads
  the fact, never the hope.
- №613 (gh#1070, P1): the movement naryad — 47 rows of the registry to
  PRECISE by the fact of the single Ok-form; the full verification table
  in the naryad; the honest Unknowns kept (№560: `__first`/`__last`, the
  telegram trio (Unit‖String), `poll_wake`/`take_interrupt` (Struct‖Unit),
  `hash_password` (Hash), `query` (Query), `respond` (HttpResponse), the
  coarse families); the floors raised in the same PR per the №757
  procedure (precise 3042, general 4670, TYPED_FLOOR 237 — the compiled
  count measured by the test run); the CI lesson repeated and absorbed:
  the gen_metrics SSOT blocks include the type metrics — the first run's
  blocking docs-metrics catch was regenerated by the GENERATOR, never by
  hand.
- №612 (gh#1061): the loop semantics — a bare call in the body never
  terminates the loop, the tails execute; the vacuous greens (a test body
  whose early termination flattened into Ok) eliminated as a class — the
  harness reads ControlFlow now; the corpus scan: zero test bodies use
  explicit return; the TW↔VM parity e2e pinned (the VM never fabricated
  the Return — the honest count 3 on BOTH backends).
- №611 (gh#1060): the schema-as-code references — the qualified name
  parsed in every spacing (the dot is the separator), a loud parse error
  instead of the silent drop; `mlog check` validates the generated DDL on
  an in-memory SQLite dry-run over the SSOT renderer (`ast::schema_table_ddl`)
  — the check only guarantees the apply when both produce the identical
  string.
- ADR-0184 (№603, gh#1044): the depth-≥2 respond boundary — REFUSED
  FOREVER, variant Б (the owner's decision; `RESPOND_SWALLOWED` stays
  forever, the migration remains one word). ADR-0185 (№609, gh#1050): the
  memory_forget container form — DEFERRED to the memory-phase planning,
  postponed, NOT rejected (the owner's decision).
- №608 (gh#1049): the Y-4 quota line in the wave report (the counter
  learns the real formats, the `--line` mode). №610 (gh#1051): the
  newcomer path — the broken first step fixed (the clone URLs), the live
  pool list, 9/9 acceptance criteria.
- The owner gates remaining (not the executor's): the 0.29 verdict form
  (satisfied by the chat verdict above), branch protection (gh#1000 — the
  owner's admin toggles), the LLM-line sequencing (ADR-0182 §7.5), the
  0.30 string deprecation, №503 (strictly last), the Known-Issue ledger
  №569 (the revision 2026-10-15).

### 6.19. Wave 33 post-wave record (naryad №619): the ПСРМ development wave after the 0.29.0 release (main @ `663447e`, 2026-10-06)

Protocol: №414/№318/№576/№579/№589/№596 — the same machines, no hand-written
numbers. The honest headline: В33 is the first DEVELOPMENT wave after the
В31 corrective + В32 release line — the external Камертон cycle closed
(№618: the R1–R6 verdicts from real runs, the mutation probe goes red),
the gh#967 §3 lane closed in the implementation (№617: the variable-scope
walk + the static opaque-concat check — the three semantic-checker
failures became the check-time refusals on BOTH backends), the first
0.30 typed-share movement landed (№616: 61 verified PRECISE rows —
precise 3042 → 4224 bp, general 4670 → 5852 bp), and the 0.30 gate
exists as the honest DRAFT (№615: gate_030_goals.txt, owner_fixed:
false, the X-3 floors strictly above the live facts; the default gate
target stays 0.29 — the blocking CI never reads a draft; the fixation is
the OWNER's gate, dispatch В33 §Гейты п.1).

**The Wave 33 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **302/516 = 5852 bp** (the 0.29 goal 3500 bp; the 0.30 DRAFT proposal 6100 bp) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | **moved** (241/516 = 4670 → 302/516 = 5852, №616 — the 61 verified rows) |
| Precise typed share (№560) | **218/516 = 4224 bp** (the 0.29 goal 3000 OWNER-FIXED; the 0.30 DRAFT proposal 4500) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_precise_baseline.txt --precise` | **moved** (157/516 = 3042 → 218/516 = 4224, №616) |
| In-tree TYPED_FLOOR (the compiled lock) | **298/499** | `cargo test -p metalogos --test type_signature_metric` | **moved** (237 → 298, the same PR — the №757 procedure) |
| `#[ignore]` debt (№468) | **14** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | **moved** (17 → 14, №617 — the three gh#967 §3 ignores lifted green) |
| `dead_code` (№468) | **33** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py` | unchanged |
| ADR count | **178** (the generated README row; `gen_metrics.py --check` green) | `python3 scripts/gen_metrics.py --check` | **moved** (+1: ADR-0186 — the 0.30 gate draft) |
| Blocking-checks cells | **31** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged (the 0.30 draft adds no cells — the counter grows only explicitly) |
| The release state | **0.29.0 PUBLISHED** (2026-10-06T14:17Z) — unchanged through the wave; the gate 0.29 v2 stays GREEN with the moved precise fact (4224 ≥ 3000, the goal file synced by №616 — the machine-read rule) | `python3 scripts/ci/sync_gate_facts.py` (with the token: «every fact_* record matches») | unchanged |

**The Wave 33 facts (the wave's own work, not counters):**

- №617 (gh#1075, PR #1081): the gh#967 §3 lane — the variable-scope walk
  (the TW env model verbatim: flat, never popped, the entities as
  globals, the render/reflex_* name-reference slots exempt) + the static
  opaque-concat check (the declared-fact twin of the eval_binop guard);
  the new `UndefinedVariable` kind carries the runtime's stable
  UNDEFINED_VARIABLE code; the `OpaqueConcat` kind stamps no invented
  code (the runtime origin has none). The corpus-driven corrections
  recorded loud: the reserved-keyword ident exemption (the tolerated
  self-host parser artifacts — parser.mlog only, 0 real hits across the
  corpus; the parser artifact itself — stray keyword tokens accepted as
  bare Idents — is a separate finding, PR-comment 6022497489), and the
  №465 fuzzer lane fix (run_vm carries the №523 semantic gate — the
  production `mlog run --backend vm` order; the lane compared compile
  behavior unreachable in production; the №479 compile-site pin moved to
  the direct `mlog compile` surface). The debt floor 17 → 14 follows the
  MACHINE fact (the naryad's «52 → 49» was stale — noted in the
  baseline).
- №618 (gh#1076, PR #1082): the Камертон closure report
  (`docs/audits/kamerton-report-v0.29.0.md`) — the SHA256 of the v0.29.0
  assets re-run 3/3 OK over the downloaded artifacts, the R1–R6 verdicts
  from real runs of the repro corpus (`scripts/repro_kamerton/`, the t92
  reconstruction, lands with the report per the §16.0-D honest note):
  R1/R2/R2c identical green, R3/R3b the honest count 3 with the tail
  executed, R4 green honestly, **R4a (the mutation probe) RED exactly as
  the contract demands** (`assert_eq failed: 3 != 999`), R6 the full
  pass with i == 3.0; the D1/D2 map (№611/PR#1067/`7b72f09`,
  №612/PR#1068/`082a213`), the affected releases (v0.28.1, v0.28.2 →
  the update path is v0.29.0), the t92 corrections verbatim. The
  forwarding to the external reviewer — the OWNER's gate (dispatch В33
  §Гейты п.2). The report is in technical English per the №383 lint
  (1.78% Cyrillic — the two verbatim t92 quotes under the threshold).
- №616 (gh#1077, PR #1090): the PRECISE movement 0.30 №1 — the №613
  procedure hardened (the machine candidate selection: every Ok a
  literal `Ok(Value::X)`, a single scalar shape; the dynamic-Ok forms
  excluded as the polymorphic risk) + every handler read. 61 rows typed;
  the honest exclusions recorded (vision_fetch_weights — the cfg-dual Ok
  behavior; the polymorphic first/last family). The floors raised in the
  same PR (№757): precise 4224, general 5852, TYPED_FLOOR 298 (the
  compiled delta exactly +61 — no package row behind a feature gate).
- №615 (gh#1078, PR #1091): the 0.30 gate draft — gate_030_goals.txt
  (owner_fixed: false; typed 6100 > 5852, precise 4500 > 4224 — the X-3
  floors strictly above the live facts; the same checkers, no new
  fact_* keys) + ADR-0186 (Proposed: the succession, the X-3 rule
  applied to the draft itself, the window checklist, the
  parameter-change rule) + the unfreeze_gate 0.30 target read (the
  honest RED on the fixation fact; the default stays 0.29). The
  fixation — the OWNER's gate.
- The environment lessons (not ledger items): the docs-metrics SSOT
  blocks include the typed/precise share rows and the ADR count — every
  registry/ADR change regenerates them (caught locally twice, the
  PLAN-SUMMARY follow-up commit once); a CI push event may not fire
  (the №616 ada4010 case — the retrigger with an explicit commit);
  the git remote's embedded token can fail fetches while the PAT works
  (fetch with the PAT explicitly).
- **2026-10-07 (wave 34, addendum):** the Security addendum to the
  published 0.29.0 issued by naryad №620 (gh#1083) — the `### Security
  (ADVISORY — restart your tests)` section in the CHANGELOG `[0.29.0]`
  and the same block appended verbatim to the GitHub release body (the
  existing sections untouched): the №612 defect's user-facing
  consequences (the guard patterns with an audit loop stop after one
  iteration; the batch inserts/updates execute one element; the vacuous
  green runs), the affected surface (`mlog run` / `mlog test` /
  `mlog serve` with `METALOGOS_SERVE_BACKEND=interpreter`, every release
  before 0.29.0; the VM not affected), the user action — restart your
  tests on ≥ 0.29.0. The tag and the assets are NOT re-created (the fix
  is IN 0.29.0 — the communication warns, the binaries do not change).
- **2026-10-07 (wave 34):** the implicit-block-value class scan issued by
  naryad №622 (gh#1085) — the №612 etalon applied to the consumers
  OUTSIDE the loops; the verdict table (consumer × semantics × file
  fact): `Statement::Match` arms/else — DISCARDED (the `eval_block!`
  replaces the running implicit value unconditionally, propagates only
  the explicit Return); `Statement::IfThen` (if WITHOUT else) and
  `Statement::IfElseBlock` branches — DISCARDED (the same macro path);
  `Expr::MatchExpr` arm blocks — DESIGNATED (the block's tail value IS
  the match expression's result; the expression channel cannot carry a
  control signal); `Declaration::Sandbox` — **N/A** (the grammar has NO
  sandbox body: `SandboxDecl { span, name, allowed, forbidden, timeout }`
  is a policy declaration, nothing executes a statement block);
  `Declaration::Flow` — **N/A** (the flow body is a pipeline of pattern
  invocations, no statement block exists in `FlowDecl`). The scan FOUND
  one parity gap and it was FIXED in the same honest class: `return`
  inside a value-channel arm body was captured as the block value by
  the TW (the documented flatten) while the VM emitted the
  function-level `Instruction::Return` and terminated the pattern — the
  VM capture landed (`Instruction::SetValueReg`, the UNCONDITIONAL
  register store — the TW `eval_statements` flatten contract, Unit
  included; the №622 pins: `let v = match x { "a" then { return 5.0 } }`
  answers "v=5:tail" on BOTH backends). The regression pins per
  consumer (the bare non-Unit call as the construct's last statement +
  the code after the construct) run the production order of BOTH
  backends — 6/6 green; the full suite green.

### 6.20. Wave 34 post-wave record (naryad №625): the verdict-execution wave after the В34 dispatch (main @ `2b6dec96`, 2026-10-07)

Protocol: №414/№318/№576/№579/№589/№596 — the same machines, no
hand-written numbers. The honest headline: В34 executed the OWNER's
2026-10-07 verdict («принимаю форму довеска В34 как основу, значение
доли параметризованных фиксирую X%…» — the "X%" resolved to the
addendum's draft benchmark ≥ 50% of 84, recorded verbatim in the gate
file) and the dispatch gh#1089 queue — and the wave's two testing
naryads (№621/№622) each surfaced a REAL latent defect that a repair
naryad fixed in the same wave (№629, №622's own finding):

- №626 (gh#1093) — the 0.30 gate draft synced with the В34 addendum: the
  typed parameter = the parameterized share among List/Struct (the draft
  benchmark ≥ 50% of 84), the typed/precise goals demoted to the
  ratchet-records (Z-2), the branch-protection-audit GREEN draft
  criterion, the X-3 start anchor rebound to gh#1086; the owner's
  verdict recorded VERBATIM; owner_fixed stays false (the §5 transition
  = №628, gh#1111, blocked by gh#1000 — the OWNER's admin gate).
- №620 (gh#1083) — the Security addendum to the PUBLISHED 0.29.0: the
  `### Security (ADVISORY — restart your tests)` section in the
  CHANGELOG `[0.29.0]` and the SAME block appended verbatim to the
  GitHub release body (the script-verified identity, the idempotent
  re-run); the tag and the assets NOT re-created. The №604 dictionary
  class: a return INVENTED by the implicit block value — the mirror of
  «a respond*/deny*/return being IGNORED».
- №621 (gh#1084) — the diff fuzzer produces the №612 class AND the
  honest finding: **the sweep was VACUOUSLY GREEN — 0 of 150 programs
  executed** (the legacy 2-arg `memory_forget` against the №280
  surface; the bare greek idents against the №617 static scan; the
  heterogeneous `+`/`==`; the each branch dead since birth — the `{{`
  string never parsed). Repaired: the typed-safe generator, the live
  each family, the №612-class shapes (the bare-call loop bodies, the
  call-assignment mixtures, the code after the loop, the nested forms),
  and the **permanent LIVENESS ratchet** (`n621_sweep_liveness_ratchet`:
  ≥ 60% of the sweep must execute on BOTH backends, the floor only-up).
  The mutation probe (№503/№618 procedure): the reverted №612 → RED
  (the direct probe: `bare_each`/`nested` tw=`"1"` vs vm=`"tail"`; the
  full sweep — the un-pinnable outcome divergences), the fixed main →
  GREEN.
- №629 (gh#1096, the repair naryad born from the №621 un-vacuumed sweep)
  — the VM comparison parity: `eval_cmp` mirrors the TW matrix 1:1 (the
  heterogeneous Eq and the non-Float ordering REFUSE on the VM now — the
  legacy silent `false` and the numeric-string coercion closed in the
  fail-closed direction, the №479 `+` precedent; `Unit == Unit` → true
  on both). The `Instruction` set unchanged for this fix.
- №622 (gh#1085) — the implicit-block-value class scan OUTSIDE the
  loops: the verdict table (match arms / if-without-else / if-else —
  DISCARDED via `eval_block!`; `Expr::MatchExpr` — DESIGNATED;
  `Sandbox`/`Flow` — **N/A**: no statement body exists in the grammar).
  The scan FOUND the parity gap: `return` inside a value-channel arm
  body was captured by the TW but TERMINATED the pattern on the VM —
  fixed in the same honest class (`Instruction::SetValueReg` — the
  UNCONDITIONAL value-register store, the `eval_statements` flatten
  contract, Unit included; appended at the enum end — the wire format
  stable).
- №623 (gh#1086) — the parameterized signatures stage-0: `Struct<...>`
  erasure in `from_path` (symmetric to List), the FIRST HONEST PACKAGE
  of 7 verified rows (weather `Struct<Weather>`, weather_forecast
  `List<DayForecast>`, geo_ip `Struct<GeoLocation>`, form_data
  `Struct<FormData>`, json_body `Struct<JsonBody>`, llm_usage
  `Struct<LlmUsage>`, mcp_list_tools `List<Tool>` — the handler-read
  table; the honest exclusions: respond/respond_html/query — opaque),
  and the **THIRD metric**: the parameterized share among the List/Struct
  rows (7/91 = 769 bp — the baseline + the in-tree lock through
  `BuiltinSpec::parameterized` + `path_is_parameterized`, the enum stays
  erased). The floors raised in the same PR (№757): the general 5852 →
  5988 bp, TYPED_FLOOR 298 → 305; the precise share UNTOUCHED (Z-2).
- №624 (gh#1087) — ADR-0187 (Proposed): the memory-phase plan (D1 the
  KNN recall reference over the #272 infra, D2 the forget↔recall
  consistency invariant, D3 the ADR-0185 container-form decision slot,
  D4 the consent-propagation design, D5 the memory e2e inventory; the
  §13 parity posture: TW+VM, JIT out per ADR-0073; owner_fixed: false).
  The rider: the ADR-0186 index row (the №615 gap).
- №625 (gh#1088) — this sync, strictly last.

**The Wave 34 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **309/516 = 5988 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | **moved** (302/516 = 5852 → 309/516 = 5988, №623 — the 7 parameterized rows) |
| Precise typed share (№560) | **218/516 = 4224 bp** | `… --gate scripts/ci/type_signature_precise_baseline.txt --precise` | unchanged (the Z-2 verdict: the scalar movement STOPPED — gh#1077 superseded) |
| **Parameterized share (№623 — the NEW third metric)** | **7/91 = 769 bp** | `… --parameterized --gate scripts/ci/type_signature_parameterized_baseline.txt` | **new** (0/84 at the audit t94 → 7/91 — the first honest package) |
| In-tree locks | **TYPED_FLOOR 305, PARAM_FLOOR 7/87** | `cargo test --test type_signature_metric` | **moved** (+7 compiled typed; the new parameterized lock) |
| `#[ignore]` debt (№468) | **14** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **33** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py` | unchanged |
| ADR count | **179** (the generated README row) | `python3 scripts/gen_metrics.py --check` | **moved** (+1: ADR-0187) |
| Blocking-checks cells | **31** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged |
| The fuzzer liveness (№621 — the NEW ratchet) | **≥ 60% of the sweep executes on BOTH backends** (the floor) | `cargo test --test naryad_465_diff_fuzzer` | **new** (the vacuous-sweep lesson — the harness that refuses everything compares nothing) |
| The release state | **0.29.0 PUBLISHED** — the Security ADVISORY appended to the release body (№620), the tag/assets unchanged | the API + `python3 /home/z/my-project/scripts/n620_release_body.py` (the idempotency check) | unchanged |

**The owner gates remaining after В34:** gh#1000 (the branch protection —
blocks №628, the §5 fixation), the parameterized-share value if ≠ ≥ 50%
of 84 (the §5 path), the stage-2 enforcement verdict (К-Б — not filed),
the ledger gh#967 §4–§7 (the revision 2026-10-15), the Камертон
forwarding. The wave-35 registered candidates:
№627 (gh#1110, the field-label metadata — the stage-2 prep), №628
(gh#1111, the §5 fixation).

**The wave report line (№608/№470):**
`naryad-quota (№470/№608): 11 naryads — domain 1/11 = 9.1% — PASS (the standing limiter <= 1/3)`

### 6.21. Wave 35 post-wave record (naryad №636): the ПСРМ development wave — the 0.30 §5 transition, the parameterized movement №2, the debt/self-host/VM-template hygiene, the field-label stage-2 preparation (main @ `c2b1bb9`, 2026-10-08)

Protocol: №414/№318/№576/№579/№589/№596/№619/№625 — the same machines,
no hand-written numbers. The honest headline: В35 executed the dispatch
gh#1105 queue (7 workorders) plus the registered candidate №627 (the
owner's К-А verdict execution, 2026-10-07) — the §5 MECHANICAL
TRANSITION of the 0.30 gate landed (the DEFAULT gate target now reads
0.30), the parameterized share moved 769 → 3736 bp (the second honest
package), and the debt/self-host/VM-template hygiene lanes all lifted.
The dated workorder №632 (the plan-relevance audit, release №3) stays
OPEN by design — its execution date is 2026-11-01 (± 1 day); the wave
queue treats it as verified-wait, NOT as no-progress:

- №630 (gh#1098) — the §5 MECHANICAL TRANSITION (ADR-0186 §5), one PR:
  `owner_fixed: true` (the owner's 2026-10-07 verdict recorded verbatim
  in the gate header by №626), ADR-0186 Proposed → **Accepted**, the
  DEFAULT gate target of `unfreeze_gate.py` 0.29 → **0.30** (the №597
  pattern: the parameters first, the CI ratchet after — the
  precondition gh#1086 (№623, the third-metric checker) merged before
  this PR, the fail-closed №525 rule honored). The 0.28/0.29 records
  stay verifiable explicit targets. The machine read on the merged
  main: the 0.30 ABSOLUTE goals read is RED-loud on the typed parameter
  (3736 bp live < 5000 bp goal — the movement is the requirement, the
  strict release-time read exits 1 by design).
- №631 (gh#1099) — the parameterized movement 0.30 №2: the SECOND
  honest package of 27 verified rows of the registry-order contour
  (split/lines/words/kv_list/memory_*/todo_*/goal*/goals_*/get_profile/
  human_mood → the typed `List<String>`/`Struct<...>` spellings; the
  honest polymorphic exclusions: push/slice/zip/sort_by/filter/… — the
  element type is the caller's, the №536/№537 posture). The parameter
  share 7/91 = 769 bp → **34/91 = 3736 bp**; the floors raised in the
  SAME PR (№757): the parameterized baseline 769 → 3736, the in-tree
  PARAM_FLOOR 7 → 31 (the compiled twin 31/87). The 0.30 goal
  (≥ 5000 bp) still demands movement — №631 is a step, not the wall.
- №633 (gh#1101) — the dead_code burn: 33 → **15** (18 places removed
  or justified), the floor re-locked in the same PR.
- №634 (gh#1102) — the self-host lane (the gh#967 §5): the §5 lexer
  test lifted GREEN — the FIXTURE had drifted behind the language
  (let-mut, the no-entry flow, the newline in the quote class, the
  stale keywords), not the parser; the №617 artifact documented-accepted
  with the mutation-verified pin; the ignore floor 14 → **13**. The
  ledger §5 RESOLVED mark + the closing comment landed with the naryad
  itself (verified 2026-10-08).
- №635 (gh#1103) — the VM template_render lane (the gh#967 §6): BOTH
  vm_golden corpus sweeps lifted green (189/189) — the template gap was
  closed by №250, the harness facts fixed (the env sidecars №385, the
  candle-gated exclusion mirror, the browser-gated p88); the ignore
  floor 13 → **11**. The ledger §6 RESOLVED mark + the closing comment
  landed with the naryad itself (verified 2026-10-08).
- №627 (gh#1110, the wave-35 registered candidate — the К-А execution)
  — the field-label METADATA (the stage-2 preparation, ADR-0178): the
  form `Struct<Name>{field:label,...}` (the label vocabulary = the
  stage-0 `Label::as_str` tokens; NO spaces, the ASCII field names —
  the machine-checkable grammar), `BuiltinSpec.field_meta` as the
  registry SIDE-TABLE (the `Type` enum NOT extended — the stage-2
  minimality), `from_path` fail-closed (a malformed section is honest
  `Unknown`), `parse_field_meta` the consumer API. The FIRST honest
  package of 28 verified field-label rows: GeoLocation 9 (the external
  ip-api body + the caller-arg echo → untrusted, labels.rs:509),
  Weather 13 (the ten Open-Meteo wire numbers untrusted; `description`
  the in-tree WMO table internal; `country` the in-tree constant
  internal; `city` the user-arg echo untrusted), LlmUsage 6 (the
  in-process counters internal). The honest exclusions: json_body/
  form_data (dynamic user shapes — no fixed field vocabulary),
  Tool/DayForecast (List elements — the section is a Struct form at
  stage 2). The **FOURTH metric**: the field-label share among the
  parameterized Struct rows — 3/20 = 1500 bp (the baseline +
  `FIELDMETA_FLOOR` 3/19 compiled — the two-locks discipline). The
  honest RED before the movement: the standalone probe 7/7 old-green,
  both new probes RED (the form did not parse, the parameterized flag
  was false). The parser mirrors synced in the same PR: gen_metrics
  (its own typed-regex lacked `{}:,` — the meta rows fell out of the
  typed count 309 → 306 AND inflated the module count 45 → 48),
  gen_reference (HANDLER_RE), the two Rust pins (readme_consistency,
  naryad_566) — the SSOT blocks + REFERENCE.md regenerated by the
  generator (№613 lesson); the lomb_scargle doc drift (№607) caught up.
- №636 (gh#1104) — this sync, strictly last.

**The Wave 35 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **309/516 = 5988 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | unchanged (the 0.30 movement lives in the parameterized share) |
| Precise typed share (№560) | **218/516 = 4224 bp** | `… --precise --gate scripts/ci/type_signature_precise_baseline.txt` | unchanged (the Z-2 verdict stands) |
| Parameterized share (№623) | **34/91 = 3736 bp** | `… --parameterized --gate scripts/ci/type_signature_parameterized_baseline.txt` | **moved** (7/91 = 769 → 34/91 = 3736, №631 — the 27-row package); the 0.30 goal 5000 bp RED-loud — the movement continues |
| **Field-label share (№627 — the NEW fourth metric)** | **3/20 = 1500 bp** | `… --fieldmeta --gate scripts/ci/type_signature_fieldmeta_baseline.txt` | **new** (0 at the В34 record → 3/20 — the first honest package of 28 field-label rows) |
| In-tree locks | **TYPED_FLOOR 305, PARAM_FLOOR 31/87, FIELDMETA_FLOOR 3 (3/19 compiled = 15.78%)** | `cargo test --test type_signature_metric` | **moved** (the new field-label lock; the param lock raised by №631) |
| `#[ignore]` debt (№468) | **11** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | **moved down** (14 → 13 №634, 13 → 11 №635) |
| `dead_code` (№468) | **15** | same gate | **moved down** (33 → 15, №633; the floor re-locked in the same PR) |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py --gate scripts/ci/src_mirror_baseline.txt` | unchanged |
| ADR count | **179** (the generated README row) | `python3 scripts/gen_metrics.py --check` | unchanged (ADR-0186 Accepted in place — no new file; ADR-0187 landed in В34) |
| Blocking-checks cells | **31** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged |
| The 0.30 gate posture | **owner_fixed: true; the DEFAULT target 0.30; the typed parameter RED-loud (3736 < 5000 bp)** | `python3 scripts/ci/unfreeze_gate.py` (the plain read reports loudly; the strict release-time read exits 1) | **the §5 transition landed** (№630) |
| The release state | **0.29.0 PUBLISHED** (the Security ADVISORY carried, №620) | the release API | unchanged |

**The owner gates remaining after В35:** gh#1000 (the branch
protection — the §5-adjacent branch-protection criterion of the 0.30
gate stays a draft line without a checker, №525), the parameterized
movement to the 0.30 goal (≥ 5000 bp — the next typing packages), the
stage-2 enforcement verdict (К-Б — not filed; №627 prepared the
metadata surface), the ledger gh#967 §4/§7 (the revision 2026-10-15;
the lanes осознанно not filed — the dispatch flag), the Камертон
forwarding, №632 (gh#1100 — the
plan-relevance audit release №3, the execution date 2026-11-01 ± 1
day). The В34 dispatch gh#1089 remains open as of this record — the
housekeeping observation for the coordinator (all its workorders
merged and the docs synced).

**The wave report line (№608/№470):** the wave-scoped surface (the 8
workorder issues gh#1101/#1102/#1103/#1100/#1098/#1099/#1104/#1110 via
`--issues`):
`naryad-quota (№470/№608): 8 naryads — domain 2/8 = 25.0% — PASS (the standing limiter <= 1/3)`
(the whole-dispatch surface gh#1105 reads 16 naryads — domain 2/16 =
12.5% PASS — the same verdict; the count surface includes the
carry-over refs the dispatch body cites).

### 6.22. Wave 37 post-wave record (naryad №647): the unified-audit execution wave — the security accounting restored, the dictionary on CHANGELOG, the sensitive-surface metric, the spec/conformance layer, the 0.30.0 lockstep (main @ `6eafee1c`, 2026-10-08)

Protocol: №414/№318/№576/№579/№589/№596/№619/№625/№636/№641 — the same
machines, no hand-written numbers. The honest headline: В37 executed the
unified-audit findings (48301708, the dispatch gh#1136) — the Q-1
security-accounting defect (a High fix in [Unreleased] without `###
Security`, invisible to the №562 gate) is closed ON BOTH layers (the
section itself + the dictionary enforcement on CHANGELOG), the Q-2
Goodhart risk got its fixed-denominator answer (the sensitive-surface
coverage), the §6.2 R1 landed (the normative spec + the conformance
suite with the both-backend oracle — «спецификация — эталон»), and the
0.30.0 lockstep is cut with the gate GREEN and `--strict` exit 0. THE
PUBLICATION OF 0.30.0 IS THE OWNER'S ACT (§6.5) — as of this record the
release is STAGED (the tag v0.30.0 on `6eafee1c`, the release body
drafted; the owner's request carries the Z-3 recommendation: the
branch-protection switches BEFORE publishing). The dated workorders:
№648 (the plan-relevance audit, release №4) verified-wait to
2026-12-01; the В36 tail (№639 the ledger revision 2026-10-15, №641 the
В36 docs) rides its own dates.

- №642 (gh#1129, PR #1137 → `81177c8`) — the `### Security` section in
  [Unreleased]: the №629 entry moved under it with the
  AFFECTS-0.29.0-AND-EARLIER stamp (all six operators incl. `!=`; the
  guard scenario `if owner != current_user { … }` with `owner = Unit`
  — the X-1/Y-1/Q-1 class) and the date stamps the №562 gate demands
  (the fix 2026-10-07 = 22d62fa AFTER the v0.29.0 tag; the audit
  2026-10-08). The gate moved from `no-security-part` to counting (the
  oldest stamp 1 day at the merge). The №617 decision: the entry STAYS
  in Changed — the runtime opaque-concat guard existed AT v0.29.0
  (`git grep -c "cannot concatenate opaque" 4be0f72 -- src/` →
  execution.rs ×2, vm.rs ×2), №617 added the compile-time twin; no
  hole, no move (М3). The README metrics block regenerated by the
  generator (684 → 685 KB).
- №643 (gh#1130, PR #1140 → `55bce5bb`) — the №604 security dictionary
  extends to CHANGELOG.md: a dictionary-matching ADDED entry in
  [Unreleased] OUTSIDE `### Security` without the release-block
  evidence = exit 1 (the same check_security_rows machinery — no
  second parser, №586/№606); the section attribution follows the
  post-image heading flow (an added entry under an UNCHANGED `###
  Security` heading is attributed right — the №642-class diff). The
  fact-backed scope calibration: the released sections pass by the
  release (the M-3/№562 limbo is «a fix outside a RELEASE»; the №620
  protocol routes the post-release amendments to [Unreleased]) — the
  retro: 1237 entries, 11 dictionary matches, 8 outside Security ALL in
  released sections, **0 in [Unreleased]** → IN SYNC (п.3: zero false
  positives in the blocking scope). The self-test 17/17; the honest
  measured boundary pinned: the LITERAL №629 text is NOT dictionary-
  caught (the «silent false» class words are not in the machine
  vocabulary — a calibration for it is a separate evidence-backed PR,
  the gh#1002 budget). The in-scope repair: the docstring's «API error
  = exit 2» was not enforced — the GithubApiError boundary lands (loud
  exit 2 at every caller).
- №644 (gh#1131, PR #1138 → `636b3cbb`) — the sensitive-surface
  coverage metric (the Q-2 fixed-denominator answer):
  `scripts/ci/sensitive_surface_share.py` — sources = Role::Source ∧
  label ≠ Public (**138 fixed**: Internal 102/Network 31/Secret 5),
  covered = typed ∧ (scalar ∨ parameterized) ∧ (¬Struct ∨ fields
  labeled) = **46/138 = 3333 bp** (the audit's t97b figures reproduced:
  54 typed among the sensitive, 8 not covered — 6 coarse, 2 Struct
  without the field-meta). The mirrors REUSED from
  type_signature_share.py (no second parser). The only-up floor
  (sensitive_surface_baseline.txt, 3333); the RECORD line
  `record_sensitive_surface_bp: 3333` + the 0.31 DRAFT benchmark
  `draft_sensitive_surface_goal: 4130 bp` (the first audit-order
  packages; the fixation is EXCLUSIVELY the owner's §5 path — №525).
  The CI wiring: the ADVISORY report step (the №535 rule — no blocking
  status introduced). The first-package candidates (§6.3 order): db
  (query*) 4/1, mail (imap*) 3/0, contacts/calendar (card_*/cal_*)
  10/6, request_body 1/0 — the private first labels wait for the К-Б
  verdict.
- №645 (gh#1132, PR #1139 → `8a5d13d5`) — the normative spec + the
  conformance layer (the §6.2 R1): `docs/spec/` (the charter + the
  topic 1 «comparisons and truthiness» — **12 norms S-VAL-001..012**,
  each with the TW+VM anchors and the checked-in pair),
  `tests/conformance/` (12 pairs `<id>.mlog` + `<id>.expected`), the
  runner demanding the CROSS-BACKEND AGREEMENT first (a divergence is
  red even against .expected — the oracle the fuzzer gets), the
  BLOCKING `conformance` CI job, the №535 cell +1
  (blocking_checks.tsv: `conformance|all|spec-pairs|blocks|
  n645_conformance_pairs_both_backends`) — 31 → **32**. Every norm was
  behaviorally probed on BOTH backends before recording. The two live
  findings recorded honestly (docs/spec «Honest limits», no silent
  fit): (а) composites in a boolean position DIVERGE (the TW as_bool
  refuses, the VM is_truthy answers) — NOT normed, a semantics-naryad
  candidate; (б) the VM `!=`-refusal wording names the internal Eq —
  the stable code identical, the wording a cosmetics candidate. The
  audit's candidate «Unit vs other → TYPE_MISMATCH» does not match the
  live twins (both answer Bool) — S-VAL-006 records the LIVE
  semantics.
- №646 (gh#1133, PR #1141 → `6eafee1c`) — the release lockstep 0.30.0
  (ONE PR, the №614/№583 образец): the workspace version + Cargo.lock
  ×5 + the badge (badge_sync.py) + the generated Version row
  (gen_metrics.py); the CHANGELOG cut `[0.30.0] - 2026-10-08` with the
  FULL [Unreleased] composition (### Security first); the
  release_gap_gate → **OK — nothing stuck** (the №629 gap CLOSES; the
  age-counting served: 1 day, no M-3); the fresh gate read
  `--office-tests pass --gate-target 0.30` → §4 GREEN ×4 + v2 GREEN
  (the parameterized 6043 vs the owner-fixed 5000: MET) → Overall
  GREEN, **`--strict` exit 0**; the tag **v0.30.0** on `6eafee1c`.
  The verdict line (п.6): the №629 fix carrier = **0.30.0** (no
  0.29.1; the recommendation recorded, the final verdict — the
  owner's). **THE PUBLICATION IS THE OWNER'S ACT** — the release is
  STAGED as of this record; the post-publication steps (the 4 assets /
  SBOM verification, REALITY STAGED → PUBLISHED, the №583 образец)
  close the naryad.
- №647 (gh#1134) — this sync, strictly last.
- №648 (gh#1135) — the DATED plan-relevance audit (release №4,
  2026-12-01 ± 1): verified-wait (the М2 §21 comment
  issuecomment-6050136217).

**The Wave 37 counters (every value from its machine, the command
reproduces it):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **309/516 = 5988 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | unchanged |
| Precise typed share (№560) | **218/516 = 4224 bp** | `… --precise --gate …` | unchanged (the Z-2 verdict stands) |
| Parameterized share (№623) | **55/91 = 6043 bp** — the 0.30 owner-fixed goal 5000 MET | `… --parameterized --gate …` | unchanged from В36 (№637) |
| Field-label share (№627) | **38/40 = 9500 bp** | `… --fieldmeta --gate …` | unchanged from В36 (№638) |
| **Sensitive-surface coverage (№644 — the NEW fixed-denominator metric)** | **46/138 = 3333 bp** (the floor 3333 only-up) | `python3 scripts/ci/sensitive_surface_share.py --gate scripts/ci/sensitive_surface_baseline.txt` | **new** (0 at the В36 record → 46/138 — the start value; the record line + the 0.31 DRAFT in gate_030_goals.txt) |
| `#[ignore]` debt (№468) | **11** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **15** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py --gate scripts/ci/src_mirror_baseline.txt` | unchanged |
| ADR count | **180** (the generated README row; ADR-0188 Proposed landed in В36) | `python3 scripts/gen_metrics.py --check` | unchanged |
| Blocking-checks cells | **32** | `python3 scripts/ci/blocking_checks_sync.py --count` | **moved** (31 → 32, №645 — the conformance surface, the explicit №535 growth) |
| The №562 release-gap gate | **OK — no pending Security entries** (the cut closed it) | `python3 scripts/ci/release_gap_gate.py` | **moved** (no-security-part → 1 day counted → OK after the cut) |
| The 0.30 gate posture | **owner_fixed: true; the DEFAULT target 0.30; the typed parameter MET (6043 ≥ 5000); the branch-protection criterion a DRAFT line (gh#1000 open)** | `python3 scripts/ci/unfreeze_gate.py --office-tests pass --gate-target 0.30 --strict` (exit 0) | **the strict release-time read GREEN** |
| The release state | **0.30.0 STAGED** (the tag v0.30.0 on `6eafee1c`; the publication = the owner's act, §6.5) | the release API + the tag | **moved** (0.29.0 PUBLISHED → the 0.30.0 lockstep landed, STAGED) |

**The owner gates remaining after В37:** the 0.30.0 PUBLICATION (the
owner's act — the Z-3 recommendation attached: the branch-protection
switches gh#1000 BEFORE publishing), the branch-protection closure
itself (then the fact-key micro-PR — №525), the К-Б stage-2 verdict
(the private first labels wait for it), the Камертон forwarding, the
the В36 tail (№639 the ledger revision 2026-10-15, №641 the В36 docs
sync + the gh#1089 closure), №648 (2026-12-01).

**The wave report line (№608/№470):** the dispatch surface gh#1136:
`naryad-quota (№470/№608): 16 naryads — domain 0/16 = 0.0% — PASS (the standing limiter <= 1/3)`

### 6.23. Wave 38 post-wave record (naryad №656): the parity-and-spec wave — the condition parity repaired (S-VAL-013), the sensitive surface moved (3333 → 3840 bp), the spec topic 2 landed (the probe's three honest divergences), the dictionary calibrated (class 4), the train and the showcase charters proposed (main @ `7036e8e5`, 2026-10-08)

Protocol: the same machines, no hand-written numbers. The honest
headline: В38 executed the ПСРМ development after the 0.30.0
publication (the dispatch gh#1151) — the №645-a finding (the
cross-backend condition divergence) is REPAIRED fail-closed (№651: the
VM condition path mirrors the TW `as_bool` 1:1 through the new
append-compat `JumpIfNotCond`; the norm S-VAL-013; the ### Security
entry — the №562 gate counts from the merge), the sensitive surface
made its first honest movement in the §6.3 audit order (№650: 7 of the
11 package rows closed against the verified handler facts; 4 rows stay
uncovered with the recorded reasons — the Гудхарт rule beats the draft
arithmetic), the spec topic 2 landed with its OWN probe yield (№652:
10 norms S-BLK-001..010 AND three new cross-backend divergences —
№652-a/b/c — recorded in the Honest limits, never silently fit), the
№643 measured gap closed (№653: the class-4 «silently-wrong»
vocabulary — the literal №629 fixture is caught now), and two charters
went Proposed for the owner (№654: the Monday release train; №655: the
examples showcase — three apps, the parity-as-display rule). The
publication of 0.30.0 happened at 06:55Z 2026-10-08 (the owner's act —
the §6.5 chain closed; the release-gap gate reads OK on the fresh
[Unreleased]).

- №651 (gh#1145, PR #1152 → `3ac294a3`) — the TW/VM condition parity
  (the №645-a repair, the №629 образец): the probe mapped EVERY
  condition-path use-site on both backends (the TW 8 `as_bool()` sites;
  the VM `is_truthy` in both `JumpIfNot` loops; the &&/|| already
  parity-twin — the №532 pair, untouched); the repair: the new
  `Instruction::JumpIfNotCond` (appended at the END of the enum — the
  bincode positional-index compatibility, the №264/№415 precedent) for
  the LANGUAGE condition paths (if / else-if / while / match guards —
  12 compiler sites + the patches), BOTH VM loops call the SAME
  `Value::as_bool` — the drift is structurally impossible; the TW
  `as_bool` refusal stamped `[TYPE_MISMATCH]` (the semantics
  unchanged — the codeification, not the behavior change). The
  escape-hatch check: no live dependency on the VM soft truthiness in
  conditions (the key suites green locally; the full suite — CI) →
  п.3 not applied, the behavior changed. The norm **S-VAL-013** (the
  condition truthy set = {true Bool, Float ≠ 0.0, non-empty String};
  Unit falsy; composites/opaques/Fluid refuse) + 3 conformance pairs
  (sval_013 composite/opaque/unit-falsy) + the №503 extension (the
  condition class on every pure value class). The ### Security entry:
  X-1/Y-1/Q-1, AFFECTS 0.30.0 AND EARLIER (VM backend), the fix
  2026-10-08 — the №562 gate counts (the №643 dictionary: PASS by the
  section).
- №650 (gh#1144, PR #1153 → `a854c2b0`) — the Q-2 movement №1 (the
  §6.3 audit order: БД → почта → контакты/календарь → request_body):
  7 of the 11 rows closed against the verified handler facts —
  imap_list/imap_search → `List<ImapMessage>`, imap_read →
  `Struct<ImapEmail>` with NINE well-formed field-labels (all
  untrusted: the IMAP server payload + the caller uid echo; the №627
  classification), cal_events/cal_read/card_contacts/card_search →
  `String` (the live returns ARE the JSON strings). The 4 HONEST
  refusals recorded with the reasons: query (the opaque `Value::Query`,
  the №538 posture), query_scalar (the heterogeneous scalar
  {Float|String|Unit}), query_row (the heterogeneous List — no single
  element type), request_body (the dynamic JsonBody fields — the №627
  dynamic-form posture). The probe opening: the field-meta section on
  the List ELEMENT is a stage-2 form (the Tool/DayForecast precedent —
  `from_path` refuses it at stage 0); the element labels recorded in
  the registry comments. **46/138 = 3333 bp → 53/138 = 3840 bp** (the
  floor raised in the SAME PR, №757); the 0.31 DRAFT (57/138 = 4130)
  is honestly NOT reachable on this package — the draft line NOT
  touched (№525). The floors that rose with the rows: general
  5988 → 6124, parameterized 6043 → 6170 (58/94 — the LS denominator
  91 → 94), fieldmeta 9500 → 9512 (39/41), precise 4224 → 4302
  (222/516); the in-tree locks TYPED_FLOOR 305 → 312, PARAM_FLOOR
  52 → 55, FIELDMETA_FLOOR 37 → 38; the fact twins synced across BOTH
  gate records (0.29 + 0.30 — the sync gate caught the 0.29 lag).
- №652 (gh#1146, PR #1154 → `a521432d`) — the spec topic 2 «Blocks and
  control flow» (strictly after №651): **10 norms S-BLK-001..010** —
  the block value, the empty block → Unit, if/else as an expression,
  the no-else arm → Unit, the while first test, the while count, the
  №622 return-capture, the match value (no match and no else → Unit),
  the sequential order, the lazy else-if (the List-condition detector:
  an unreached else-if never refuses) — each with the TW+VM anchors
  and the pair; docs/spec/README.md: the topic 2 row live, the
  normative core **13 + 10 = 23 norms**. THE PROBE YIELD: three
  cross-backend divergences found BEFORE writing, recorded in the
  topic's Honest limits as the candidate repair naryads — **№652-a**
  (the VM ignores `break`/`continue` inside a while body), **№652-b**
  (the trailing `let` in a value-channel arm: the VM keeps the last
  non-Unit value, the TW overwrites with Unit — the №370 vs the
  eval_statements contract), **№652-c** (the VM arm-let LEAKS into the
  enclosing scope — the TW clones the env, the №14 P0-3 precedent; the
  probe: `outer|inner` vs `inner|inner`). The №535 cell stays 32.
- №653 (gh#1147, PR #1155 → `55b95a6c`) — the №604 dictionary
  calibration (the №643 obligation): the **class 4** — a
  silent/quiet event in the ≤120-char neighborhood of a wrong-result
  word (false/wrong/incorrect/misleading/guard/branch/bypass/accept) —
  three patterns, the gh#1002 false-positive budget governing. The
  self-test flipped: the LITERAL №629 fixture outside `### Security`
  is CAUGHT now (exit 1) — the measured boundary closed; the window
  bound pinned (the silent/wrong pair further than 120 chars — no
  match). The retrospective: limitations.md 79 rows (3 class-4
  matches — all carrying their release-block evidence) + CHANGELOG
  1238 entries (27 matches: 6 under ### Security, 21 in the released
  sections, **0 in [Unreleased]**) → **IN SYNC, zero false positives
  on the clean rows**.
- №654 (gh#1148, PR #1156 → `9fb190bc`) — **ADR-0189 the release
  train** (Proposed — the cadence is the OWNER's §-answer): the Monday
  window (the first candidate 2026-10-12), the nine named window steps
  (the checklist section «The train»), the honest-content rule — a
  train without content does not depart (the empty-window skip without
  blame), the version-number semantics (Security/Fixed → the patch;
  Added/Changed → the minor), the release-block hold, the unscheduled
  security cut as the permitted exception (the 0.28.1 shape, the
  reason recorded).
- №655 (gh#1149, PR #1157 → `7036e8e5`) — **ADR-0190 the examples
  showcase charter** (Proposed — the placement and the app list are
  the OWNER's §-answers; the creation is the OWNER's act): three apps
  (the stateful serve application; the mlogpkg bot behind the
  deterministic stub; the small LLM agent on the mock circuit), each
  with the bounded subset, the e2e oracle and the CI contour; the
  showcase honesty rule: an app runs on BOTH backends on its subset —
  the parity as the display requirement (the №629/№651/№652
  motivation); the placement §-question: in-tree first (the executor's
  recommendation) vs the separate public repository.

**The Wave 38 counters (every value from its machine, the command
reproduces it; main @ 7036e8e5):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **316/516 = 6124 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | **moved** (309 → 316, №650) |
| Precise typed share (№560) | **222/516 = 4302 bp** | `… --precise --gate …` | **moved** (218 → 222, №650 — the four String rows) |
| Parameterized share (№623) | **58/94 = 6170 bp** (the LS denominator 91 → 94) | `… --parameterized --gate …` | **moved** (55/91 → 58/94, №650) |
| Field-label share (№627) | **39/41 = 9512 bp** | `… --fieldmeta --gate …` | **moved** (38/40 → 39/41, №650 — ImapEmail) |
| **Sensitive-surface coverage (№644)** | **53/138 = 3840 bp** (the floor 3840 only-up) | `python3 scripts/ci/sensitive_surface_share.py --gate scripts/ci/sensitive_surface_baseline.txt` | **moved** (46/138 → 53/138 — the №650 package) |
| The spec normative core (№645/№652) | **23 norms** (S-VAL ×13 + S-BLK ×10) with 45+10 conformance pairs | the pair count in `tests/conformance/`; the Topics table | **moved** (12 → 23 norms; the topic 2 live) |
| The №604 dictionary | **classes 1–4** (the class 4 added, №653) | `python3 scripts/ci/honest_boundary_check.py --self-test` (ALL PASS) | **moved** (the №643 boundary closed) |
| `#[ignore]` debt (№468) | **11** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged |
| `dead_code` (№468) | **15** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py --gate scripts/ci/src_mirror_baseline.txt` | unchanged |
| ADR count | **182** (the generated README row; 0189 + 0190 Proposed landed in В38) | `python3 scripts/gen_metrics.py --check` | **moved** (180 → 182) |
| Blocking-checks cells | **32** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged (the №535 rule — no growth without the explicit naryad) |
| The №562 release-gap gate | **OK** (the fresh [Unreleased] after the 0.30.0 cut) | `python3 scripts/ci/release_gap_gate.py` | unchanged (OK) |
| The 0.30 gate posture | **owner_fixed: true; the DEFAULT target 0.30; the typed parameter MET (6170 ≥ 5000); the branch-protection verdict fact GREEN** | `python3 scripts/ci/unfreeze_gate.py --office-tests pass --gate-target 0.30 --strict` (exit 0) | unchanged GREEN (the goal_parameterized_share_bp 5000 stays the owner's record) |
| The release state | **0.30.0 PUBLISHED** 2026-10-08 06:55Z (the owner's act — the §6.5 chain closed) | the release API + the tag | **moved** (STAGED → PUBLISHED at the wave start) |

**The owner gates remaining after В38:** the ADR-0189 §-answer (the
cadence acceptance), the ADR-0190 §-answers (the placement + the app
list; then the showcase wave), the К-Б stage-2 verdict (the private
first labels wait for it — the №650 boundary), the №652-a/b/c repair
naryads (the semantics — the owner's gate), the Camerton forwarding,
the В36 tail (№639 the ledger revision 2026-10-15, №641 the В36
docs + the gh#1089 closure), №648 (2026-12-01).

**The wave report line (№608/№470):** the dispatch surface gh#1151:
`naryad-quota (№470/№608): 21 naryads — domain 0/21 = 0.0% — PASS (the standing limiter <= 1/3)`

### 6.24. Wave 39 post-wave record (naryad №665): the verdict-execution + repair wave — the №652-a/b/c repairs landed (S-BLK-011..014), the 0.31 gate skeleton (ADR-0191), the К-Б stage-2 enforcement (the first private label), the ADR-0188 option (b) checker unreleased, the verdicts → Accepted, and the train №1 executed at the window — the 0.30.1 cut TAGGED, the publication pending (main @ `9f5cee7`, 2026-10-12)

Protocol: the same machines, no hand-written numbers. The honest
headline: В39 executed the owner's verdict session 08.10 (the dispatch
gh#1167) — the №652-a/b/c repair line from the В38 probe landed
fail-closed (№658: the VM break/continue parity — every loop jump
resolves from the loop-stack entry on BOTH backends, the stale-local
patch loops gone; №659: the trailing let in a value arm is Unit and
the arm lets do not leak — the S-BLK-011..014 norms ride with the
pairs), the 0.31 gate skeleton stood up (№660: ADR-0191 Proposed —
the verdict 08.10 verbatim core with the recorded divergence;
`gate_031_goals.txt` DRAFT carrying both OD variants; the 030 DRAFT
line retired — №525 discipline), the К-Б enforcement reached stage-2
(№661: the blocking field-meta rule — a parameterized Struct row
without a well-formed field-meta section fails CI; the FIRST PRIVATE
label — canary_check Struct<CanaryCheck>; the floors in the same PR),
the ADR-0188 option (b) compile-time checker landed as UNRELEASED
material (№662: the implicit String→opaque coercion and the unknown
template names refuse at CHECK time on the SHARED checker — both
backends, one diagnostics front door; the recursive dry-run inventory
370 .mlog files, 0 refusals; the [Unreleased] `### Changed` entry
rides to the 0.31.0 semantics — NOT in the 0.30.1 train), the
verdicts landed in the tree (№663: ADR-0189/0190 → Accepted; the
The-train checklist section + the train №1 activation), and the train
№1 was executed AT THE WINDOW 2026-10-12 (№664: the 0.30.1 lockstep —
the cut `[0.30.1] - 2026-10-12` with `### Security` №658+№651 (both
AFFECTS 0.30.0 AND EARLIER on the VM backend; no re-tag — the №620
precedent) then `### Fixed` #1169/#1170; the PR #1193 → `27877b8`; the
tag **v0.30.1** ON THE MERGE COMMIT — the tag push triggers nothing;
**the publication is the OWNER's act, §6.5 — PENDING**). The window
also executed В40's Фаза 0/1 (the completion audits №666/№667; the
merges №668/№669/№670 — the sensitive surface 54 → 66/138, the spec
topic 3 «Errors and the try result») — the full В40 record is §6.25
(naryad №671, strictly last); this section fixes the В39 story and
the window outcome that touches it (the train).

- №658 (gh#1159, PR #1168 → `3f9fee3`) — the №652-a repair (the
  №651/№629 образец): the VM loop path mirrors the TW `ControlFlow`
  contract 1:1 — every `break`/`continue` reaches its NEAREST
  enclosing loop on BOTH backends at any nesting depth; the shared
  statement compiler registers a `Jump(0)` placeholder in the NEAREST
  loop-stack entry and the loop compiler patches it from the POPPED
  entry (the pre-№658 root cause — the stale-local patch loops —
  gone). The X-1/Y-1/Q-1 silently-wrong control-flow class closes
  fail-closed; the TW unchanged; the norms S-BLK-011/012 + five
  conformance pairs pin the record; the ### Security entry rides the
  0.30.1 cut (the fix landed after the v0.30.0 tag `b4b9abb` — the
  published tag is affected on the VM backend).
- №659 (gh#1160, PR #1171 → `5c5a1a1`) — the №652-b/c repairs: the
  trailing `let` in a value arm evaluates to Unit (not the let-bound
  value) and the arm lets do not leak into the enclosing scope — both
  backends agree; the norms S-BLK-013/014 + the pairs; the
  docs/spec/blocks.md synced (the topic 2 counter 10 → 14 in the same
  lane with №658's norms — the spec README header 23 → 27 before the
  window's №670 moved it to 33).
- №660 (gh#1161, PR #1173 → `aea5cc5`) — **ADR-0191 the 0.31 gate
  skeleton** (Proposed): the verdict 08.10 verbatim core + the
  recorded divergence; `gate_031_goals.txt` DRAFT carrying BOTH
  OD variants (the owner's §-answers pending); the 030 DRAFT line
  retired (the 0.30 gate went owner_fixed at В34 — the DRAFT surface
  would be a shadow record).
- №661 (gh#1162, PR #1174 → `2da1e50`) — **the К-Б enforcement
  stage-2**: the field-meta rule is BLOCKING (the №637 anti-dilution
  grammar becomes a CI gate — a parameterized Struct row lands bare →
  red); the FIRST PRIVATE label: canary_check Struct<CanaryCheck> —
  the private-surface precedent the №650 boundary asked for; the
  floors in the same PR (typed 316 → 317, parameterized 58/94 → 59/95,
  fieldmeta 39/41 → 40/42, sensitive 53/138 → 54/138).
- №662 (gh#1163, PR #1175 → `ddca7ca`) — **ADR-0188 option (b)**: the
  compile-time port — the SHARED semantic checker refuses two
  late-failure classes at CHECK time on BOTH backends: (1) the
  IMPLICIT String→opaque coercion (a literal or a `+` chain flowing
  into an opaque-typed target — the explicit producer path stays
  legal, №114); (2) the unknown template names (a `render("Name", …)`
  whose statically-resolvable name is absent from the program's
  template declarations — only a string-literal name is statically
  resolvable, every non-literal form stays the runtime's backstop).
  The corpus is clean (370 .mlog files, 0 refusals); the diagnostics
  move left for NEW code only. **UNRELEASED** — the [Unreleased]
  `### Changed` entry; the 0.31.0 semantics carrier. The `#[ignore]`
  debt 11 → 9 rode this PR (the two runtime-500 waits retired by the
  check-time refusals).
- №663 (gh#1164, PR #1172 → `c1b1a72`) — the verdicts land in the
  tree: ADR-0189 (the release train) and ADR-0190 (the examples
  showcase) → **Accepted** (the owner's session 08.10); the
  release-checklist gains the The-train section; the train №1
  activation (the Monday window 2026-10-12).
- №664 (gh#1165, PR #1193 → `27877b8`) — **the train №1, executed at
  the window**: the 0.30.1 lockstep (one PR — the №646/№614 образец):
  workspace Cargo.toml 0.30.0 → 0.30.1, Cargo.lock ×5 members, the
  README badge + the generated Version row (gen_metrics --check OK);
  the cut `[0.30.1] - 2026-10-12`: `### Security` FIRST (№658 + №651)
  then `### Fixed` (#1169/#1170 — «rides the 0.30.1 train»); the
  [Unreleased] keeps `### Changed` (№662). The «only what landed»
  recheck on the window: the CHANGELOG delta a13e81c..bf65207 EMPTY
  (the hygiene/docs PRs added no [Unreleased] entries); the rebase
  onto the owner's grants-removal main (`e3acc04`) clean. The
  unfreeze gate `--strict` exit 0 (§4.1–4.4 GREEN; v2: parameterized
  6210 bp ≥ 5000 MET at the cut fact); the release-gap gate OK; CI
  48/48. **The tag v0.30.1 on the squash commit `27877b8`; the
  publication — the OWNER's act, §6.5, PENDING** (release_notes.py
  drafts from the [0.30.1] section; the v0.30.0 release shape is what
  release.yml builds).

**The Wave 39 counters (every value from its machine, the command
reproduces it; main @ 9f5cee7):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **326/516 = 6317 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | **moved** (316 → 317 №661 canary_check; the window merges №668 +5, №669 +4 — the В40 record §6.25) |
| Precise typed share (№560) | **222/516 = 4302 bp** | `… --precise --gate …` | unchanged (the X-3 rule — no scalars in the packages) |
| Parameterized share (№623) | **71/103 = 6893 bp** (the LS denominator 94 → 103) | `… --parameterized --gate …` | **moved** (58/94 → 59/95 №661; the window №668 → 67/99, №669 → 71/103 — §6.25; the goal 5000 stays the owner's record) |
| Field-label share (№627) | **44/46 = 9565 bp** | `… --parameterized --fieldmeta --gate …` | **moved** (39/41 → 40/42 №661 canary; the window №668 +2, №669 +2 — every parameterized Struct row WITH the field-meta, the anti-dilution rule) |
| **Sensitive-surface coverage (№644)** | **66/138 = 4782 bp** (the floor 4782 only-up) | `python3 scripts/ci/sensitive_surface_share.py --gate scripts/ci/sensitive_surface_baseline.txt` | **moved** (53/138 → 54/138 №661 canary; the window №668 +8, №669 +4 — §6.25) |
| The spec normative core (№645/№652) | **33 norms** (S-VAL ×13 + S-BLK ×14 + S-ERR ×6) with 40 conformance pairs | the pair count in `tests/conformance/`; the Topics table | **moved** (23 → 27 №658/№659 S-BLK-011..014; 27 → 33 the window №670 S-ERR-001..006 — §6.25) |
| The №604 dictionary | **classes 1–4** | `python3 scripts/ci/honest_boundary_check.py --self-test` (ALL PASS) | unchanged |
| `#[ignore]` debt (№468) | **9** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | **moved** (11 → 9 — №662: the two runtime-500 waits retired by the check-time refusals; the threshold 11 stands) |
| `dead_code` (№468) | **15** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py --gate scripts/ci/src_mirror_baseline.txt` | unchanged |
| ADR count | **183** (the generated README row; ADR-0191 Proposed landed in В39) | `python3 scripts/gen_metrics.py --check` | **moved** (182 → 183) |
| Blocking-checks cells | **32** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged (the №535 rule — no growth without the explicit naryad) |
| The №562 release-gap gate | **OK** (the fresh [Unreleased] carries only №662's Changed) | `python3 scripts/ci/release_gap_gate.py` | unchanged (OK) |
| The 0.30 gate posture | **owner_fixed: true; the DEFAULT target 0.30; the parameterized fact MET (6893 ≥ 5000); the branch-protection verdict fact GREEN** | `python3 scripts/ci/unfreeze_gate.py --office-tests pass --gate-target 0.30 --strict` (exit 0) | unchanged GREEN |
| The release state | **0.30.1 PUBLISHED** (2026-10-12 11:08Z) — the owner's act on the verbatim chat verdict («публикуй»): the release 410516037 on the tag v0.30.1 (`27877b8`), the body generated by release_notes.py from the [0.30.1] section (№497 — never hand-written), the release workflow green (run 38240781637), the four assets attached (binary, SBOM, BUILD-INFO, SHA256SUMS), the two Sigstore attestations signed; the syncing PR lands this record (ADR-0189 the train №1 step 8, the №583/PR #1059 pattern) | the release API + the tag + the release run 38240781637 | **moved** (TAGGED → PUBLISHED — the §6.5 chain closed) |

**The owner gates remaining after В39:** ~~the **v0.30.1
publication**~~ — **closed 2026-10-12** (the owner's verdict «публикуй»,
the release 410516037, the train №1 completed — see the release-state
row), the ADR-0191 §-answers (the OD variants — the
0.31 direction), the В40 tail (№671 the В40 docs record — strictly
last, after this section), the В36 tail (№639 the ledger revision
2026-10-15, №641 the В36 docs + the gh#1089 closure), №648
(2026-12-01), the showcase repository
creation (owner — then the showcase wave).

**The wave report line (№608/№470):** the dispatch surface gh#1167:
`naryad-quota (№470/№608): 24 naryads — domain 0/24 = 0.0% — PASS (the standing limiter <= 1/3)`

### 6.25. Wave 40 post-wave record (naryad №671): the FO-050 parity wave — the completion audits №666/№667, the Q-2 movement №2 «memory → recognition», the typing movement 0.31 №1 (forecast/graph) with the sequential recount, the spec topic 3 «Errors and the try result» (S-ERR-001..006), and the window: the 0.30.1 cut TAGGED (main @ `9f5cee7`, 2026-10-12)

Protocol: the same machines, no hand-written numbers. The honest
headline: В40 executed the owner's FO-050 bug-report line and the
types movement at the window (the dispatch gh#1187). The repairs'
content had landed BEFORE the wave (the owner's verbatim directive
«Выполни #1169 и #1170» — PR #1188, 2026-10-09); the wave verified and
closed it: **№666/№667 (gh#1181/gh#1182)** — the completion audits at
the window: the repair suite `issue_1169_1170_vm_boot_db` T1–T4 4/4
(the VM fresh-DB bootstrap replays the schema DDL eagerly at boot; the
db_url resolves ONCE in the Process context — no per-handler env()
reads), the regressions №259 7/7 · №758 5/5 · №403 4/4 · №457 5/5 ·
№399 4/4, `vm::` 14/14 — 0 failed; the classification `### Fixed`
(honest probe: availability/parity, not the vulnerability class); the
issues closed with the audit reports. **№668 (gh#1183, PR #1194 →
`1640984`)** — the Q-2 movement №2 «memory → recognition»: 8 of 9
gate_031 rows closed against the verified handler facts (the four
typed-coarse human_* rows parameterized; semantic_search/
mtree_retrieve typed; mtree_stats/mtree_summarize Structs WITH the
field-meta — the anti-dilution rule), the ninth (world_state) — the
honest OPAQUE WorldState handle (№355/ADR-0159; not a scalar, not a
parameterized List/Struct — the dictionary line is a separate owner
decision); sensitive 54/138 = 3913 → 62/138 = 4492 bp on its merge.
**№669 (gh#1184, PR #1195 → `b6300f8`)** — the typing movement 0.31
№1: the forecast/graph untyped package, 4 of 6 rows typed
(forecast_state Struct<ForecastState>{10 fields — the prov-block
metadata}, forecast_points Struct<ForecastPoints>{4 internal},
graph_neighbors List<Neighbor>, graph_query List<GraphEntry>), **2
honest refusals** (the №650 form): forecast_next — the opaque
ForecastHandle (the stage-0 OpaqueKind {Secret,Reflex,Grant,Series,
Media,Embodied} has NO Forecast kind; a fictitious spelling is
forbidden) and graph_path — the heterogeneous List (PathNode vs the
PathError singleton); **the sequential recount on the merged base**
(the branch sat on the pre-№668 main per the staging directive): the
machines re-read the live fact — sensitive 62+4 = **66/138 = 4782 bp**
(the phase-0 prediction held), typed 322+4 = **326/516 = 6317 bp**,
parameterized 67+4 = **71/103 = 6893 bp**, field-label 42+2 = **44/46
= 9565 bp**; the compiled pins TYPED_FLOOR 318 → 322, PARAM_FLOOR
64 → 68, PARAM_LS_DENOM 95 → 99, FIELDMETA_FLOOR 41 → 43; the
fact_parameterized_share_bp → 6893 (the gate-facts-sync twin). **№670
(gh#1185, PR #1196 → `9f5cee7`)** — the spec topic 3 «Errors and the
try result»: 6 norms S-ERR-001..006 pin the EXISTING try machinery on
BOTH backends per the №645 protocol (the TryResult shape, the caught
runtime refusal with the execution continuing, the TryError{code,
message}, the №385 classification contract — the whitelisted
position-0 origin stamp else the honest RUNTIME_ERROR, the
semantic-blocking boundary — try does NOT catch check-time refusals,
the first-class nested try); 7 conformance pairs ride the existing
`conformance (blocking)` job (the №535 rule); the probe found NO
cross-backend divergence (the shared-code parity by construction);
the Topics sync (the normative core 27 → 33). The window's headline
(the В39 naryad №664, cross-ref §6.24): the 0.30.1 lockstep PR #1193 →
`27877b8`, the tag **v0.30.1** ON THE MERGE COMMIT — the publication
is the OWNER's act, §6.5, PENDING. This naryad (№671) is the wave
tail — the dispatch gh#1187 closes with it.

- №666 (gh#1181, PR #1188 → `7874a28`) — the gh#1169 repair (the VM
  fresh-DB bootstrap): the schema DDL replays eagerly at boot (the
  fresh DB carries the schema BEFORE the first request — the
  interpreter parity); the unset variable is a LOUD BOOT error naming
  the variable and the remedy (was: a silent OK boot then per-request
  500s). Closed 2026-10-12 with the completion-audit report.
- №667 (gh#1182, PR #1188 → `7874a28`) — the gh#1170 repair (the
  db_url boot resolution): the URL resolves ONCE in the Process
  (boot) context on BOTH backends; the VM injects NO per-handler env()
  re-reads; the №259 serve-route env gate UNCHANGED (deny by default;
  ENV_NOT_PERMITTED — the frozen contract). Closed 2026-10-12 with
  the completion-audit report.
- №668 (gh#1183, PR #1194 → `1640984`) — the Q-2 movement №2 (the
  §6.3 audit order): 8 of 9 rows + the honest OPAQUE ninth; the
  floors in the same PR (typed 317 → 322 compiled 318, sensitive 54 →
  62/138, parameterized 59/95 → 67/99, fieldmeta 40/42 → 42/44); the
  test `naryad_668_memory_recognition` 5/5.
- №669 (gh#1184, PR #1195 → `b6300f8`) — the typing movement 0.31 №1:
  4 of 6 rows + 2 honest refusals; the sequential recount on the
  post-№668 base (66/138 = 4782 bp; 326/516; 71/103; 44/46); the
  test `naryad_669_forecast_graph` 6/6 (including the refusal pins).
- №670 (gh#1185, PR #1196 → `9f5cee7`) — the spec topic 3: 6 norms +
  7 pairs + the Topics sync (27 → 33); NO behavior change.
- №671 (gh#1186, PR #1198 → this merge) — the wave tail: §6.25 +
  PLAN-SUMMARY Wave 40 + the [Unreleased] Changed entries for the
  movement lanes; the dispatch gh#1187 closes.

**The FO-050 pilot status (honest):** the two VM serve-boot repairs
(#1169/#1170) are on main and ride the 0.30.1 tag; the office re-verify
waits the 0.30.1 artifact (the publication — the owner's act). The
pilot's `TeamSkillWired expects 2 args, got 0` first-request symptom
is NOT reproduced minimally and is NOT claimed fixed — it is expected
to disappear with the empty-schema state gone; the minimal-repro ask
stands in the gh#1169 thread.

**The Wave 40 counters (every value from its machine, the command
reproduces it; main @ 9f5cee7):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **326/516 = 6317 bp** | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | **moved** (317 → 326: №668 +5 — incl. the world_state OPAQUE handle typed, №669 +4) |
| Precise typed share (№560) | **222/516 = 4302 bp** | `… --precise --gate …` | unchanged (the X-3 rule — no scalars in the packages) |
| Parameterized share (№623) | **71/103 = 6893 bp** (the LS denominator 95 → 103) | `… --parameterized --gate …` | **moved** (59/95 → 71/103: №668 +8/95→99 — the four typed-coarse reparameterized + four new, №669 +4/99→103; the goal 5000 stays the owner's record) |
| Field-label share (№627) | **44/46 = 9565 bp** | `… --parameterized --fieldmeta --gate …` | **moved** (40/42 → 44/46: №668 mtree_stats/mtree_summarize, №669 forecast_state/forecast_points — all WITH the field-meta, the anti-dilution rule) |
| **Sensitive-surface coverage (№644)** | **66/138 = 4782 bp** (the floor 4782 only-up) | `python3 scripts/ci/sensitive_surface_share.py --gate scripts/ci/sensitive_surface_baseline.txt` | **moved** (54/138 → 66/138: №668 +8, №669 +4; the uncovered by reason: struct-no-meta 2, typed-coarse 3, untyped 67) |
| The spec normative core (№645/№652/№670) | **33 norms** (S-VAL ×13 + S-BLK ×14 + S-ERR ×6) with 40 conformance pairs | the pair count in `tests/conformance/`; the Topics table | **moved** (27 → 33: №670 the topic 3 live — the first spec topic recorded WITHOUT a divergence yield) |
| The №604 dictionary | **classes 1–4** | `python3 scripts/ci/honest_boundary_check.py --self-test` (ALL PASS) | unchanged |
| `#[ignore]` debt (№468) | **9** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | unchanged (the 11 → 9 drop was В39's №662 — §6.24) |
| `dead_code` (№468) | **15** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py --gate scripts/ci/src_mirror_baseline.txt` | unchanged |
| ADR count | **183** | `python3 scripts/gen_metrics.py --check` | unchanged |
| Blocking-checks cells | **32** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged (the №535 rule) |
| The №562 release-gap gate | **OK** | `python3 scripts/ci/release_gap_gate.py` | unchanged (OK) |
| The 0.30 gate posture | **owner_fixed: true; the DEFAULT target 0.30; the parameterized fact MET (6893 ≥ 5000); the branch-protection verdict fact GREEN** | `python3 scripts/ci/unfreeze_gate.py --office-tests pass --gate-target 0.30 --strict` (exit 0) | unchanged GREEN |
| The release state | **0.30.1 PUBLISHED** (2026-10-12 11:08Z) — the owner's act on the verbatim chat verdict («публикуй»): the release 410516037 on the tag v0.30.1 (`27877b8`), the body generated by release_notes.py from the [0.30.1] section (№497 — never hand-written), the release workflow green (run 38240781637), the four assets attached (binary, SBOM, BUILD-INFO, SHA256SUMS), the two Sigstore attestations signed; the syncing PR lands this record (ADR-0189 the train №1 step 8, the №583/PR #1059 pattern) | the release API + the tag + the release run 38240781637 | **moved** (TAGGED → PUBLISHED at the train №1 window) |

**The owner gates remaining after В40:** ~~the **v0.30.1
publication**~~ — **closed 2026-10-12** (the release 410516037, the
train №1 completed), the ADR-0191 §-answers (the
OD variants — the 0.31 direction; gh#1161), the train №2 candidate
(the Monday window 2026-10-19 — no empty trains, the ADR-0189 honest-
content rule), the examples showcase — the
repository is NOT created (the owner's act; the showcase is not
referenced as existing), the В36 tail (№639 the ledger revision
2026-10-15, №641) and №648 (2026-12-01) — outside В40, untouched.

**The wave report line (№608/№470):** the dispatch surface gh#1187:
`naryad-quota (№470/№608): 20 naryads — domain 0/20 = 0.0% — PASS (the standing limiter <= 1/3)`

### 6.26. Wave 41 post-wave record (naryad №677): the honest-boundaries-machine wave — BREAK_OUTSIDE_LOOP refuses on both backends (the №658-a/b REPAIRED), the Honest-limits spec scan mirrors the 659-x/y rows, the ignore floor 9 with the below-floor rule, and the §4 hygiene checklist with the porcelain gates that caught two real writers (main @ `b072748`, 2026-10-14)

Protocol: the same machines, no hand-written numbers. The honest
headline: В41 executed the owner's honest-boundary line (the dispatch
gh#1205) as ONE wave across four lanes plus the wave tail, and the
hygiene lane's new gate proved itself ON THE WAVE ITSELF — the first
full-suite porcelain run caught two committed repo-tree writers that
had sat in the tree for weeks. **№673 (gh#1200, PR #1247 →
`9b8cdc5`)** — the BREAK_OUTSIDE_LOOP semantic gate: the two
silent-wrong edges of the №658 probe (a bare break/continue OUTSIDE any
loop; a break/continue crossing the VALUE channel) now refuse BEFORE
execution on BOTH backends (the static codes
`BREAK_OUTSIDE_LOOP`/`CONTINUE_OUTSIDE_LOOP`, the traversal is
loop-stack-based: Each/EachWithIndex/While, transparent through
statement-if/match, the value channel = the Expr::BlockIfElse/Expr::MatchExpr
branches); the spec Honest-limits records №658-a/№658-b carry the
REPAIRED-by-№673 markers with the anchors. **№674 (gh#1201, PR #1248 →
`250ef31`)** — the honest-boundary machine sees the spec: the
Honest-limits mirror scan over `docs/spec/*.md` (every ACTIVE record
must have a limitations.md row by id, REPAIRED records are exempt; the
scan rides the retrospective mode of the SAME blocking job — the cell
does not grow), and the two still-open value-arm divergences
**№659-x/№659-y** land their №588-form limitation mirrors (the guard
scenarios named; the repairs stay a separate lane OUTSIDE В41). **№675
(gh#1202, PR #1249 → `8912c9d`)** — the ignore floor 11 → 9 (the
machine fact: the debt counter reads 9, the committed floor said 11)
PLUS the below-floor rule in the debt gate: a fact BELOW the floor is
now exit 1 naming the old/new floors and demanding the floor be lowered
in the same PR (the №662/№668 lecalo generalized). **№676 (gh#1203, PR
#1250 → `b072748`)** — the audit 7c3456b §4 hygiene checklist, all 11
rows: A3 the blocking `git status --porcelain` gates in test-lib AND
test-integration (no new job), D2 the README front-door (149 lines,
Quick Start at line 29, feature-gates → docs/), A7 the CHANGELOG
archive (727 KB → 57 KB; the releases 0.5.0→0.28.2 moved BYTE-EXACT to
docs/changelog/, 42 sections, machine-verified 46=46, the №588
Security records of the live releases stay in the root file), A1 the
frames/ leak class out of the tree (+ the two committed data/
artifacts), A6 the root at 8 .md files, A5 the GEMINI.md byte-duplicate
deduped, D3 the Newcomer Contract link-not-duplicate (the §3/§5 claims
stay README-pinned by the docs_consistency machine), D1 the generated
naryad-number stamp off + the grep floor in badge-sync, A2 the repo-
writing tests on the №387 sandbox form, C1 the MSRV claims synced
(1.93.1 everywhere), D5 the CI badge rendered from ci.yml (28 → 44,
the drift is now machine-caught). The gate's own harvest ON THE WAVE:
the full-suite porcelain caught `data/n402_ledger.jsonl` (the №402
route's ledger export — re-committed leak) and the masked
`data/n394_mcp_test_out.txt` (the tracked content equaled the write;
exposed the moment the leak was deleted) — both lanes now export into
throwaway `METALOGOS_DATA_DIR` roots, and the №337 GENERATE leg's
latent shared-scratch race with the CAPTURE leg was found and separated
(the №332 lesson applied). **№677 (gh#1204, this merge)** — the wave
tail: §6.26 + PLAN-SUMMARY Wave 41 + the [Unreleased] entries; the
dispatch gh#1205 closes.

**The debt-gate posture (the №675 fact):** the ignore counter reads 9
AT the floor 9 (the floor was lowered from 11 in the same PR that
proved the fact; the below-floor rule would refuse any future drop
without a same-PR floor edit); `dead_code` 15 unchanged.

**The Wave 41 counters (every value from its machine, the command
reproduces it; main @ `b072748`):**

| Counter | Value | Machine | Movement |
|---|---|---|---|
| Typed-signature share (№467) | **337/526 = 6406 bp** (the gate notes the floor 6317 should rise) | `python3 scripts/ci/type_signature_share.py --gate scripts/ci/type_signature_baseline.txt` | unchanged BY В41 (the §6.25 → b072748 delta 326/516 → 337/526 belongs to the PE-1/PE-2 lanes, merged between the snapshots) |
| Precise typed share (№560) | **227/526 = 4315 bp** | `… --precise --gate …` | unchanged BY В41 (the delta is the PE lanes') |
| Parameterized share (№623) | **77/109 = 7064 bp** | `… --parameterized --gate …` | unchanged BY В41 (the PE lanes' rows) |
| Field-label share (№627) | **46/48 = 9583 bp** | `… --parameterized --fieldmeta --gate …` | unchanged BY В41 (the PE lanes' rows) |
| **Sensitive-surface coverage (№644)** | **73/145 = 5034 bp** (only-up) | `python3 scripts/ci/sensitive_surface_share.py --gate scripts/ci/sensitive_surface_baseline.txt` | unchanged BY В41 (the PE lanes' surface) |
| The spec normative core (№645/№652/№670) | **33 norms** (S-VAL ×13 + S-BLK ×14 + S-ERR ×6) with 40 conformance pairs | the pair count in `tests/conformance/`; the Topics table | unchanged (В41 touched no norms) |
| The Honest-limits spec scan (№674) | **7 records (2 active, 5 REPAIRED-exempt) — IN SYNC** (every active record mirrored) | `python3 scripts/ci/honest_boundary_check.py --retrospective` | **NEW machine on В41** (the №674 scan rides the retrospective mode; the 659-x/y rows are the 2 active) |
| The semantic loop gates (№673) | **BREAK_OUTSIDE_LOOP / CONTINUE_OUTSIDE_LOOP** refuse before execution on both backends | the static codes + `tests/naryad_673_break_outside_loop.rs` (9 pins) | **NEW on В41** (the №658-a/b REPAIRED) |
| The №604 dictionary | **classes 1–4** | `python3 scripts/ci/honest_boundary_check.py --self-test` (ALL PASS) | unchanged |
| `#[ignore]` debt (№468) | **9 AT the floor 9** (TODO: 0) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` → DEBT GATE OK | **moved** (the floor 11 → 9 with the below-floor rule — №675; the counter value itself unchanged since №662) |
| `dead_code` (№468) | **15** | same gate | unchanged |
| TW/VM duplicated builtin names (№462) | **0** (the quorum 0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py --gate scripts/ci/src_mirror_baseline.txt` | unchanged |
| ADR count | **184** | `python3 scripts/gen_metrics.py --check` | unchanged BY В41 (+1 ADR-0192 — the PE-2 lane) |
| Blocking-checks cells | **32** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged (the №535 rule; the №676 gates are STEPS in existing jobs) |
| The №562 release-gap gate | **OK** | `python3 scripts/ci/release_gap_gate.py` | unchanged (OK) |
| The 0.30 gate posture | **owner_fixed: true; the DEFAULT target 0.30; --strict exit 0 GREEN** | `python3 scripts/ci/unfreeze_gate.py --office-tests pass --gate-target 0.30 --strict` | unchanged GREEN |
| The release state | **0.30.1 PUBLISHED** (2026-10-12 11:08Z, the release 410516037 on `27877b8`); the next window is the train №2 (Monday 2026-10-19): the [Unreleased] carries Added (the PE lanes, №674) + Fixed (№673, №684) → the §2.4 ADR-0189 classification = the 0.31.0 MINOR cut | the release API + the tag | unchanged (PUBLISHED; the train №2 is the owner's act) |

**The owner gates remaining after В41:** the ADR-0191 §-answers (the
OD variants — the 0.31 direction; gh#1161), the train №2 candidate (the
Monday window 2026-10-19 — no empty trains, the ADR-0189 honest-content
rule; the Security-факт: the live CHANGELOG root carries the
0.29.0/0.30.0/0.30.1 Security records, the archive preserved them
byte-exact), the В36 tail (№639 the ledger revision 2026-10-15, №641)
and №648 (2026-12-01) — outside В41, untouched.

**The wave report line (№608/№470):** the dispatch surface gh#1205:
`naryad-quota (№470/№608): 15 naryads — domain 0/15 = 0.0% — PASS (the standing limiter <= 1/3)`

### 6.27. Wave 42 post-wave record (naryad №683): the showcase wave — metalogos-examples in the separate public repository, the cross-repo lane parallel to the PE line (main @ `d403832`, 2026-10-13)

Protocol: the same machines, no hand-written numbers. The honest
headline: В42 executed the showcase charter (ADR-0190) in the OWNER's
separate public repository `ShkodnikAI/metalogos-examples` (created by
the owner; the wave's trigger 2026-10-13, the trace
1a1330e6f7a80a99: «metalogos-examples создал — сними блок лейна, волна
в реализацию»; the canon registration §16.9-тригесимус-секундус,
FOSVED-office-v2 `31e71ea`). The wave ran PARALLEL to the PE line
(the dispatch gh#1235 housekeeping: PE keeps the core-CI priority; В42
is the cross-repo lane) — the core source is untouched except this
docs record (the №683 exception), and the core counters below moved
only by the PE line, never by В42.

**What actually works on the showcase (the pinned core v0.30.1,
`27877b8` — the release binary, SHA256 + BUILD-INFO verified by the
showcase harness before every leg):**

- **№679 the scaffold** (gh#1230, showcase PR #1 → `fcd369a`) — the
  charter README (the three apps, the honesty rule §2.4, the pin
  policy §3(а) «pin + scheduled drift-check», the boundary with the
  core's developer-facing `examples/` and GFI #951), the verbatim
  licenses from the pin, the loudly-empty layout, the blocking
  both-backend CI + the weekly drift-check. The live premise held:
  the token's write grant was refused at first (403) — the no-progress
  flag was issued per the naryad's premise (no circumvention), the
  owner unlocked Contents: R&W, the same commit was pushed unchanged.
- **№680 App A — the stateful serve application** (gh#1231, showcase
  PR #2 → `e9155a3`) — `serve.mlog` 62 lines (the ≤120 budget checked
  in CI), the typed-memory arc (`memory_open`/`put`/`read`/`keys`);
  the restart arc **persist → query → restart → query is green on
  BOTH backends** (each a genuinely fresh process — the state
  survives through `METALOGOS_MEMORY_DB` + `METALOGOS_MEMORY_MASTER`,
  the load-on-open posture №351/ADR-0173 §3.5); the transcripts are
  byte-identical TW == VM (§2.4, `assert_identical`); the blocking CI
  + the nightly stress leg (5 notes × 3 restarts × both backends,
  parity diffed). The showcase's security-by-design lesson recorded:
  the first draft branched on a read value in a route — the boot
  audit refused it loudly (`UNTRUSTED_DECISION`, Category A); the
  shipped program branches nowhere on untrusted data.
- **№681 App B — the bot on mlogpkg** (gh#1232, showcase PR #3 →
  `bef13e6`) — an mlogpkg package (`mlog.toml`, `src/main.mlog` 117
  lines): the HOOK intake (`POST /hook`) + the POLL intake
  (`POST /enqueue` + `POST /poll` — the external world replaced by
  the deterministic queue), the per-chat dialog state machine
  (`new → greeted → helped`) over the typed-memory registry, the
  outbound `send_message` behind the deterministic `[AUDIT]` stub
  (offline); the hook+poll transcripts are byte-identical TW == VM;
  the honest boundary documented: the mlogpkg BINARY is not among the
  pinned release assets, so the leg exercises the surface the pin
  carries (the manifest shape + `mlog check` — the same
  `check_program` family `mlogpkg build` runs).

**The deviation from the registration (the loudest finding of the
wave):** **№682 App C — STOPPED no-progress** (the flag gh#1233,
comment 6146003277). The agent was built and runs green on the TW
(the model call under `METALOGOS_MOCK_LLM=1` — the CODE spelling,
`llm.rs`; the docs write `METALOGOS_LLM_MOCK` — the name divergence
is recorded — + the tool call + the visible trace through
`escape_html`, the §391 sanitizer), but the VM leg fails at COMPILE
time: **the VM compiler cannot resolve tool-method qualified calls**.
The minimal repro on the pinned binary: `mlog check` OK → TW run `42`
→ `mlog compile` fails `qualified call 'calc.double()' — not found`.
The localization: `src/compiler.rs` skips `Declaration::Tool(_)`
entirely (both arms), the `Expr::QualifiedCall` resolution has NO
tool-namespace branch (builtin.module → pattern → builtin only);
`src/interpreter/execution.rs` registers the tool namespaces
(ADR-0054) — that is why the TW works; the tool contract tests
(`tests/tool_abstraction_contract.rs`) are TW-only — the VM path was
never covered. Per the parity rule §2.4 (a divergence found in a
showcase app is the loudest possible finding — STOP, never adjust the
oracle): no pattern-call substitute, no TW-only demo, no VM-skip —
the divergent code did NOT enter the showcase; App C remains a loud
stub. The unlock paths (the conveyor's/owner's call): (a) a core
naryad — the VM compiler learns the tool-method resolution (register
`Declaration::Tool`, add the tool branch to the QualifiedCall
resolution, the VM leg in the tool contract suite), then App C lands
on the bumped pin; (b) a charter amendment for the App C bounded
subset (§2.3). №682 stays OPEN.

**The showcase gates after В42:** the pin bump carries the
both-backend proof (the core-pin.txt rule); the drift-check stays
non-blocking weekly; the showcase is NOT a GFI (№561) and adds no
language constructs (§4).

**The Wave 42 counters (the core machines on the base `d403832` —
the values moved by the parallel PE line are attributed to it, NOT to
В42; В42 moved nothing in the core except this docs record):**

| Counter | Value (the base `d403832`) | Machine | Movement vs §6.25 |
|---|---|---|---|
| Typed-signature share (№467) | **328/519 = 6319 bp** | `python3 scripts/ci/type_signature_share.py` | moved by the PE line (326/516 → 328/519) |
| Precise typed share (№560) | **224/519 = 4315 bp** | `… --precise` | moved by the PE line (222/516 → 224/519) |
| Parameterized share (№623) | **71/103 = 6893 bp** | `… --parameterized` | unchanged |
| Field-label share (№627) | **44/46 = 9565 bp** | `… --parameterized --fieldmeta` | unchanged |
| **Sensitive-surface coverage (№644)** | **67/139 = 4820 bp** | `python3 scripts/ci/sensitive_surface_share.py` | moved by the PE line (66/138 → 67/139; the floor raise note printed) |
| `#[ignore]` debt (№468) | **at or below the threshold** (the gate OK) | `python3 scripts/ci/debt_counters.py --gate …` | unchanged OK |
| TW/VM duplicated builtin names (№462) | **0** (0/8 groups) | `python3 scripts/ci/count_duplicated_names.py --quorum` | unchanged |
| Registered mirrors (№502/№564) | **6** | `python3 scripts/ci/mirror_counter.py --gate …` | unchanged |
| ADR count | **183** (the generated block up to date) | `python3 scripts/gen_metrics.py --check` | unchanged |
| Blocking-checks cells | **32** | `python3 scripts/ci/blocking_checks_sync.py --count` | unchanged (the №535 rule) |
| The №562 release-gap gate | **OK** | `python3 scripts/ci/release_gap_gate.py` | unchanged (OK) |
| The showcase state | **2 of 3 apps green + the finding** (App A, App B closed; App C the loud stub on the №682 flag) | the showcase CI runs on every PR/push | **moved** (the repository did not exist at §6.25) |

**The owner gates remaining after В42:** the №682 unlock decision
(the core naryad (a) or the charter amendment (b)), the ADR-0191
§-answers, the train №2 (the Monday window 2026-10-19), the В36 tail
(№639 15.10, №641), №648 (01.12) — outside В42, untouched.

**The wave report line (№608/№470):** the dispatch surface gh#1235:
`naryad-quota (№470/№608): 5 naryads — domain 0/5 = 0.0% — PASS (the standing limiter <= 1/3)`

### 6.28. Wave 36 post-wave record (naryad №641, the delayed tail): the parameterized movement №3 — the 0.30 owner-fixed goal REACHED, the field-labels №2, the ADR-0188 draft, and the dated ledger revision №639 executed at its window (the wave's merges `e7894a1e`/`48301708`/`bddc4cc5`, 2026-10-08…09; the record written on main @ `3c10534`, 2026-10-15)

Protocol: №414/№318/№576/№579/№589/№596/№619/№625/№636 — the same
machines, no hand-written numbers. The honest delay note: this record
is written by the wave's closing naryad №641 (gh#1124) at the №639
dated window (2026-10-15 ± 1 day), not right after the wave — the
§6.22–§6.27 slots were taken by В37–В42 in the meantime, so the record
takes the next free number instead of renumbering history. The
machine facts are the merge-time facts of the wave's PRs (replayed on
the execution-date main where the machine exists — the table below).

The honest headline: В36 executed the dispatch gh#1125 queue — the
parameterized movement 0.30 №3 **REACHED THE 0.30 OWNER-FIXED GOAL**
(55/91 = 6043 bp ≥ 5000 bp), the field-labels №2 landed (9500 bp =
95%), the ADR-0188 draft registered and **Accepted — option (б)** by
the owner's verdict (2026-10-08, gh#1123), and the dated ledger
revision №639 (gh#1122) executed at its window 2026-10-15: the ledger
gh#967 book-sync (the §3 RESOLVED mark restored — the findings were
implemented by №617 while the section lacked the mark; the §2 test
path fixed — the №567 crate split moved the file; the §4 park verdict
with the evidence table; the §7 partial resolution — the compile-time
half by №662), every lane's regression run GREEN, the debt gate OK.

- №637 (gh#1120, PR #1126 → `e7894a1e`) — the parameterized movement
  0.30 №3: the PDF contour, 21 verified rows (the 20 Struct rows WITH
  field_meta — the anti-dilution rule), the share 34/91 = 3736 bp →
  **55/91 = 6043 bp — THE 0.30 OWNER-FIXED GOAL (≥ 5000 bp) REACHED**
  (the floors raised in the same PR: the parameterized baseline 6043,
  PARAM_FLOOR 52).
- №638 (gh#1121, PR #1128 → `48301708`) — the field-labels №2: the 15
  existing parameterized Struct rows labeled per the №627 table read
  off the handlers, 5750 → **9500 bp (38/40 = 95%)** (FIELDMETA_FLOOR
  37).
- №640 (gh#1123, PR #1127 → `bddc4cc5`) — the ADR-0188 draft (the
  template-semantics composition, the ledger gh#967 §7 lane):
  Proposed, owner_fixed: false, no implementations, ADR 179 → 180;
  the owner's verdict 2026-10-08: **Accepted — option (б)**; the
  option (б) implementation: №662 (gh#1163, `ddca7ca`, В39).
- №639 (gh#1122, the dated tail) — EXECUTED 2026-10-15: the ledger
  gh#967 revision (the §3 book-sync, the §2 path fix, the §4 park,
  the §7 partial resolution); the regression runs 9/9, 8/8, 20/20,
  1/1, 4/4 GREEN; the closing act comment carries the marker
  `<!-- w36: ledger-revision -->`.
- №641 (gh#1124) — this record: the REALITY §6.28, the PLAN-SUMMARY
  Wave-36 line finalized (the early hand of №647 pre-landed it), the
  CHANGELOG Unreleased В36 entries (the registry surfaces moved by
  №637/№638; the ADR-0188 doc by №640), and the dispatch closures
  gh#1089 (the В34 housekeeping, the evidence list) + gh#1125 (the
  В36 dispatch).

| Surface | The value | Machine | Note |
|---|---|---|---|
| Parameterized share | **55/91 = 6043 bp — the 0.30 goal REACHED** | PR #1126 (the floors raised in-PR: baseline 6043, PARAM_FLOOR 52); the replay: `python3 scripts/ci/unfreeze_gate.py` | moved (3736 → 6043) |
| Field-label | **9500 bp (38/40 = 95%)** | PR #1128 (FIELDMETA_FLOOR 37) | moved (5750 → 9500) |
| ADR count | **180** (at the wave tail; 183 by the §6.27 date) | `python3 scripts/gen_metrics.py --check` | moved (179 → 180) |
| The debt gate | **GATE OK** (0/15/0/0 vs the live thresholds after №675) | `python3 scripts/ci/debt_counters.py --gate scripts/ci/debt_baseline.txt` | replayed 2026-10-15 |
| REFERENCE | **526/526 = 100.0%** | `python3 scripts/gen_reference_check.py` | replayed 2026-10-15 — the wave's PRs regenerated it in-lockstep |

**The owner gates remaining after В36 (the status on the record
date):** gh#1000 (the branch protection), the К-Б stage-2 enforcement
verdict (not filed), the Камертон forwarding, №632 (gh#1100 — the
plan-relevance audit release №3, 2026-11-01 ± 1 day), №648 (gh#1135 —
2026-12-01), №682 (gh#1233 — the App C landing: the owner's release
act → the pin bump). REMOVED from the gates by this record: the
NLnet-20.10 line (the grants line closed by the owner's verdict,
2026-10-12 — §6.24/§6.25), the parameterized movement to 5000 bp
(REACHED by №637), the В36 tail itself (№639 executed, №641 this
record).

**The wave report line (№608/№470):** the whole-dispatch surface
gh#1125 via `python3 scripts/ci/naryad_classes.py 1125 --line`
(2026-10-15):
`naryad-quota (№470/№608): 21 naryads — domain 2/21 = 9.5% — PASS (the standing limiter <= 1/3)`

### 6.29. The release record (naryad №692, issue #1268): the 0.32.0 publication — the FULL [Unreleased] cut, the version lockstep 0.30.1 → 0.32.0, the gate GREEN (the owner's verbatim chat verdict «Мы можем издавать релиз 0.32.0», 2026-10-15; main @ `08482d1`)

Protocol: №664/№646 (the release lockstep, ONE PR) + ADR-0189 step 8
(the №583/PR #1059 pattern). The window's queue was already merged
before the verdict: №641 (gh#1124 → #1261), №686 (gh#1253 → #1262),
№687 (gh#1254 → #1263), №689 (gh#1256 → #1264), №688 (gh#1255 →
#1265), №690 (gh#1257 → #1266), №691 (gh#1258 → #1267). The audit
gate «do not cut 0.31.0 before S-1» is satisfied by №685 (the marker
text rides inside the cut itself). The 0.31 number stays the GOAL name
(gate_031, the typing movement); the RELEASE number 0.32.0 is the
owner's verdict. The honest finding of the release: the MAIN push-runs
№690/№691 carried the honest-boundary red (the GH_TOKEN release-block
evidence step — the release-time human step BY DESIGN, №604/
ADR-0179 §6); the step-1 sync executed at the publication (the
release-block label on the linked issues #1084/#1096/#1041), the
retrospective with the token → IN SYNC, and the merge-commit push run
went GREEN on the check (the only red left is the coverage job, below).

- №692 (gh#1268, PR #1269 → `08482d1`) — the release lockstep: the
  FULL `[Unreleased]` cut → `[0.32.0] - 2026-10-15` (Security №689/
  №685/№673 + Added №688/№691/№674/№670/№640 + Changed №686/№687/
  №690/№676/№660 §5/№669/№668/№662/№637/№638 + Fixed №684; 19 entries,
  `[Unreleased]` EMPTY) + the version lockstep 0.30.1 → 0.32.0 (the
  workspace Cargo.toml ×1, Cargo.lock ×5 members, the README badge +
  table). The №688 draft (ADR-0193 Proposed, verdict pending) moved
  verbatim — NOT accepted by the release.

| Surface | The value | Machine | Note |
|---|---|---|---|
| The release state | **0.32.0 PUBLISHED** (2026-10-15) — the owner's act on the verbatim chat verdict («Мы можем издавать релиз 0.32.0»): the release **413675413** on the tag v0.32.0 (`08482d1`), the body generated by release_notes.py from the [0.32.0] section (№497 — never hand-written), the release workflow GREEN (run **38719993643**, event `release: published`), the four assets attached (the binary 35.2 MB, the SBOM `mlog-linux-x86_64.cdx.json`, BUILD-INFO, SHA256SUMS), the two Sigstore attestations signed (the green steps: *Attest build provenance* + *Attest SBOM (bound to the binary)*); the syncing PR lands this record (the №583/PR #1059 pattern) | the release API + the tag + the release run 38719993643 | **moved** (PUBLISHED — the №6.5 chain closed) |
| The strict gate | **EXIT 0** (`unfreeze_gate.py --office-tests pass --strict`, the DEFAULT target 0.31 per №660 §5) | the release-time read, replayed on `08482d1` | unchanged |
| The release-gap | **EXIT 0** (no pending Security in `[Unreleased]`) + self-test OK | `python3 scripts/ci/release_gap_gate.py` | unchanged |
| release-block open | **0** (the API count at the publication, the fresh token) | the API | unchanged |
| Honest-boundary | **IN SYNC** (82 rows + the changelog 61 entries + the spec mirrors 10 records) — the token retrospective | `GH_TOKEN=… python3 scripts/ci/honest_boundary_check.py --retrospective` | moved (the step-1 sync: #1084/#1096/#1041 labeled; the push-run red healed) |
| Local verification | build EXIT 0 (5/5 members at 0.32.0), fmt EXIT 0, clippy `-D warnings` EXIT 0, lib **870/870**, conformance runner 1/1, vm_golden 4/4, tool_abstraction 22+4, the wave-44 Security set №685 8/8 + №686 3/3 + №687 3/3 + №689 9/9 | the cargo/python runs on `883b913` | replayed at the staging |
| CI on the merge commit | **50/52** — the required contexts ALL GREEN; the ONLY red: coverage (non-blocking) | the checks API on `08482d1` | the honest finding below |
| **The coverage honest finding** | **the runner DISK EXHAUSTION** — `No space left on device (os error 28)` on the ACTIONS RUNNER (`target/llvm-cov-target`, `ld … signal 7 [Bus error]`), deterministic across two runs, INHERITED from main `e5f9f98` (the check `116305367127`): the wave 43–44 suite growth (pe2 n5–n9, the №685–№691 suites, the new conformance pairs) outgrew the runner disk under `--all-targets` × `--instrument-coverage`. NO code signal (the floor check never started). The follow-up наряд needed: the runner-disk fix (clean the llvm-cov target between phases / split the suite / `-C debuginfo=0`) | the job logs of `116359809068`/`116371686966` | new (documented on PR #1269) |

**The owner gates remaining after the publication (the status on the
record date):** gh#1000 (the branch protection), the К-Б stage-2
enforcement verdict (not filed), the Камертон forwarding, №632 (gh#1100
— 2026-11-01), №648 (gh#1135 — 2026-12-01), №682 (gh#1233 — the App C
landing), and the NEW follow-up from this record: the coverage
runner-disk fix (the honest finding above). REMOVED by this record:
the 0.32.0 publication itself (executed on the owner's verdict).

**Note (added 2026-10-22, naryad №768 — the external audit 21.10
§3 M-2, третий перенос):** the narrative line for the version line —
**0.31 was never released; its goal folded into 0.32.0.** The 0.31
number stayed the GOAL name (gate_031, the typing movement — the
protocol paragraph above), the RELEASE number 0.32.0 is the owner's
verdict; the tag line `v0.30.1` → `v0.32.0` carries no `v0.31.*`, and
the CHANGELOG [0.32.0] section carries the same note (added by the
same naryad — the historical sections gain a line, not a rewrite).


### 6.30. Wave 46 post-wave record (naryad №698): the safe-pipe + spec-tools + fixed-list wave — the ADR-0188 phase 2 landed (the safe stage `{{ param | safe }}` with the check-time Html typing; the ledger gh#967 §7 closed, №115 executed), the spec topic 6 «Tools and the abstraction» (S-TOOL-001..007), the 0.32 fixed-list movement №1 (9 honest closures; the sensitive floor 5034 → 5655 bp), and the showcase pin v0.32.0 + App C landed cross-repo (main @ `8088962`, 2026-10-16)

The wave executed by the OWNER's direct command («Исполни волну 46» —
this branch as the executor, the methodologist role §21 kept for the
acceptance form). The dispatch gh#1278 queue, all four code naryads
squash-merged with 49/49 blocking checks green EACH:

- **№694 (gh#1273 → showcase PR #4, squash `7b640966`)** — the showcase
  pin v0.30.1 → v0.32.0 in ONE PR with the both-backend proof of every
  app (the ADR-0190 §3(а) policy) + App C landed from `n682-app-c`
  (the honesty-boundary agent, 107/120 lines); the CI of the showcase
  green (SHA256 + BUILD-INFO + e2e both-backend 3/3); №682 (gh#1233)
  closed with the completion-audit; the dead branches deleted. ZERO
  edits to the core src (the naryad boundary).
- **№695 (gh#1274 → PR gh#1279, squash `40e7da6`)** — the ADR-0188
  phase 2: the pipe grammar v1 (ONE stage, `safe`) with the check-time
  Html typing — `SAFE_PIPE_NOT_HTML` refuses on BOTH backends (the
  №685 mirror for the escape-exempt slot), the runtime backstop
  requires the opaque `Value::Html`, the taint layer CONDUCTS through
  a safe slot (the html_label seeded map), the unrecognized forms
  refuse (`UNSAFE_PIPE_FORM` — the N9 refusal narrowed and shifted
  left); the classification lives in the ONE tokenizer
  (`template_scan::classify_pipe`/`pipe_scan` — the №685 discipline;
  the pest grammar untouched, the template bodies stay raw by design).
  The ledger gh#967 §7 lifted (the last ignore GREEN both-backend;
  the body patched to RESOLVED — all three lifts), №115 closes
  EXECUTED (the lineage: PR gh#74 → №662 → №695), the debt floor
  9 → 8 re-locked in the same PR (the №569/№675 procedure — the gate
  itself demanded it). Docs in the same PR: REFERENCE §5.7 + the
  render/template_render rows, the book syntax page, the CHANGELOG
  Added entry (NO Security classification — the honest probe: the pipe
  was a LOUD refusal before, nothing silently exploitable closed).
- **№696 (gh#1275 → PR gh#1280, squash `81a5cd5`)** — the spec topic 6
  «Tools and the abstraction»: 7 norms S-TOOL-001..007 over the live
  №684 contract, each with the per-backend anchors and a conformance
  pair (`stool_001..007`); the Topics table + the core counter synced
  (33 → 49 norms). The probe found NO cross-backend divergence; the
  honest finding (the topic's «Honest limits» + the PR flag): the
  negative-path tool refusals carry NO `[CODE]` stamps — the refusal
  norms are NOT recorded (the conformance contract requires a code for
  an error pair); landing the stamps is the №385 additive lane.
- **№697 (gh#1276 → PR gh#1281, squash `8088962`)** — the 0.32
  fixed-list movement №1: NINE honest closures over the frozen list
  (read_file_or, list_dir, voice_list, policy_check, http_download,
  stt_transcribe, omni_ask, ocr_extract, vision_understand — every
  signature from the LIVE handler, every Ok-site inventoried, each
  verified by a both-backend test `tests/naryad_697_type_closures.rs`);
  `--goal032` closed 9/67 (the report-only mode — the ADR-0193
  parameters stay the owner's); the PDn core stayed HONESTLY uncovered
  (the №650 refusal forms with the file:line facts: the opaque
  handles, the stub rows, the dynamic-shape Struct, the union return);
  the goals-facts №525 synced in the same PR.

The counters at the record date (the generators, NOT hands):
**ignore debt 8** (todo 0, dead_code 15 — the floor 8, №695),
**sensitive 82/145 = 5655 bp** (the floor 5655 only-up — №697; the
uncovered by reason: struct-no-meta 2, typed-coarse 3, untyped 58),
**goal032 closed 9/67** (report-only), **typed signatures 346/526 =
65.77% (precise 234/526 = 44.48%)** (the README generated block),
**norms 49** (13+14+6+6+3+7) with **59 conformance pairs**,
**mirrors 6, dups 0** (no new mirror: the pipe logic is single-home),
**[Unreleased] carries the №695 Added** — the 19.10 train window can
cut (the §2.4 ADR-0189 rule; the publication is the owner's act).

The owner gates remaining (unchanged + the wave's flags): the
ADR-0193 parameters (≥20/67, the PDn priority) + the «0.32 release»
label RE-ANCHOR (v0.32.0 published 15.10 — the label is dead; the
next minor window), the coverage threshold decision (№569, ≥76% line),
the second maintainer, the №385 additive lane for the tool refusal
codes (the №696 finding), the train window 19.10, NLnet/Restack M1
(03.11.2026), №648 (01.12.2026).

### 6.31. Wave 47 movement №2 record (naryad №702): the PDn core of the 0.32 fixed list — TWO honest closures (conv_context → String, conv_history → List<Message>), EIGHT anchored refusals — the backend-divergent query contract surfaced (main @ the naryad's PR, 2026-10-16)

The movement №2 over the ADR-0193 frozen list
(`scripts/ci/gate_032_untyped_sources.txt`, 67 ids) closed the
conversation core of the PDn set: **conv_context → String** (the
"{role}: {text}" injection format — deterministic on BOTH backends,
session_ops.rs) and **conv_history → List<Message>** (the fixed
Message{role, text, timestamp} shape, session_ops.rs:198-209). Both
rows carry the fail-closed empty fallbacks for the context-less lanes
(the authenticate mock-handler form; the query_param №565 value form)
— the real dispatch stays in session_ops behind the №72 TW/VM
intercepts. The both-backend proof lives in
`tests/naryad_702_type_closures.rs` (9 tests, the лекало
tests/naryad_697_type_closures.rs).

The EIGHT honest refusals — each with its live anchor (the №650 form):
**query** — the BACKEND-DIVERGENT contract (TW mutations answer the
affected-row count as a String, db_ops.rs:200-204; the VM builds rows
unconditionally and a mutation yields the EMPTY list, db_ops.rs:699+ —
the №538 comment verbatim: "the EXECUTION result type depends on the
backend"; a unification is a language-design decision, the owner's
gate), **query_row** — the element type is data-dependent by column
type (Null→Unit, Integer/Real→Float, Text/Blob→String;
db_ops.rs:541-557), **query_scalar** — the value type is
data-dependent (numeric and text columns verified live),
**stdin** — the true stub (no handler anywhere; UNDEFINED_FUNCTION on
both backends), **request_body** — the dynamic-shape
Struct<JsonBody> (http.rs:469-476; the №661 field-meta armor refuses
the static labeling), **secret** — the opaque Value::Secret behind the
SECRET_LEAK Category-A contract (to_string renders the masked
"[Secret]"), **memory_open** — the opaque Memory handle behind the
consent gate (MEMORY_CONSENT_REQUIRED live on both backends; №539),
**memory_read** — String|Secret by the container label (typing String
would false-warn the private lane — the in-tree comment verbatim).
The frozen list is NOT edited.

The counters at the record date (the generators, NOT hands):
**sensitive 84/145 = 5793 bp** (the floor 5655 → 5793 in the SAME PR —
№757/№613; the uncovered by reason: struct-no-meta 2, typed-coarse 3,
untyped 56), **goal032 closed 11/67** (report-only, GREEN — the
machine verdict; the ≥20 figure stays the owner's Proposed
parameter), **typed signatures 348/526 = 66.15% (precise 235/526 =
44.67%)** (the README generated block), **parameterized share 80/112
= 7142 bp** (the gate_030 fact twin), **[Unreleased] untouched** (the
typing is internal — the №697 precedent; the 19.10 window keeps the
№695 Added). The post-wave wave-record (§6.31 completion) lands with
naryad №704 (the strictly-last sync).

### 6.32. Wave 47 post-wave record (naryad №704): the hygiene-and-prep wave — the dependabot sweep 11/11, the movement №2 merged, the M1 traction-refresh v2, the 0.33.0 lockstep in verified-wait (main @ the naryad's PR, 2026-10-16)

The wave 47 (dispatch gh#1299) landed the hygiene and the prep:

- **№700** (gh#1294): the dependabot sweep **11/11** — 7 squash merges
  (upload-artifact 6→7, hyper 1.11.1→1.12.0, proptest 1.11→1.12,
  zeroize 1.9.0→1.9.1, serde_json 1.0.151→1.0.154, sha2 0.11.0→0.11.1,
  pest_derive 2.9.2→2.9.3), the toml 1.1.6+spec→1.1.8+spec patch landed
  by **gh#1303** (`30ae37e9d` — the gh#1289 content; dependabot closed
  its PR as obsolete and did not honor the recreate), and the two
  parser majors **closed with honest reports** (gh#1286 xml5ever 0.40,
  gh#1288 html5ever 0.40.1: the build breaks on the markup5ever
  0.12.1/0.40 ecosystem split — `RcDom` implements the old `TreeSink`;
  crates.io has no rcdom release for markup5ever 0.40, the latest is
  `0.39.0+unofficial` pinned to `^0.39` — the migration is a future
  dedicated naryad, the owner's flag). pest rides 2.9.4 via the caret
  spec. The ratchet: the cell №535 does not grow, the debt 8/0/15 does
  not grow, the sensitive floor never dipped.
- **№702** (gh#1296, **gh#1300** `da03a2a85`): the movement №2 over
  the frozen list — the §6.31 record (2 honest closures, 8 anchored
  refusals, the floor 5655 → 5793 bp in the same PR).
- **№703** (gh#1297, closed 16.10): the M1 traction-refresh №2 — the
  package v2 landed in `ShkodnikAI/metalogos-grants` (the cross-repo
  home; the generator numbers on the fresh main; honest claims; the
  publications stay the owner's).
- **№701** (gh#1295, **gh#1304**): the 19.10 train prep — the
  `[0.33.0] - 2026-10-19` cut (the single №695 Added), the version
  lockstep ×5, the badge + the generated row; the fresh gate read
  `--gate-target 0.33.0 --strict` **EXIT 0** (4.1–4.4 GREEN; the twin
  4/4), `release_gap_gate` EXIT 0. **Verified-wait**: the merge and
  the publication await the owner's go/no-go verdict (§6.5/ADR-0186
  §4.5) — no tag exists at the record date; the record does NOT count
  a release.

The counters at the record date (the generators, NOT hands):
**ignore debt 8** (todo 0, dead_code 15), **sensitive 84/145 =
5793 bp** (the floor 5793 only-up; the uncovered by reason:
struct-no-meta 2, typed-coarse 3, untyped 56), **goal032 closed
11/67** (report-only; the ≥20 figure stays the owner's Proposed
parameter), **typed signatures 348/526 = 66.15% (precise 235/526 =
44.67%)** (the README generated block), **parameterized share 80/112
= 7142 bp**, **norms 49 with 59 conformance pairs**, **mirrors 6,
dups 0**, **[Unreleased] on main carries the №695 Added** — the open
lockstep PR gh#1304 cuts it to `[0.33.0] - 2026-10-19` pending the
owner's verdict.

The owner gates remaining (unchanged + the wave's flags): the
ADR-0193 parameters (≥20/67, the PDn priority) + the «0.32 release»
label RE-ANCHOR, the coverage threshold (№569), the second maintainer,
the 19.10 train verdict (gh#1304), NLnet/Restack M1 (03.11.2026),
№648 (01.12.2026), and the wave's new flag: the **parser-migration
naryad candidate** (html5ever/xml5ever → the 0.39+unofficial pin
line, or the rcdom 0.40 wait — the №700 audit).

### 6.33. Wave 48 post-wave record (naryad №709): the movement №3 merged — EIGHT honest closures, the floor 5793 → 6344 bp, the extraction family on the markup5ever 0.39 line, the conveyor zip-audit enablement, the 0.33.1 lockstep in verified-wait (main @ the naryad's PR, 2026-10-17)

The wave 48 (dispatch gh#1310) landed the development:

- **№706** (gh#1306, **gh#1313** squash `79901442`; the successor of
  gh#1311 after the conveyor rebase): the movement №3 over the frozen
  list — **EIGHT honest closures** by the №644 criterion:
  `recall_top_k → String` (BOTH backend intercepts serialize the JSON
  hit array; the fail-closed no-memory fallback `[]`, the №702 form),
  `video_understand → String` (the mock-golden success path, the
  omni_ask №697 posture), and the six parameterized Struct rows WITH
  the well-formed field-meta — `media_manifest_read`
  (6 provenance fields, all untrusted), `cron_list` (15 fields;
  prompt/payload **private** per the №689 mail form, the scheduling
  metadata internal per the goal_get store-read form, the optional
  Float|Unit fields per the memory_revise precedent), `ledger_verify`
  (7 verdict fields — BOTH arms one shape), `proof_verify` (8 report
  fields), `device_state` (4 fields, the optional bounds_digest),
  `replay_snapshot` (the snapshot echo → untrusted). The movement №3
  obligation: the PDn core re-verified LIVE — the backend-divergent
  query contract is ALIVE (TW db_ops.rs:133/202-205 vs VM
  db_ops.rs:699+; the `ONE contract` comments belong to db_execute),
  the 8 refusals of №702 hold, +3 fresh re-reads refused
  (generate_key — the opaque Secret; backend_select — the
  heterogeneous arms; config_load — the dynamic shape). The ratchet:
  the floor **5793 → 6344 bp** in the same PR (92/145; the machine's
  own note honored), the twins gate_029 **4505** / gate_030 **4505 +
  7288** / gate_031 **6344**; goal032 **11 → 19/67** (report-only; the
  ≥20 figure stays the owner's Proposed parameter — ONE closure away).
- **№707** (gh#1307, **gh#1314** squash `81edb389`): the extraction
  family moved to the **markup5ever 0.39 line** — html5ever
  0.27 → 0.39, xml5ever 0.18 → 0.39, markup5ever_rcdom
  0.3 → 0.39.0+unofficial (the newest COMPILABLE pair: no rcdom
  release under markup5ever 0.40 — the №700 reports re-verified;
  the 0.39 members unify on ONE markup5ever 0.39 graph — `cargo tree`).
  The RcDom/TreeSink integration needed NO rework (the surface is
  stable); the features stay off-by-default, DTD NEVER; n5 14/14 +
  n6 14/14 both backends, the smoke, the conformance, the leak-suite,
  the №465 diff-fuzzer, the crosscheck — green. The CHANGELOG
  `[Unreleased]` **Changed** entry landed (the wave's single row).
- **The conveyor enablement** (PR **gh#1312** squash `f3c3c288`, the
  PR#1301 form): the fresh RustSec advisory set (2026-10-10) fired
  THREE criticals on the transitive zip 8.6.0 — the fix is
  upstream-blocked (candle-core 0.11.0 pins `zip ^8.6.0`, no zip 8.x
  patch exists, the path unreachable in the default build); the
  documented `--ignore` trio landed in deny.toml + ci.yml WITH the
  reason and the removal trigger (the first candle-core zip-^9
  release; the review 2026-11-17) — the №459 removal form.
- **№708** (gh#1308, **gh#1315**): the 26.10 train prep — the
  classification §2.5 (the single №707 Changed → the PATCH proposal
  **0.33.1**), the lockstep ×5, the badge + the generated row, the
  `[0.33.1] - 2026-10-26` cut; the gates: `--gate-target 0.33.1
  --strict` **EXIT 0** (the twin 4/4 — the №701 discipline),
  `release_gap_gate` EXIT 0. **The owner merged the lockstep** (the
  owner's account, 2026-10-17T05:44Z, `e526a8bb`) — the `[0.33.1]`
  cut is ON MAIN; the tag `v0.33.1` + the publication await the
  owner's train-day act (the 26.10 window). The 0.33.0 publish
  itself LANDED — **PUBLISHED 2026-10-19T03:39:46Z** (the owner's
  explicit in-chat delegation trace `1a1523c80479edcd`: the draft id
  415523172 → the release event; the run #18 `39195749095` success —
  the 4 assets rebuilt + re-attested from the tag `474ff7ae7`, the
  binary sha256 `d1be65b6…` unchanged, the SHA256SUMS / SBOM /
  attestations re-verified ALL GREEN; the post-release sync, the
  №646 form).

The counters at the record date (the generators, NOT hands):
**ignore debt 8** (todo 0, dead_code 15), **sensitive 92/145 = 6344
bp** (the floor 6344 only-up; the uncovered by reason: struct-no-meta
2, typed-coarse 3, untyped 48), **goal032 closed 19/67** (report-only),
**typed signatures 356/526 = 67.68% (precise 237/526 = 45.05%)**, the
**parameterized share 86/118 = 7288 bp**, the **field-label share
53/55**, **mirrors 0 mentions, dups 0**, **[Unreleased] on main is EMPTY** (the
№708 lockstep `e526a8bb` cut the №707 Changed to
`[0.33.1] - 2026-10-26`; the next content comes only from the merged
naryads).

The owner gates remaining (unchanged + the wave's flags): the
ADR-0193 parameters (≥20/67 — the movement №3 delivered 19, the
figure is ONE honest closure away; the PDn priority), the coverage
threshold (№569), the second maintainer, the 19.10 publish verdict RESOLVED
(0.33.0 PUBLISHED 2026-10-19T03:39:46Z) + the 26.10 train verdict
(gh#1315), the
zip-ignore review (2026-11-17 or the first candle zip-^9 release),
the query-divergence language-design gate (the owner's), NLnet/Restack
M1 (03.11.2026), №648 (01.12.2026), and the infra flag: the executor
PAT carries NO git-push transport (the API-write path works — the
publish lane runs through the git-data API).

### 6.34. Wave 49 post-wave record (naryad №714): the serve secret-comparison contract resolved — the gh#1317 refutation + the loud recipe, the movement №4 — goal032 19 → 20/67 (the ≥20 figure REACHED as a machine fact), the floor 6344 → 6551 bp, the query-parity ADR-0194 Proposed, the 26.10 train prep in verified-wait (main @ the naryad's PR, 2026-10-17)

The wave 49 (dispatch gh#1322) landed the development — the supplement
first (the production bug), then the parallel pair, the prep, the sync:

- **№716** (gh#1323, **gh#1324** squash `86d7d0b`; the ДОВЕСОК of the
  wave, the В40 form — the owner's bug report gh#1317 turned into the
  наряд by the §16.0-1 canon edit): the honest probe on the release
  binary v0.32.0 (sha `dd8164b7…` verified) **REFUTED the inbound-path
  hypothesis** — serve delivers header/query bytes UNCHANGED (the
  raw-decision block is the intended Category-A `UNTRUSTED_DECISION`
  integrity gate); the alterer is the program's own `redact(x, "all")`
  — its entropy net (`src/builtins/string.rs:1373-1398` +
  `re_b64_run` `:1239-1242`, the ≥24 base64-class runs with a digit ∧
  a hex letter) masks token-like values into
  `[REDACTED:entropy…last4]`; the gh#1317 table is explained **7/7**
  and golden-locked (`tests/n716_serve_secret_compare.rs` 9/9 both
  backends — the table, the sanctioned `hash_only`-BOTH-sides recipe
  comparability, the trap shape); the `UNTRUSTED_DECISION` refusal
  now NAMES the recipe (the №327 substring locks green); the contract
  documented (`limitations.md` § Serve Secret Comparison); the
  `[Unreleased]` Changed row with the honest AFFECTS. **AFFECTS:
  none — no language bug; 0.33.0 NOT affected** (the owner's
  remaining verdicts: gh#1317 (open), the office #536 fix (the
  hash_only recipe, the office-side change), the §2.5 ride of the
  `[Unreleased]` row at the 19.10/26.10 windows).
- **№712** (gh#1319, **gh#1325** squash `1ad8772`): the movement №4 —
  **THREE honest closures** by the №644 criterion:
  `duplex_state → Struct<DuplexState>` (6 fields; id → untrusted, the
  stream projections → internal; the OPTIONAL Struct|Unit slots —
  the memory_revise precedent) — **the FROZEN-LIST closure:
  goal032 19 → 20/67, the ≥20 figure of ADR-0193 REACHED as a
  machine fact** (the parameter stays the owner's verdict —
  report-only, no self-capture), `read_file_tokens →
  Struct<FileInfo>` (content/chars/tokens all untrusted — the
  length-oracle conservatism, the gh#1317 class), `user_profile →
  Struct<UserProfile>` (container untrusted, count internal, the
  KV content private — the №689 form). The ratchet: the floor
  **6344 → 6551 bp** in the same PR (95/145; the machine's own note
  honored), the twins gate_030 **7288 → 7478** (89/119) / gate_031
  **6551**, precise **unchanged 237/526** (no scalar closures —
  gate_029 untouched); the ПДн refusals re-verified with FRESH
  anchors (query → the №711 ADR-0194 pin; secret/generate_key — the
  no-Secret-spelling dictionary lane; request_body — the dynamic
  JsonBody fields; stdin — the true stub; session_poll_wake /
  session_take_interrupt — the №543 unions; recipe_list /
  recipe_search — the always-empty List; query_row/query_scalar —
  the data-dependent stubs; memory_open/memory_read — the №539
  anchors hold). The CI-infra guards landed with the PR (zero check
  weakened — the №445 posture): the kv fixture moved to the temp
  sandbox (the №676 A3 rule) and the runner **ENOSPC guard** in
  test-integration (the hosted runner disk stopped fitting the
  workspace build — two deterministic `os error 28` runs at the
  libmetalogos rlib archive; the preinstalled toolchain junk freed
  before the build).
- **№711** (gh#1318, **gh#1326** squash `16562e5`): the query-parity
  decision package — the live probe record (the mutation divergence
  LIVE: TW `n=3` / VM `n=[]`; the SELECT parity and the
  duplicate-column probes identical; the error paths verified
  practically unreachable — `get_ref` errs only on
  `InvalidColumnIndex`, in-bounds by construction, the TW closure
  cannot fail, the row-step errors need OS/IO faults — **the STOP
  trigger did NOT fire**); **ADR-0194 (Proposed)** with the two
  decision packages (A — the error-path direction, the
  recommendation (c) the loud-fail contract on BOTH lanes; B — the
  query result contract, the recommendation (i) String-count on
  both, the №474 mirror); the honest-limits row in the TW/VM
  Divergences section; the candidate comment db_ops.rs:690-698
  **PINNED** (the line count preserved — the anchors hold); NO
  divergence-pinning test (the №476 rule); **zero behavior change**
  — the OWNER's verbatim on the packages A/B is the gate.
- **№713** (gh#1320, **gh#1327**): the 26.10 train prep — the notes
  draft checked in (`docs/release-notes-0.33.1-draft.md`, the №497
  verbatim construction + the regenerate-at-the-window rule — a
  stale draft is a lie), the gates **GREEN** (`unfreeze
  --gate-target 0.33.1 --office-tests pass` — the twin 4/4, the
  №701 discipline → Overall GREEN EXIT 0; `release_gap_gate`
  EXIT 0), the prep contour verified (the triggers / the runner
  `ubuntu-24.04` / BUILD-INFO = the checkout ref = the future tag
  commit); the go/no-go filed: **GO 26.10** with the §2.5 preview
  (the №716 `[Unreleased]` row's ride — the owner's call). The tag
  `v0.33.1` + the publication await the owner's train-day act.

The counters at the record date (the generators, NOT hands):
**ignore debt 8** (todo 0, dead_code 15), **sensitive 95/145 = 6551
bp** (the floor 6551 only-up; the uncovered by reason:
struct-no-meta 2, typed-coarse 1, untyped 47), **goal032 closed
20/67** (report-only; the ≥20 figure reached — the owner's verdict
prepares), **typed signatures 357/526 = 67.87% (precise 237/526 =
45.05%)**, the **parameterized share 89/119 = 7478 bp**, the
**field-label share 56/58**, **mirrors 0 mentions, dups 0**,
**[Unreleased] on main carries the №716 Changed row** (the merged
naryads only — the №687 rule; the ride is the owner's §2.5 call).

The owner gates remaining (unchanged + the wave's flags): the
gh#1317
resolution verdict + the office #536 fix decision; the 19.10 publish
verdict RESOLVED (0.33.0 PUBLISHED 2026-10-19T03:39:46Z); the ADR-0193
parameters (≥20/67 reached — the verdict fixes the figure and the
gate wiring); the **ADR-0194 verbatim** (the packages A/B — the
query error paths + the result contract); the 26.10 train verdict
(gh#1327); the zip-ignore review (2026-11-17 or the first candle
zip-^9 release); the coverage threshold (№569); the second
maintainer; NLnet/Restack M1 (03.11.2026); №648 (01.12.2026); the
PAT rotation (the exposed-token discipline); the draft v0.4.0
cleanup (the owner's act).

### 6.35. Wave 50 post-wave record (naryad №720): the 0.33.1 publication — the full train ride, the soak ENOSPC guard (the В48 lesson), the fixed-list movement №5 — goal032 20 → 23/67 with the gate WIRED BLOCKING (ADR-0193 Accepted by the owner's live verbatim), the floor 6551 → 6758 bp, the rcdom metadata line hygiene, the good-first pool executed one — #956 (main @ the naryad's PR, 2026-10-19)

The wave 50 (dispatch gh#1333) landed the development — the movement,
the verdict package, the publication, the infra guard, the wiring, the
hygiene, the pool execution, the sync:

- **№717** (gh#1329, squash `58cecc8`): the movement №5 — THREE honest
  closures: `listen_start → Struct<DuplexFlow>` (stream_id/direction/
  preempted internal, priority untrusted — duplex.rs:122 the SINGLE Ok
  arm), `llm_stream_open → Struct<LlmStream>` (handle/model/provider
  internal — llm_stream.rs:59 the SINGLE Ok arm), `vision_fetch_weights
  → String` (the dest_dir echo, the allowlist default-deny + SSRF +
  pickle refusals stay loud) — **the FROZEN-LIST closure: goal032 20 →
  23/67** (report-only at the merge; the wiring is №723); the floor
  **6551 → 6758 bp** (98/145) in the SAME PR (the №757/№613 procedure),
  the fact twins in the same PR (precise 4505 → 4524 = 238/526;
  parameterized 7478 → 7520 = 91/121; gate_031 6758); the general typed
  share 357 → **360/526 = 68.44%**; the 30 fresh anchors + 14
  re-verified refusals (the movement's honesty map);
  `tests/naryad_717_type_closures.rs` — the registry meta facts + the
  live both-backend probes (the №428 consent grant arms the listen
  flow; the vision default-deny loud refusal). Zero semantic edits;
  the list NOT edited (the id closes once).
- **№718** (gh#1330, squash `99dfded`): the ADR-0193 verdict package —
  the fact map, the FOUR parameters (the figure ≥20/67; the deadline;
  the ПДн-first record; the wiring form), the endgame rule — packaged
  to the owner's verbatim with ZERO self-capture (the report-only
  posture held; the gate stayed outside the workflows until the word).
- **№719** (gh#1331, closed by the completion audit — no PR): the
  release line of the two windows — the 0.33.0 verified-wait RESOLVED
  (the publication verified: the tag == `474ff7ae7`, 4/4 assets, the
  SHA256 cross OK, BUILD-INFO == the tag commit); the §2.5 ride
  resolved by the owner's choice **(б)** — the full train ride; the
  notes-regen rule executed in №721. The draft v0.4.0 (id 333311937)
  flagged — the owner's act by one word.
- **№721** (gh#1339, **gh#1340** squash `268cdde`): the **0.33.1
  PUBLICATION** — the full train ride (№707 + №716 + the gh#1336
  nested-bodies fix) by the owner's live directive («б»); the
  date-stamp 2026-10-26 → 2026-10-19; the notes regen by №497 (the
  №713 draft superseded — the regenerate-at-the-window rule held);
  the tag `v0.33.1` on 268cdde; **PUBLISHED 2026-10-19T08:33:38Z —
  Latest**; 4/4 assets verified TWICE (draft + post-publish runs; the
  SHA256 cross OK; BUILD-INFO == 268cdde; rustc 1.99.1 58cf8f9c3). The
  intermediate "checksum mismatch" — the verification artifact of the
  office (the dispatch-epoch SUMS vs the final files), not a defect.
- **№722** (gh#1341, **gh#1342** squash `160a6ee`): the soak-parity
  ENOSPC guard — the 19.10 nightly failure (run 39215565228)
  root-caused to the runner disk (`os error 28` at the doc-test build
  phase AFTER three green test phases; the В48 class, NOT a product
  regression); the "Free runner disk" step lands before the build
  phases (dotnet/android/ghc/ghcup/CodeQL + docker prune; df-h
  before/after); verified by the dispatch run **39231764238 SUCCESS**
  (both jobs; df 86G → 110G, +24G).
- **№723** (gh#1343, **gh#1345** squash `657a0db`): **ADR-0193
  RATIFIED** — the owner's live verbatim «вербатим принят»
  (2026-10-19); Status → Accepted; §7 the ratification record; §6.7
  SUPERSEDED (the gate is wired); **the goal032 gate WIRED BLOCKING**
  in ci.yml — the gate_031 §5 one-PR form: the fact key
  (`gate_032_untyped_sources.txt`, FROZEN) WITH its checker; the
  operational floor = the live counter at the wiring **23/67,
  only-up**; the lift rides the closure PR (the №757/№613 form). The
  live verification on main: `--gate032 … 23` → OK (the owner-fixed
  figure), exit 0. The MARKED tail: the `--goal032` report strings
  still cited the Proposed phase — fixed in №720 (this record).
- **№724** (gh#1344, **gh#1346** squash `24d2823`): the rcdom req line
  hygiene — `markup5ever_rcdom = "0.39.0+unofficial"` → `"0.39.0"`
  (the №707 tail; the only build warning gone); the resolver
  compatibility unchanged (the semver metadata is ignored by cargo);
  Cargo.lock untouched (the package version stays the real published
  tag).
- **#956** (the good-first pool, **gh#1347** squash `d990531`):
  the risk-review perimeter follows the files — the checklist's
  `src/io.rs` entry was a DEAD trigger (the file never existed in this
  history); the perimeter now names `src/builtins/io.rs` +
  `src/fs_gate.rs` (the №567 form); the pool refill rides the policy
  mechanics (the office note to the owner).
- **№720** (this record; gh#1332): the post-wave sync — REALITY §6.35
  + the PLAN-SUMMARY Wave 50 row + the REFERENCE check (fresh, 526/526
  = 100.0%, no diff) + the **MARKED deviation**: the `--goal032`
  report strings cite the ADR phase — Proposed → Accepted (the
  report-text-only fix, zero semantics; announced in the №723
  completion audit; the CHANGELOG untouched — №687).

The counters at the record date (the generators, NOT hands):
**ignore debt 8** (todo 0, dead_code 15), **sensitive 98/145 = 6758
bp** (the floor 6758 only-up; the uncovered by reason: struct-no-meta
2, typed-coarse 1, untyped 44), **goal032 closed 23/67** (the gate
BLOCKING — ADR-0193 Accepted; the floor 23 only-up), **typed
signatures 360/526 = 68.44% (precise 238/526 = 45.24%)**, the
**parameterized share 91/121 = 75.20%** (twins 7520 bp), the
**field-label share 58/60 = 96.66%**, **mirrors 6/6 (the threshold 6 —
no excess), dups 0**, **REFERENCE 526/526 = 100.0%, Missing 0**,
**[Unreleased] on main carries the №723 Changed row + the №724 Fixed
row** (the merged naryads only — the №687 rule).

The owner gates remaining (accumulated): the gh#1317 resolution
verdict + the office #536 fix decision; the **ADR-0194 verbatim** (the
packages A/B — the query error paths + the result contract); the
good-first pool refill mechanics (the office note after the #956
execution); the zip-ignore review (2026-11-17 or the first candle
zip-^9 release); the coverage threshold (№569); the second maintainer;
NLnet/Restack M1 (03.11.2026); №648 (01.12.2026); the PAT rotation
(the exposed-token discipline); the draft v0.4.0 cleanup (the owner's
act). The machine tail: **№725** (gh#1348) — the typed-floor raise to
6844 bp (the №467 procedure; the unfreeze-gate advisory convergence),
P3, the next нарядный PR.

### 6.36. Wave 51 post-wave record (naryad №732): the В50 tail executed + the residue-wall movement №6 + the release-line prep with ADR-0195 + the ADR-0194 ratification landing — query() is the ONE contract on both lanes (main @ the naryad's PR, 2026-10-20)

The В50 tail rode outside the В51 composition (the dispatch gh#1362
pre-registered the ride) and was executed first; the wave composition
№730 ∥ №731 → №732 (strictly last) + the tail №734:

- **The В50 tail** (2026-10-19): №727 (gh#1354, squash `bd56dc1`) — the
  carrier B note into CHANGELOG [0.33.1] (the gh#1350 Ask 1); №728
  (gh#1355, squash `b2aa774`) — the sink-clearance refusal branches by
  the failed gate (the Ask 3 verbatim (a2): the ACTION six keeps the
  №391 bridge text byte-for-byte, the others cite the generic rule +
  the №316 classification); №729 (gh#1356, squash `2610039`) — the
  sanctioned read-back pair recognition (the Ask 2 verbatim (a):
  `semantic::READBACK_PAIRS`, the key-position-only exemption,
  fail-closed by construction; `mtree_store` refuted by the source —
  the true twin of `cron_remove` is `mtree_forget`); gh#1350 closed
  executed; main CI GREEN on 2610039.
- **№730** (gh#1359, squash `a3e95eb`): the movement №6 — the FIRST
  package under the BLOCKING gate — returned the HONEST ZERO-closure
  result: the ПДн residue (13 rows) closes nothing; all 44 open
  fixed-list rows carry standing refusal postures (the handler code
  has not moved since the №717 walk — the diff over the handler
  surface EMPTY, verified 2026-10-20). Delivered as the refusal
  package: 10 drifted anchors refreshed in registry.rs (comment-only),
  the refusal shapes probed live on both backends
  (`tests/naryad_730_type_closures.rs`: recipe_list `List/[]`, the
  session unions BOTH arms live, memory_read public arm String, stdin
  the loud true-stub refusal), the 44-row Unknown ground re-pinned
  (42 default + 2 feature-gated — the №717 form). **goal032 stays
  23/67** (the gate GREEN, exit 0); the growth paths run through
  owner/semantics lanes (the Secret spelling — the dictionary lane;
  the №543 union forms; the stub handlers; query* — post-№734); the
  residue wall is the designed endgame frontier (ADR-0193 endgame
  (A)).
- **№731** (gh#1360, squash `126c2d2`): the release line — the 26.10
  prep contour VERIFIED GREEN (unfreeze --gate-target 0.33.1
  --office-tests pass --strict EXIT 0; release_gap EXIT 0; the notes
  verbatim both ways EXIT 0; release.yml as-is);
  `docs/release-notes-next-draft.md` — the prep-day snapshot generated
  from [Unreleased] byte-for-byte (the figure UNDECIDED — the owner's
  call to the window; the regen-at-the-window rule, №713);
  **ADR-0195** (Proposed, zero wiring) — the 0.34 release-gate
  criteria: (a) the ratchet floors only / (b) the ADR-0193 endgame (A)
  figure as the release RECORD (the honest N today = 23 — the wall) /
  (c) the precise floor + №569 (parked until the owner lanes open);
  the §5 recommendation without the decision; the §6 verdict package.
  The §2.5 map + go/no-go filed to gh#1360: **GO 26.10 verified-wait**
  (0.33.2 recommended — the tail-ride PATCH); the ADR index row 0194
  registered + 0195; the README ADR count 186 → 187 (the
  readme_consistency machine caught the drift — it works as designed).
- **№734** (gh#1364, squash `8b76ab2`): **ADR-0194 RATIFIED — Accepted
  2026-10-20** (the owner's verbatim with the package-choice
  delegation, the В49 lekalo; packages **A(c)** + **B(i)** — the §8
  record) and LANDED: the VM mutation detect mirrors the TW
  byte-for-byte (trim().to_uppercase(); NOT SELECT/PRAGMA) —
  **the observable change: the VM-lane mutations through `query()`
  return the affected count as a String ("n") instead of the empty
  List** (the №474 ONE contract on BOTH lanes; the examples/office
  mutations ride `db_execute` — zero affected consumers, the §3 ADR
  inventory); the TW SELECT loop mirrors the VM lane (the loud-fail
  CONTRACT: the cell error names the column — the :769 mirror; the row
  error propagates; the dead `Unit` lie and the silent skip are gone —
  №604/№523/№531; practically unreachable on the reachable inputs —
  the ADR §2 probe record remains the carrier);
  `tests/n194_query_parity.rs` (t1 the parity pin — the mutation →
  String "3" on TW AND VM; t2 the SELECT control; t3 the №381
  regression; t4 the parameterized mutation; t5 the №474 cross-check);
  **the mutation teeth reproduced in CI** (the A.3 revert turned
  t1/t4 RED on the VM lane — `[TYPE_MISMATCH] String + List` — then
  restored byte-identical); the №702 divergence pin rides the
  sanctioned parity (renamed `n702_query_mutation_parity_after_n734`,
  assert_both "1/1" — the №728 lesson); the limitations №711 row →
  the resolved form (split by root); the [Unreleased] Changed row №734
  with the AFFECTS estimate; the registry query*/query_row/
  query_scalar anchor cites synced (the same-PR rule); the pin tail
  db_ops.rs:708-716 (9 lines preserved — the №711 lekalo). The CI
  rounds were the honest walk: 1 (the E0599 fix — the TW SELECT loop
  mirrors the VM; no UserFunctionError in this rusqlite), 2 (the
  mutation teeth), 3 (the №702 pin + fmt), 4 (the anchors + the index
  Accepted), 5 (the fmt t5 short-last-arg).

The machine numbers at the sync (2026-10-20, main @ 8b76ab2): the
sensitive surface **98/145 = 6758 bp** (the enforced floor **6844 bp**,
only-up — №725's raise stands over the current share), **goal032
closed 23/67** (the gate BLOCKING — ADR-0193 Accepted; the floor 23
only-up; the movement №6 honestly closed zero), **typed 360/526 =
68.44% (precise 238/526 = 45.24%)**, the **parameterized share
91/121 = 75.20%** (twins 7520 bp), the **field-label share 58/60 =
96.66%**, **mirrors 6/6, dups 0**, **debt 8/0/15**,
**REFERENCE 526/526 = 100.0%, Missing 0**, **[Unreleased] carries the
№729 Added + №723/№734 Changed + №724 Fixed rows** (the merged naryads
only — the №687 rule).

The owner gates remaining (accumulated): the **26.10 window — the
figure (0.33.2 recommended / 0.34.0) + the tag → publication** (the
§2.5 map in gh#1360); the **ADR-0195 verdict** (the 0.34-gate
criteria — the §6 package: the form / the endgame N / the precise
floor + №569); the **goal032 next-movement shape** (the residue-wall
report of №730 — the owner lanes: the Secret spelling, the №543 union
forms, the stub naryads); the office #536 fix decision; the good-first
pool refill mechanics; the zip-ignore review (2026-11-17); the
coverage threshold (№569); the second maintainer; NLnet/Restack M1
(03.11.2026); №648 (01.12.2026); the PAT rotation. RESOLVED this
cycle: the ADR-0194 verbatim (the packages A(c)+B(i) — №734); the
draft v0.4.0 cleanup (deleted, verified 0 drafts).

### 6.37. Wave 52 post-wave record (naryad №738): the ADR-0195 ratification tail + the query*-wall movement №7 + the 26.10 window refresh — the vocabulary lane is the named growth path (main @ the naryad's PR, 2026-10-20)

The №735 tail rode outside the В52 composition (the dispatch gh#1374
pre-registered the ride) and merged first; the wave composition
№736 ∥ №737 → №738 (strictly last):

- **№735** (gh#1369, squash `a21674b`): **ADR-0195 RATIFIED — Accepted
  2026-10-20** (the owner's verbatim «вербатим ADR-0195 (0.34-гейт»;
  the §5 package as submitted — the ADR-0193 §7 form, the delegation
  lekalo §8 ADR-0194): **P1 = (b)** the accumulated ratchet floors as
  the base + the endgame (A) figure as the 0.34 line's release RECORD
  (the notes-record, NOT a blocking wire — the cadence stays decoupled
  from the types program); **P2 = N=23** (the honest value — the
  residue wall; the ADR-0193 §7.5 floor already enforces 23 only-up);
  **P3 = (c) PARKED** (the precise floor + №569 — until the owner
  lanes open). Zero CI wiring (the RECORD form — §6.4); the landing:
  the ADR §7 + the index rows (0195 → Accepted; the 0193 row synced —
  stale since №723) + the CHANGELOG Changed row (the №723 form); the
  README metrics regen 77 → 78 KB. CI 49/49 (gh#1370).
- **№736** (gh#1371, squash `3681662`): the movement №7 — the query*
  package after the ADR-0194 ratification — returned the SECOND HONEST
  ZERO-closure result, with the refusal table CORRECTED by the machine
  facts: the №712/№730 DIVERGENCE cause is DEAD (the №734 parity — the
  mutation String on both lanes, the SELECT List on both); the
  query_row/query_scalar "planned wrapper, no handler" anchors were
  factually STALE (the №26 P1-7 intercepts ARE live — TW
  execution.rs:2474-2490, VM vm.rs:2439-2442/2706-2712); what SURVIVES
  is the UNION ground the stage-0 vocabulary cannot spell (query
  rows|count — the context-less call refuses loudly: the №7 intercepts
  precede the registry dispatch on BOTH lanes, execution.rs:2440-2442 /
  vm.rs:2447; query_scalar String|Float|Unit; query_row the
  heterogeneous List). Delivered: the three registry anchors refreshed
  (comment-only), the live-handler battery
  `tests/naryad_736_query_refusals.rs` (8 tests: the wired handlers on
  both lanes, the union arms String/Float/Unit, the loud context-less
  refusals — the t6c machine fact caught by the CI round 1 and
  re-pinned). **goal032 stays 23/67** (BLOCKING GREEN); the sensitive
  floor unraised (the №757/№613 form — nothing honest to raise). The
  named growth path (the owner lane, №644 boundary): (α) the union
  spelling in the №627 grammar / (β) the fixed-result №543 form /
  (γ) the Query/Row opaque kinds.
- **№737** (gh#1372, squash `d650bfc`): the 26.10 window refresh — the
  notes draft regenerated from the current [Unreleased] (the
  №734/№735 rows in; verbatim-verified True — the №497/№713 rules);
  the release gates GREEN (unfreeze --strict EXIT 0, release_gap
  EXIT 0); the §2.5 figure map filed to gh#1372: 0.33.2 tail-ride
  carries all five rows (the №729 compiler unlock + the №734 VM
  observable + the CI/process rows), 0.34.0 has NO feature roster (the
  version-semantics lie); **the executor's position: 0.33.2
  tail-ride; the decision — the owner's (gate 1)**; **GO 26.10
  verified-wait**.

The machine numbers at the sync (2026-10-20, main @ d650bfc): the
sensitive surface **98/145 = 6758 bp** (the enforced floor **6844 bp**,
only-up), **goal032 closed 23/67** (the gate BLOCKING — ADR-0193
Accepted; the movement №7 honestly closed zero), **typed 360/526 =
68.44% (precise 238/526 = 45.24%)**, the **parameterized share
91/121 = 75.20%**, the **field-label share 58/60 = 96.66%**,
**mirrors 6/6, dups 0**, **debt 8/0/15**, **REFERENCE 526/526 =
100.0%, Missing 0**, **[Unreleased] carries the №729 Added +
№723/№734/№735 Changed + №724 Fixed rows** (the merged naryads only —
the №687 rule).

The owner gates remaining (accumulated): the **26.10 window — the
figure (0.33.2 recommended / 0.34.0) + the tag → publication** (the
§2.5 map in gh#1372); the **query* vocabulary lane** (the №736
decision point: (α) the union spelling / (β) the fixed-result №543
form / (γ) the Query/Row opaque kinds); the goal032 lanes (the Secret
spelling, the №543 union forms, the stub naryads); the office #536 fix
decision; the good-first pool refill mechanics; the zip-ignore review
(2026-11-17); the coverage threshold (№569 — PARKED by ADR-0195 (c));
the second maintainer; NLnet/Restack M1 (03.11.2026); №648
(01.12.2026); the PAT rotation. RESOLVED this cycle: the ADR-0195
verdict (the form (b) + N=23, (c) PARKED — №735); the 26.10 prep
(the verified-wait GO — №737).

### 6.38. Wave 53 post-wave record (naryad №742): the query* movement №8 — the №740 UNION vocabulary (lane α) LANDED + the 26.10 window prep with the post-merge refresh — the first ПДн-class group fully closed (main @ the naryad's PR, 2026-10-20)

The wave composition №740 ∥ №741 → №742 (strictly last; the dispatch
gh#1381):

- **№740** (gh#1378, squash `88ea0d9`): the 0.32 fixed-list movement
  №8 — the query* package through the **№740 UNION vocabulary**
  (lane α — the growth path №736/§6.37 named): the stage-0 dictionary
  gains the union spelling `arm|arm|...` (the split at the TOP-LEVEL
  pipes — depth 0, outside any `<>`; a pipe inside `<>` is an inner
  union, the erasure's parameter business; ≥2 stage-0-clean arms —
  `arm_is_clean`, the drop-free const mirror of from_path's accept
  set; STRICTLY ASCENDING byte order — the canonical form; a
  duplicate, an unsorted or a malformed arm fails closed to Unknown).
  THREE registry rows closed by the №644 criterion from the LIVE
  handler facts (the №539 false-warn guard satisfied by construction
  — every reachable arm named): `query` →
  `List<Struct<Row>>|String` (the SQL-class union rows|count — the
  №474 ONE contract; the Row fields dynamic by construction, no
  field-meta claimed, the rows-path NOT extended — the ADR-0194
  Запреты hold), `query_scalar` → `Float|String|Unit` (the cell
  union, db_ops.rs:478-489), `query_row` → `List<Float|String|Unit>`
  (the FIRST row's cells, db_ops.rs:554-576 — the №669
  "a List<X> spelling would be fictitious" concern dissolves with the
  union inner). The new `intercept` spec! variant (the
  typed-handlerless form — the live handlers are the №26 P1-7 lane
  intercepts, the registry rows stay handler: None, the context-less
  dispatch the loud stub; the t6 pin loud on both lanes). The mirrors
  in lockstep (the two-locks discipline): type_signature_share.py +
  sensitive_surface_share.py (the №740-union branch of the №644
  covered rule — well-formed ∧ every arm covered-grade; a malformed
  union is typed-coarse, fail-closed) + the five regex copies
  (`|` in the char class — type_signature_share, gen_metrics ×2,
  gen_reference, readme_consistency, n566). The floors (the №757/№613
  same-PR form, the machine's own notes): the sensitive 6758 →
  **6965 bp** (101/145), the general typed 6844 → **6901 bp**
  (363/526), the parameterized 6893 → **7540 bp** (92/122 — the
  pending №668-era note to 7520 consumed), the goal032 BLOCKING
  figure **23 → 26** in ci.yml (**26/67** — the db (query*) group
  3/3, the FIRST ПДн-class group fully closed; the raise rides the
  owner's В53 dispatch — the ADR-0195 §7 note); the fact twins synced
  same-PR (gate_030 fact_parameterized_share_bp 7540, gate_031
  fact_sensitive_surface_bp 6965 — the №525 form). The battery
  `tests/naryad_740_union_spellings.rs` (6 tests: the registry-side
  pins, the grammar armor, the false-warn guard against the LIVE
  both-backend contract, the t6 pin). The interlocks №706/№712/№717/
  №730 re-pinned (44 → 41 open ground). **The honest round-fixation:**
  CI round 1 (run 39418087071) caught the two locks the movement
  missed — the interlock pins (the query* rows were pinned
  Unknown-honest by three earlier batteries) and the gate-facts fact
  twins; both re-pinned/synced in the SAME PR (c5bfcc5) — the form
  worked as designed. CI 49/49 (round 2 @ c5bfcc5). (β) and (γ) NOT
  taken (the dispatch gate 7 — separate owner verdicts).
- **№741** (gh#1379, squash `f4741c5`): the 26.10 window prep —
  the notes draft re-cut (the wave-53 refresh of the №737 snapshot)
  and REFRESHED over the №740 merge per the naryad's own form (the
  [Unreleased] carries the movement №8 row; verbatim-verified True —
  the №497/№713 rules); the release gates GREEN twice (round 1 on the
  prep snapshot @ 676ad4e, round 2 on the post-merge refresh
  @ 344cad3: unfreeze --strict EXIT 0, release_gap EXIT 0, goal032
  26/67 EXIT 0, gate-facts-sync EXIT 0); the §2.5 figure map filed to
  gh#1379: 0.33.2 tail-ride carries all six rows (the №740 row is
  metrics/registry-comments/gates — zero product semantics),
  0.34.0 has NO feature roster (the RECORD «closed 26/67» reads
  naturally in the 0.34 notes when the owner opens the figure);
  **the executor's position: 0.33.2 tail-ride; the decision — the
  owner's (gate 1)**; **GO 26.10 verified-wait**.

The machine numbers at the sync (2026-10-20, main @ f4741c5): the
sensitive surface **101/145 = 6965 bp** (the enforced floor **6965
bp**, only-up — the equality the №740 raise recorded), **goal032
closed 26/67** (the gate BLOCKING — ADR-0193 Accepted; the db group
3/3), **typed 363/526 = 69.01% (precise 238/526 = 45.24%)**, the
**parameterized share 92/122 = 75.40%**, the **field-label share
58/60 = 96.66%**, **mirrors 6/6, dups 0, quorum 0/8**, **debt
8/0/15**, **REFERENCE 526/526 = 100.0%, TODO(doc) 0**, **[Unreleased]
carries the №729 Added + №723/№734/№735/№740 Changed + №724 Fixed
rows** (the merged naryads only — the №687 rule). gen_metrics --check
OK (the hand-written numbers match the generators — the №566
consistency holds).

The owner gates remaining (accumulated): the **26.10 window — the
figure (0.33.2 recommended / 0.34.0) + the tag → publication** (the
§2.5 map in gh#1379); the **query* lanes (β)/(γ)** (the fixed-result
№543 form / the Query/Row opaque kinds — the owner verdicts, the
dispatch gh#1381 gate 7); the goal032 lanes (the Secret spelling, the
№543 union forms, the stub naryads); the office #536 fix decision; the
good-first pool refill mechanics; the zip-ignore review (2026-11-17);
the coverage threshold (№569 — PARKED by ADR-0195 (c)); the second
maintainer; NLnet/Restack M1 (03.11.2026); №648 (01.12.2026); the PAT
rotation. RESOLVED this cycle: the **query* vocabulary lane (α)** —
the union spelling LANDED (№740, the goal032 23 → 26/67); the 26.10
prep (the verified-wait GO, the post-merge refresh — №741).

### 6.39. Wave 54 post-wave record (naryad №746): the movement №9 — the lane α continuation (the ПДн wall + the machine-chosen package) + the 26.10 prep refresh — the graph/vision package closed (main @ the naryad's PR, 2026-10-20)

The wave composition №744 ∥ №745 → №746 (strictly last; the dispatch
gh#1388):

- **№744** (gh#1385, squash `5bdbbeb`): the 0.32 fixed-list movement
  №9 — the lane α continuation through the **№740 UNION vocabulary**.
  The ПДн prima-facie group (the ADR-0193 priority — eight rows:
  session_poll_wake, session_take_interrupt, memory_open, memory_read,
  secret, stdin, request_body, device_open) read against the live
  sources on e1c66b0 — **a refusal wall, every row with a named
  class**: session_* — the №543-form state-dependent unions (the (β)
  lane, the owner gate NOT taken; ADR-0195 §7 (c) «№543 union-формы»
  parked); memory_read — the Secret arm (ADR-0195 §7 (c)
  «Secret-спеллинги» parked; the Opaque arm not covered-grade by the
  №740 union rule); memory_open/secret — the opaque handles with no
  kind in the dictionary; stdin — the true stub; request_body — the
  dynamic Struct fields (the №661 armor); device_open — the opaque
  Device handle (Embodied = typed-coarse, never covered). The
  honest-zero branch fired for the ПДн group; the NEXT group chosen by
  the machine (`--goal032 --list` + the covered-grade rule). THREE
  rows closed by the №644 criterion from the LIVE both-lane handler
  facts: `graph_path` → `List<Struct<PathError>|Struct<PathNode>>`
  (the union INNER — the №740 query_row form; the result is ALWAYS a
  List — the №170 SSOT handler memory.rs:789 serves both lanes; the
  found arm the PathNode{id,text,level} elements memory.rs:800, the
  no-path arm the SINGLE PathError{status,from,to} memory.rs:812; the
  №669 "a fictitious List<PathNode>" concern dissolves with BOTH
  element shapes named; field-labels stage 2 — the sibling
  graph_query precedent), `vision_list` → `List<String>` (the №240
  intercept is the LIVE path on both lanes — execution.rs:599/2061,
  vm.rs:2955 — the `[Vision#N]` String handles vision.rs:362; the row
  keeps its loud-Err fallback byte-for-byte — the query №740 posture:
  the type is the INTERCEPT's shape, not the fallback's),
  `vision_lora_load` → `String` (the №244 intercept —
  execution.rs:813/2275, vm.rs:3008; the SINGLE Ok arm vision.rs:1097
  — the №757 decrypt form). The floors (the №757/№613 same-PR form):
  the sensitive 6965 → **7172 bp** (104/145), the general typed
  6901 → **6958 bp** (366/526), the precise 4302 → **4543 bp**
  (239/526 — the enforced FIRST-match triple restored to the live
  value; the three stale history triples reworded to the was-form —
  the №536 procedure; the pending machine note consumed), the
  parameterized 7540 → **7580 bp** (94/124 — the denominator grows
  WITH the two new List rows), the goal032 BLOCKING figure **26 → 29**
  in ci.yml (**29/67**); the fact twins synced same-PR (gate_031
  fact_sensitive_surface_bp 7172; gate_029 + gate_030
  fact_precise_share_bp 4543 — BOTH records, caught by the local
  sync-checker run before the push; gate_030
  fact_parameterized_share_bp 7580 — the №525 form). The compiled
  floors: TYPED_FLOOR 339, PARAM_FLOOR 77, PARAM_LS_DENOM 102. The
  battery `tests/naryad_744_lane_alpha_spellings.rs` (5 tests: the
  registry-side pins, the inner-union canonical grammar — the
  TOP-LEVEL twin proves the ascending order, the live both-backend
  probes: the graph_path found/no-path arms through mtree_store + the
  dijkstra self-path, the vision_list empty-registry List, the
  vision_lora_load loud refusals naming the builtin). The interlocks
  №717/№730 re-pinned (41 → 38 open ground) + **the honest
  round-fixation:** CI round 1 (run 39462069570) caught the n669
  graph_path pin (the honest-refusal pin of the wave-40 battery — the
  row left the ground with the live fact; re-pinned to the closure
  form) and one fmt fold — both fixed same-PR (4e6b826); CI 49/49
  (round 2). (β) and (γ) NOT taken (the dispatch gate 7).
- **№745** (gh#1386, squash `e1bbd98`): the 26.10 window prep — the
  notes draft re-cut (the wave-54 refresh of the №741 snapshot),
  REFRESHED over the №744 merge per the naryad's own form (the
  [Unreleased] carries the movement №9 row; verbatim-verified True —
  the №497/№713 rules); the release gates GREEN twice (the refreshed
  base 5bdbbeb AND the main at the audit: unfreeze --strict EXIT 0 —
  Overall GREEN, 4.1 enforced 6958 = recorded 6958; release_gap EXIT
  0; goal032 29/67 EXIT 0; gate-facts-sync EXIT 0); the §2.5 figure
  map filed to gh#1386: 0.33.2 tail-ride carries all SEVEN rows
  ([Unreleased]: №729 Added + №723/№734/№735/№740/№744 Changed +
  №724 Fixed), 0.34.0 has NO feature roster (the RECORD «closed
  29/67» reads naturally in the notes when the owner opens the
  figure); **the executor's position: 0.33.2 tail-ride; the decision
  — the owner's (gate 1)**; **GO 26.10 verified-wait**.

The machine numbers at the sync (2026-10-20, main @ e1bbd98): the
sensitive surface **104/145 = 7172 bp** (the enforced floor **7172
bp**, only-up — the equality the №744 raise recorded), **goal032
closed 29/67** (the gate BLOCKING — ADR-0193 Accepted; the db group
3/3, the request_body group 0/1), **typed 366/526 = 69.58% (precise
239/526 = 45.43%)**, the **parameterized share 94/124 = 75.80%**, the
**field-label share 58/60 = 96.66%**, **mirrors 6/6, dups 0, quorum
0/8**, **debt 8/0/15, example_uncovered 0**, **REFERENCE 526/526 =
100.0%**, **[Unreleased] carries the №729 Added + №723/№734/№735/
№740/№744 Changed + №724 Fixed rows** (the merged naryads only — the
№687 rule). gen_metrics --check OK (the hand-written numbers match
the generators — the №566 consistency holds).

The owner gates remaining (accumulated): the **26.10 window — the
figure (0.33.2 recommended / 0.34.0) + the tag → publication** (the
§2.5 map in gh#1386); the **goal032 lanes (β)/(γ)** (the fixed-result
№543 form / the Query/Row opaque kinds — the owner verdicts, the
dispatch gh#1388 gate 7); the Secret spellings + the №543 union forms
(ADR-0195 §7 (c) — parked until the lanes open); the office #536 fix
decision; the good-first pool refill mechanics; the zip-ignore review
(2026-11-17); the coverage threshold (№569 — PARKED by ADR-0195 (c));
the second maintainer; NLnet/Restack M1 (03.11.2026); №648
(01.12.2026); the PAT rotation. RESOLVED this cycle: the **movement
№9** — the lane α continuation landed (№744, the goal032 26 → 29/67;
the graph/vision package closed); the 26.10 prep (the verified-wait
GO, the post-merge refresh — №745).

### 6.40. Wave 55 post-wave record (naryad №749): the movement №10 — the intercept walk of the WHOLE wall (the №736 lesson applied systematically) — the six-row closure package (main @ the naryad's PR, 2026-10-21)

The wave composition №748 → №749 (strictly last; the dispatch gh#1394;
the release naryad NOT assigned — the 26.10 window occupied by the GO
verified-wait №745, the dedup):

- **№748** (gh#1392, squash `ef14797`): the 0.32 fixed-list movement
  №10 — the lane α continuation (the В55 dispatch gh#1394). The
  machine walk of ALL 38 open rows on live main 4b13319 with the
  **№736 intercept-walk lesson applied to the WHOLE wall** (the
  registry comment is the REGISTRY arm, not the whole ground — every
  row's interpreter/VM dispatch chain checked, not just the comment).
  SIX rows closed by the №644 criterion from the LIVE both-lane
  handler facts: `reflex_list` → `List<String>` (the shared
  reflex_list_dispatch precedes the registry stub on BOTH lanes — TW
  execution.rs:578/2038, VM vm.rs:2918-2923; the names sorted by
  ReflexId.0 — deterministic, reflex.rs:1220-1241; the empty registry
  answers the honest empty List — the vision_list №744 form; the row
  keeps its loud-Err stub byte-for-byte — the query №740 posture),
  `event_count` → `Float` + `event_sum` → `Float` (the ADR-0052
  intercepts ARE the live handlers on both lanes — execution.rs:2337/
  2351, vm.rs:2824/2829; audit_ops.rs:86-91/112-124 — the single-shape
  №757 decrypt form; the №717 «planned wrapper, no handler» anchors
  were factually STALE), `events_since` → `List<Struct<Event>>` (the
  same ground — execution.rs:2342, vm.rs:2834; audit_ops.rs:142-179 —
  every element Struct{type_name: "Event"}; the fields dynamic by
  construction — duration_ms optional — NO field-meta claimed, the
  query rows №740 form), `deny_event` →
  `Struct<DenyEvent>{reason:internal,sink:internal,class:internal,
  argument:untrusted,label:internal,line:internal,human:untrusted}`
  (the №392 on_deny intercepts ARE the live handlers on both lanes —
  execution.rs:2318, vm.rs:2395; deny.rs:122-143 make_event — the
  FIXED 7-field projection, the listen_start №717 form; argument/human
  echo the refused call → untrusted — the №706 device_state echo form;
  OUTSIDE an on_deny body the usage is a COMPILE error
  [DENY_HANDLER_SCOPE] — the error channel, not a shape),
  `deny_reason` → `String` (the reason field make_event ALWAYS inserts
  — deny.rs:132 — the defensive unwrap_or(Unit) unreachable by
  construction; the single reachable arm). The OTHER 32 rows refused —
  the wall classes re-verified with fresh anchors: **the (β) lane, the
  owner gate 7 (4)** — session_poll_wake/session_take_interrupt (the
  №543-form state unions; the №730 anchor ranges 276-291/326-345 were
  STALE — the live arms session.rs:148-160/198-207; refreshed
  comment-only), backend_select (the №543 form, backends.rs:225/262),
  **inspect** (the Struct "PatternStats" | Unit state union — the
  pattern-existence state; the live intercepts execution.rs:2326/
  vm.rs:2716; the SAME (β) class as session_*); **data-dependent (3)**
  — call_llm_schema (llm_schema.rs:162/287-303), config_load
  (office/config.rs:130 → json.rs:321-341), **find** (the ADR-0051
  intercepts LIVE on both lanes — execution.rs:2313, vm.rs:2415; the
  first matched Value of an ARBITRARY type | the soft-failure Unit —
  the №717 anchor STALE, refreshed); **the feature duality (1)** —
  reflex_generate (the Ok arm List<Float> under candle reflex.rs:1353;
  the default build refuses loudly reflex.rs:1359 — no house precedent
  for feature-conditional typing, the vision №744 intercepts were
  UNCONDITIONAL; the owner lane); **coarse-opaques (2)** — reflex_load
  (the live dispatch → Value::Reflex reflex.rs:1113 — the kind EXISTS
  in the vocabulary, the bare opaque is not covered-grade — the №744
  device_open form), reflex_bpe_load (never Ok, no dispatches);
  **no-vocabulary-name opaques (9)** — memory_open (Value::Memory),
  duplex_open (Value::Duplex; the anchor 213 → 218 refreshed),
  device_open, pose_make, trajectory_make, goal_make, subgraph_extract,
  forecast_next, vision_load (the live dispatch → Value::Vision);
  **named-but-coarse opaques (4)** — series_make (SeriesHandle),
  media_source_capture (the live dispatch → Value::Media media.rs:459),
  secret, generate_key (Secret — parked, ADR-0195 §7 (c));
  **parked/dynamic (3)** — memory_read (String|Secret — parked),
  request_body (the №661 dynamic fields), stdin (the true stub —
  verified UNDEFINED_FUNCTION on both lanes); **always-empty Lists
  (2)** — recipe_list/recipe_search (the №669 form); **never-Ok/loud
  (3)** — series_pull (src/forecast.rs:451-457), voice_load,
  video_fetch_weights. The floors (the №757/№613 same-PR form): the
  sensitive 7172 → **7586 bp** (110/145), the general typed
  6958 → **7072 bp** (372/526), the precise 4543 → **4600 bp**
  (242/526), the parameterized 7580 → **7637 bp** (97/127 — the
  denominator grows WITH the three new List/Struct rows), the
  field-label share 58/60 → **59/61**, the goal032 BLOCKING figure
  **29 → 35** in ci.yml (**35/67**); the fact twins synced same-PR
  (gate_031 fact_sensitive_surface_bp 7586; gate_029 + gate_030
  fact_precise_share_bp 4600 — BOTH records; gate_030
  fact_parameterized_share_bp 7637 — the №525 form). The compiled
  floors: TYPED_FLOOR 345, PARAM_FLOOR 80, PARAM_LS_DENOM 105. The
  battery `tests/naryad_748_wall_walk.rs` (8 tests: the six-spelling
  registry pins — the DenyEvent meta byte-exact, the 32-row wall
  Unknown pins (+2 feature-gated under cfg), the live both-backend
  probes: reflex_list List|0, the events trio Float|Float|List (the
  shapes only — the counts data-dependent by design), the deny pair
  INSIDE on_deny through the kv bridge
  (`Struct|IRREVERSIBLE_NO_GRANT|String` — both lanes), the
  [DENY_HANDLER_SCOPE] compile refusal outside the handler, the stdin
  loud stub, the Secret/empty-List grounds). The interlocks
  №706/№717/№730 re-pinned. **The honest round-fixation:** CI round 1
  caught `cargo-deny` — the ADVISORY-DB DRIFT (the emSign-intermediates
  advisories published between main's 19:05Z green and the 21:16Z run;
  the fix = the patch bumps rustls-webpki 0.103.15 → 0.103.16 +
  webpki-roots 1.0.9 → 1.0.10, Cargo.lock only, zero API change —
  fed52f4) — and CI round 2 caught the **n706 interlock pin** (the
  №706 battery still pinned the deny pair and the events trio as
  Unknown — the local sweep had missed that battery; re-pinned, the
  full workspace sweep ran clean — 0d390ba); the coverage
  (non-blocking) flake of round 2 did NOT reproduce in round 3; CI
  **50/50** (round 3, run 39499832931 tail). (β) and (γ) NOT taken
  (the dispatch gate 7).
- **№749** (gh#1393, this PR): the post-wave docs sync — REALITY §6.40,
  the PLAN-SUMMARY Wave 55 row, the counters from the generators
  (gen_metrics --check OK — the README/PLAN-SUMMARY blocks were
  regenerated inside №748's PR per the fact-key-with-checker form;
  gen_reference --check OK — no REFERENCE drift: the six spellings
  ride the registry the REFERENCE renders from); docs/limitations.md —
  NO new honest-boundaries records (the six closures typed EXISTING
  surfaces; the §16.0-D stub discipline held — no new stub rows).

The machine numbers at the sync (2026-10-21, main @ ef14797): the
sensitive surface **110/145 = 7586 bp** (the enforced floor **7586
bp**, only-up — the equality the №748 raise recorded), **goal032
closed 35/67** (the gate BLOCKING — ADR-0193 Accepted; the db group
3/3, the request_body group 0/1), **typed 372/526 = 70.72% (precise
242/526 = 46.00%)**, the **parameterized share 97/127 = 76.37%**, the
**field-label share 59/61 = 96.72%**, **mirrors 6, dups 0, quorum
0/8**, **debt 8/0/15 (ignore 8, example_uncovered 0, dead_code 15)**,
**REFERENCE 526/526 = 100.0%** (gen_reference --check OK), **[Unreleased]
carries the №729 Added + №723/№734/№735/№740/№744/№748 Changed +
№724 Fixed rows** (the merged naryads only — the №687 rule).
gen_metrics --check OK (the hand-written numbers match the generators
— the №566 consistency holds).

The owner gates remaining (accumulated): the **26.10 window — the
figure (0.33.2 recommended / 0.34.0) + the tag → publication** (the
§2.5 map in gh#1386; the [Unreleased] now carries the movement №10 row
— the 0.33.2 tail-ride figure reads EIGHT rows); the **goal032 lanes
(β)/(γ)** (the fixed-result №543 form / the Query/Row opaque kinds —
the owner verdicts, the dispatch gh#1394 gate 7; the В55 walk added
the inspect classification to the (β) queue and the reflex_generate
feature-duality question to the owner lane); the Secret spellings +
the №543 union forms (ADR-0195 §7 (c) — parked until the lanes open);
the NLnet/Restack M1 (03.11.2026); №648 (01.12.2026); the zip-ignore
review (2026-11-17); the coverage threshold (№569 — PARKED by ADR-0195
(c)); the second maintainer; the PAT rotation. RESOLVED this cycle:
the **movement №10** — the intercept walk of the whole wall landed
(№748, the goal032 29 → 35/67; the six-row package closed; the deny
pair + the events trio + reflex_list live contracts pinned).

### 6.41. Wave 56 post-wave record (naryad №753): the movement №11 — the lane α ENDGAME: the honest zero + the (α) fixation — the two divergence discoveries; the release line — the draft refreshed to the eight-row truth, the 0.33.2 figure decided (main @ the naryad's PR, 2026-10-21)

The wave composition №751 → №752 → №753 (strictly last; the dispatch
gh#1400; the owner trigger 21.10 + the verbatim «Исполняй волну 56,
затем готовь релиз 0.33.2 к публикации» — the 0.33.2 figure FIXED):

- **№751** (gh#1397, squash `ccb4f9b`): the 0.32 fixed-list movement
  №11 — the lane α endgame. The machine walk of ALL 32 open rows on
  live main faa35c3 (the `--goal032 --list` list confirmed 1:1) — the
  **HONEST ZERO** (the №736 form) with the **(α) fixation: lane α is
  EXHAUSTED** — every remaining row grows only through the owner
  gates (the (β) state unions — the gate 7; the (γ) opaque kinds; the
  Secret spellings — ADR-0195 §7 (c)) or through the lane-parity
  fixes OUTSIDE the types movement (ADR-0194 Запреты). The TWO
  discoveries (both lane-parity facts, both outside the movement —
  the candidate [compiler] naryads): **(1) resolve_skill_index** —
  the №717 «true-stub posture» anchor factually STALE (the same error
  class as the №748 events/deny anchors): the handlers ARE LIVE on
  both lanes (TW execution.rs:2537-2603, VM vm.rs:2466-2522, the
  shared №466 group 7 validation runtime_ops.rs:261-290) — but the
  lanes DIVERGE: the TW type_name is `format!("SkillIndex_{dept}")`
  (execution.rs:2600) while the VM's is the fixed «SkillIndex»
  (vm.rs:2519), and the truncation field is TW-only (execution.rs:
  2588-2598; the VM arm ends at budget — vm.rs:2515-2521) — a single
  honest spelling is impossible without lying about a lane (the №669
  form; no dynamic-arm form in the union grammar) — the refusal
  class: the **LANE DIVERGENCE** (the ADR-0075 family), the row stays
  Unknown honest; **(2) the absent-field READ diverges across ALL
  structs** — the TW FieldAccess propagates the get_field error
  LOUDLY (execution.rs:1743; values.rs:386-394 — Result) while the
  VM GetField falls back to the silent Unit (vm.rs:1232 — the
  unwrap_or) — NOT in the ADR-0075 ledger; the loud-fail contract
  (ADR-0194) candidate. The registry comment refreshes (comment-only,
  the spec! args byte-for-byte): resolve_skill_index (true-stub → the
  divergence class) + fit_to_budget (the stale «no handler» anchor —
  live on both lanes: execution.rs:964/2605-2609, vm.rs:2839,
  runtime_ops.rs:78 — outside the №688 list, outside the movement).
  The interlock №730 noted (the Unknown pin holds). The battery
  `tests/naryad_751_endgame_walk.rs` (4 tests: the both-lane Struct
  liveness Struct|List|List|Struct — the TriggerRule type_name the
  SAME on both lanes; the loud no-declaration refusal both lanes; the
  behavioral divergence pin — the SAME program answers TW «String» /
  VM «Unit»; the registry Unknown pin). CI **49/49 GREEN round 1**;
  the floors NOT moved (the zero closures — only-up respected:
  goal032 35/67, sensitive 7586 bp, typed 7072, precise 4600,
  parameterized 7637 bp unchanged); gen_metrics/gen_reference --check
  OK; clippy -D warnings clean; locally 23/23 across the four
  batteries (n730 6, n748 8, n751 4, problem_a 5).
- **№752** (gh#1398, squash `f560c0b`): the release line of two
  windows (lekalo №719; the prep form №745). The machine draft check:
  the draft was the №745/В54 cut (7 rows) while [Unreleased] @
  ccb4f9b carries **EIGHT** (В55 assigned no release naryad — the
  dedup) → the composition changed → **the regen**: the body =
  `release_notes.py Unreleased` VERBATIM (the diff check
  BODY_VERBATIM_OK — №497: the notes and the changelog can never
  disagree by construction), the header re-dated (№752, 2026-10-21).
  The header records the DECIDED figure: **0.33.2 tail-ride — the
  owner's verbatim 21.10**; the cut [Unreleased] → [0.33.2] (the
  stamp 2026-10-26) + the lockstep 0.33.1 → 0.33.2 + the regen
  `release_notes.py 0.33.2` — the prep-to-publication tail (the
  В56-tail naryad); the publication — the OWNER's act (§6.5/ADR-0186
  §4.5). The gates GREEN on ccb4f9b: **unfreeze --strict Overall
  GREEN (§4 + v2 0.31), exit 0** (4.1 enforced 7072 bp = recorded;
  4.2 dups 0 / mirrors 6; 4.3 debt 8/8·0/0·15/15; 4.4 the twin
  naryad_429_memory_office_path GREEN 4/4 on this commit — the
  honest environmental note: the first local strict read fell RED on
  4.4 — the runner disk 100%, ENOSPC os error 28 at the twin build,
  the В48 lesson: target 8.2G → cargo clean → the twin 4/4; the RED
  was not code, the CI verdict OT=pass matches); **release_gap EXIT
  0**; **goal032 35/67 EXIT 0**; **gate-facts-sync EXIT 0**;
  gen_metrics/gen_reference --check OK. The release objects: the
  tags v0.33.2/v0.34.0 ABSENT, drafts 0, Latest v0.33.1. The §2.5
  package of the 02.11 window (the figure NOT pre-decided): (i)
  0.33.3 tail-ride — the ratchet-cadence continuation; (ii) 0.34.0
  feature-cut per ADR-0195 §7 form (b) — **the FIRST set where the
  0.34 gate formally passes** (the ratchets enforced=recorded + the
  RECORD line «closed 23/67 — live 35/67»; N=23 exceeded). GO
  verified-wait both windows. CI **49/49 GREEN round 1**.
- **№753** (gh#1399, this PR): the post-wave docs sync — REALITY
  §6.41, the PLAN-SUMMARY Wave 56 row, the counters from the
  generators (gen_reference --check OK — no REFERENCE drift: the
  №751 changes are registry-comment-only, the rendered surface is
  unchanged); docs/limitations.md — NO new honest-boundaries records
  (zero product changes; the divergences are the EXISTING-surface
  facts recorded here and in the registry comments, not new stub
  rows; the §16.0-D stub discipline held).

The machine numbers at the sync (2026-10-21, main @ f560c0b): the
sensitive surface **110/145 = 7586 bp** (the enforced floor **7586
bp**, only-up), **goal032 closed 35/67** (the gate BLOCKING —
ADR-0193 Accepted; the db group 3/3, the request_body group 0/1),
**typed 372/526 = 70.72% (precise 242/526 = 46.00%)**, the
**parameterized share 97/127 = 76.37%**, the **field-label share
59/61 = 96.72%**, **mirrors 6, dups 0, quorum 0/8**, **debt 8/0/15
(ignore 8, example_uncovered 0, dead_code 15)**, **REFERENCE
526/526 = 100.0%** (gen_reference --check OK), **[Unreleased] carries
the №729 Added + №723/№734/№735/№740/№744/№748 Changed + №724 Fixed
rows** (the merged naryads only — the №687 rule). gen_metrics
--check OK (the hand-written numbers match the generators — the №566
consistency holds).

The owner gates remaining (accumulated): the **26.10 window — the
0.33.2 prep-to-publication tail** (the cut [Unreleased] → [0.33.2]
with the stamp 2026-10-26 + the lockstep 0.33.1 → 0.33.2 + the regen
`release_notes.py 0.33.2` — the В56-tail naryad by the owner's
verbatim 21.10) **+ the tag → publication** (the OWNER's act —
ADR-0186 §4.5); the **goal032 lanes** — the (α)-result package (the
gate 7): (β) session_poll_wake/session_take_interrupt/backend_select/
inspect (the №543-form state unions), (γ) the opaque kinds (the 9
unnamed + series_make/media_source_capture + reflex_load/
reflex_bpe_load), the Secret spellings secret/generate_key/
memory_read (ADR-0195 §7 (c) — parked until the lanes open), the
parity fixes OUTSIDE the types movement — resolve_skill_index (the
type_name + the truncation field) and the GetField loudness (the
ADR-0194 loud-fail contract); the data-dependent trio
(call_llm_schema/config_load/find) and the candle-gated
reflex_generate — the semantics lanes; the **02.11 window figure**
(0.33.3 tail-ride vs 0.34.0 feature-cut — the §2.5 package in
gh#1398; the first set where the 0.34 gate formally passes); the
NLnet/Restack M1 (03.11.2026); №648 (01.12.2026); the zip-ignore
review (2026-11-17); the coverage threshold (№569 — PARKED by
ADR-0195 (c)); the second maintainer; the PAT rotation. RESOLVED
this cycle: the **movement №11** — lane α exhausted honestly (the
zero closures, the (α) fixation; the two divergence discoveries
pinned behaviorally); the **draft refreshed to the eight-row truth**
(№752 — the В55 dedup gap closed); the **0.33.2 figure fixed** (the
owner's verbatim 21.10).

### 6.42. Wave 57 post-wave record (naryad №760): the two lane divergences RESOLVED — the resolve_skill_index parity + the GetField loud-fail (the ADR-0194 (c) contract); the release line — the next-draft refreshed by the Wave 57 fact, the 02.11 figure mapped, GO verified-wait (main @ the naryad's PR, 2026-10-21)

The wave composition №756 → №759 → №760 (strictly last; the dispatch
gh#1411; the machine trigger — the В56 close + the 0.33.2 publication
tail consumed):

- **№756** (gh#1408, squash `21b3be9`): the [compiler] naryad — BOTH
  lane divergences from the №751 (α)-walk resolved by parity,
  semantically, OUTSIDE the types movement (ADR-0194 Запреты — the
  floors not moved, the return_type stays Unknown honest per the №669
  form). **(1) resolve_skill_index — ONE shape on both lanes:** the
  type_name `format!("SkillIndex_{dept}")` on BOTH (the TW canon; the
  VM fixed «SkillIndex» is gone — the vm.rs run-loop arm); the
  truncation field carries on BOTH — `CompiledSkillIndex.truncation`
  (bytecode.rs, `serde(default)` — the old .mbc safe), the compiler
  maps the declaration's `TruncationMode` (compiler.rs), the VM arm
  inserts the field in the TW spelling (`whole_skill_only` /
  `truncate_at_boundary`); absent stays absent on both (no fictitious
  default). **(2) The absent-field READ — loud-fail on both lanes**
  (ADR-0194 package A = (c)): the VM `GetField` `Unit` fallback
  removed on BOTH execution loops (`run()` and `execute_code_inner()`
  — was `unwrap_or(Value::Unit)`, the error masked as a value — the
  №604 class-4 shape); the `get_field` Err propagates verbatim (the
  same strings the TW FieldAccess always propagated). The ADR-0075
  ledger section «Resolved in Naryad №756» (rows 22–24 — the
  GetField loudness NOT in the original 21-case list, the walk's
  discovery 2); the registry comment carries the resolution anchors.
  The pins: NEW `tests/naryad_756_lane_divergences.rs` (8 behavioral
  pins, both lanes: the dept-qualified type_name probe + the negative
  fixed-form control; the truncation both-modes + absent-stays-absent;
  the absent-field and non-struct loud refusals with the pinned error
  strings; the present-field happy path `List|Float`); the №751
  divergence pin re-pinned to the parity fact (форма №740, same-PR):
  `n756_resolved_type_name_and_truncation_are_parity_across_lanes` —
  both lanes «String». CI **49/49 GREEN round 2** (round 1 48/49 — the
  honest note: the naryad's own CHANGELOG row matched the №604/№643
  security-dictionary surface on the compound «silently-wrong»
  bi-gram; the row re-spelled honestly without the bi-gram — the fact
  still named («the error masked as a value — the №604 class-4
  shape»), the release-block label NOT claimed deliberately — the
  naryad FIXES the divergence, it does not describe a live
  security-relevant boundary; the dictionary probe 0 matches after).
  The gates GREEN: gen_metrics --check OK (the README regen — the
  CHANGELOG-size fact, the №735 lesson); gen_reference --check OK
  (526/526); goal032 EXIT 0 (35/67 only-up); release_gap EXIT 0;
  gate-facts-sync EXIT 0; type_signature_share EXIT 0 (the floors
  untouched); unfreeze --gate-target 0.33.2 --strict --office-tests
  pass **EXIT 0 Overall GREEN** (twin naryad_429_memory_office_path
  4/4 on the tree; the toolchain 1.99.1 58cf8f9c3 — the sandbox
  reassembled after the snapshot rollback layer); fmt clean; clippy
  -D warnings clean; the interlock batteries green (751 4/4 after the
  re-pin, crosscheck 8/8 — the 58/58 goldens, vm_golden 24/24,
  inspect 4/4, phase19_22 20/20, 269 4/4).
- **№759** (gh#1409, squash `cdc5140`): the release line (the sixth
  window of the Monday cadence; the lekalo №752/№745/№741). The
  [Unreleased] carries the №756 Fixed row → the №754 prep-day
  next-draft STALE (№713: a stale draft is a lie) → **the regen**: the
  header the №759 form (the figure UNDECIDED — the owner's call to the
  02.11 window), the body = `release_notes.py Unreleased` VERBATIM
  (the diff probe **BODY VERBATIM OK** — №497). The **§2.5-map** of
  the 02.11 figure filed in the issue (the decision support, NOT the
  choice): (i) 0.33.3 tail-ride — 1 Fixed (№756), the low prep cost,
  the AFFECTS note mandatory (the VM absent-field reads refuse loudly
  now); (ii) 0.34.0 feature-cut — the ADR-0195 §7 criteria gate-ready
  (the ratchets enforced=recorded, the RECORD N=23 exceeded by the
  live 35/67) but the roster EMPTY and the minor-number cost (the
  gate consumed, the next feature-cut pushed to 0.35.0). The gates
  GREEN ×2 (before/after the refresh). **GO 02.11 — verified-wait**
  (both figures technically ready; the merge/tag/publication — the
  OWNER's act, §6.5/ADR-0186 §4.5). CI **49/49 GREEN round 1**.
- **№760** (gh#1410, this PR): the post-wave docs sync — REALITY
  §6.42, the PLAN-SUMMARY Wave 57 row, the counters from the
  generators; docs/limitations.md — NO new honest-boundaries records
  (the divergences are RESOLVED by №756, not declared live — the loud
  GetField is the landed contract, not a boundary; the §16.0-D stub
  discipline held).

The machine numbers at the sync (2026-10-21, main @ cdc5140): the
sensitive surface **110/145 = 7586 bp** (the enforced floor **7586
bp**, only-up), **goal032 closed 35/67** (the gate BLOCKING —
ADR-0193 Accepted), **typed 372/526 = 70.72% (precise 242/526 =
46.00%)**, the **parameterized share 97/127 = 76.37%**, the
**field-label share 59/61 = 96.72%**, **mirrors 6, dups 0**, **debt
8/0/15 (ignore 8, example_uncovered 0, dead_code 15)**, **REFERENCE
526/526 = 100.0%** (gen_reference --check OK — no drift: the №756
changes are semantics, the rendered signature surface unchanged),
**[Unreleased] carries the №756 Fixed row** (the merged naryads only
— the №687 rule). gen_metrics --check OK (the №566 consistency
holds).

The owner gates remaining (accumulated): the **02.11 window — the
figure** (0.33.3 tail-ride vs 0.34.0 feature-cut — the §2.5-map in
gh#1409; GO verified-wait) **+ the tag → publication** (the OWNER's
act — ADR-0186 §4.5); the **goal032 lanes** — the (α)-package (the
gate 7): (β) the state unions, (γ) the opaque kinds, the Secret
spellings (ADR-0195 §7 (c) — parked until the lanes open), the
data-dependent trio and the candle-gated reflex_generate — the
semantics lanes; the NLnet/Restack M1 (03.11.2026); №648
(01.12.2026); the zip-ignore review (2026-11-17); the coverage
threshold (№569 — PARKED by ADR-0195 (c)); the second maintainer;
the PAT rotation. RESOLVED this cycle: **both lane divergences** —
the resolve_skill_index parity (the dept-qualified type_name + the
truncation field on both lanes) and the GetField loud-fail (the
ADR-0194 (c) contract landed on both lanes and both VM loops); the
**next-draft refreshed** by the Wave 57 fact; the **02.11 figure
mapped** with the GO verified-wait.

### 6.43. Wave 58 post-wave record (naryad №763): the census of the 129 quiet Unit fallbacks — the loud operand contract for the 82 reachable-maskers (ADR-0194 (c)), 47 pinned honest; the agent-index honesty — llms.txt from the generators, the bridge byte-identity, the NEW blocking gate (main @ the naryad's PR, 2026-10-22)

The wave composition №762 → №763 (strictly last; the dispatch gh#1417;
the machine trigger — the В57 close + the direct owner trigger 22.10):

- **№762** (gh#1415, squash `8e631ab`, CI **49/49 GREEN round 1**): the
  [compiler] naryad — the census/classification of ALL **129
  `unwrap_or(Value::Unit)`** sites in vm.rs (the r29 anchor list was the
  INPUT, the live handlers the verdict; the kant-facts confirmed 1-в-1:
  129 sites / 7 `unwrap_or_default` / 4 `let _ =`; the GetField arms
  loud-by-result per №756). **(a) 47 pinned, semantics untouched**:
  (a1) 39 loud-through-result — the binop family (16+16 through
  `eval_binop`'s coded mismatch), `Contains` run-loop (2) through
  `eval_contains`, `CmpGt/Lt/Ge/Le` (16) through the `eval_cmp`
  catch-all — **only `CmpEq`/`CmpNe` answer Unit silently**
  (`Unit==Unit → Ok(true)`) — the (b) class; `GetField` pops (2) via
  `get_field` (№756), the rule-value `FieldAccess` read (3749),
  `LoadGlobalByName` (2) — every valid emission is a name-reference
  position whose miss lands the builtin's own loud `model not declared`
  (the semantic.rs «unchanged» contract — the text NOT touched);
  (a2) 6 documented-soft — the `IndexAccess` in-value lookups
  (out-of-range index / missing key = valid-program events);
  (a3) 2 — the `EndValueExpr` register-stack pops. **(b) 82
  reachable-masking — FIXED loud** (run 44 / inner 37 / rule-read 1):
  the `.mbc` delivery path is live (`deserialize` — NO version or
  integrity gate, bytecode.rs) so underflow/slot-misses are reachable
  via a desynchronized or corrupt program; the silent Unit flowed into
  results, stores, branches and memory — the №604 class-4 shape; the
  strongest forms named in the audit: the quiet `Bool(true)` from the
  empty-stack `CmpEq`, the `"()"` garbage the memory family wrote, the
  silent `Ok(Unit)` body value. Every class-(b) site refuses naming the
  instruction and the fact (`VM: stack underflow at …`); the arity
  loops name the callee. Zero observable change on valid bytecode (the
  compiler guarantees every operand: `expr; Return` in every context —
  compiler.rs:3593, the №582 epilogue, the constructor-paired slots);
  ALL fixes **line-count-neutral 1:1** — the registry `vm.rs:` anchors
  (17, макс. 3008-3009) hold without edits (the same-PR rule, форма
  №740). The boundary notes recorded honestly: the MakeFluid-confidence
  Float-fallback (1271) is outside the 129 census; the StoreGlobal
  out-of-range silent drop (`if *slot < len`) is adjacent, not one of
  the 129; the run/inner `Contains`+`StartsWith` soft-arm loop
  divergence on VALID bytecode is the ADR-0075 family — out of the
  underflow class, untouched. Pins: NEW
  `tests/naryad_762_unit_fallback_census.rs` (7: both loops with the
  named refusals, the slot-bounds, the inner Return + the body-epilogue
  tail, the (a1) behavioral pins — Add/CmpGt/GetField refuse with the
  evaluator's text and NO underflow text, the (a2) soft pins, the
  happy-path negative controls). The interlocks GREEN: №751 (4), №756
  (8), №757 (5), №758 (5), №736 (6), №744 (8), №730 (7), №194 (5),
  №146/№415/№266/№264/№385/№328; **lib 876/876**. The gates GREEN:
  gen_metrics --check (the README regen — the CHANGELOG-size fact),
  gen_reference 526/526, goal032 35/67 exit 0, release_gap,
  gate-facts-sync, type_signature_share (the floors untouched —
  semantics, NOT typing), sensitive 7586 bp at the floor, debt 0/0/15,
  blocking-checks-sync 32 cells, the stop-list does not grow. The
  [Unreleased] carries the №762 Fixed row with AFFECTS (форма №756).
- **№763** (gh#1416, this PR): the agent-index honesty (the R-01
  finding). **llms.txt refreshed FROM the generators**: 409 → **526
  builtins**, 214 → **253 examples**, 132 → **187 ADR**, №1-№289+ →
  **№1-№763** (the merged-history №505-grammar scan — the naryad
  sequence machine); the three missing canonical rows ADDED
  (docs/spec/tools.md, docs/limitations.md, docs/PLAN-SUMMARY.md);
  the CLAUDE.md row note re-spelled (the manual sync → the gate).
  **CLAUDE.md restored byte-identical to AGENTS.md** (was 8958 vs
  10703 bytes — the drift the r29 contour measured; the №676 A5
  content-source form). **The NEW blocking gate**
  `scripts/ci/agent_index_gate.py` (the gen_metrics --check form): the
  counters == the generators (the SAME machines — gen_metrics
  count_total_builtins/count_adrs/count_examples + the git-history
  naryad scan), the bridge byte-identity, the llms.txt link targets
  exist; `--self-test` — the tamper traps spring (the stale counter,
  the broken bridge, the dangling link); wired as the **agent-index
  (blocking) CI job** (fetch-depth: 0 — the git-log machine) + the
  **blocking_checks.tsv cell** (33 cells: blocks=28) pinned by NEW
  `tests/naryad_763_agent_index_gate.rs` (the gate green + the traps +
  the Rust byte-identity twin). docs/limitations.md — NO new
  honest-boundaries records (the underflow refusals are the LANDED
  contract of №762, not a live boundary; the agent-index drift is
  FIXED by this naryad, not declared live — the §16.0-D stub
  discipline held).

The machine numbers at the sync (2026-10-22, main @ 8e631ab): the
sensitive surface **110/145 = 7586 bp** (the enforced floor **7586
bp**, only-up), **goal032 closed 35/67** (BLOCKING), **typed 372/526
= 70.72% (precise 242/526 = 46.00%)**, the **parameterized share
97/127 = 76.37%**, the **field-label share 59/61 = 96.72%**,
**mirrors 6, dups 0**, **debt 8/0/15**, **REFERENCE 526/526 =
100.0%**, the agent-index gate GREEN (526/253/187/№763 + the bridge
byte-identity), **blocking cells 33**. gen_metrics --check OK.

The owner gates remaining (accumulated): the **02.11 window — the
figure** (0.33.3 tail-ride vs 0.34.0 feature-cut — the §2.5-map in
gh#1409; GO verified-wait) **+ the tag → publication** (the OWNER's
act — ADR-0186 §4.5); the **goal032 lanes** (the gate 7): (β) the
state unions, (γ) the opaque kinds, the Secret spellings (ADR-0195
§7 (c) — parked until the lanes open), the data-dependent trio and
the candle-gated reflex_generate; the NLnet/Restack M1 (03.11.2026);
№648 (01.12.2026); the zip-ignore review (2026-11-17); the coverage
threshold (№569 — PARKED by ADR-0195 (c)); the second maintainer
(§6 п.7 of the audit — the CODEOWNERS src/ coverage); the PAT
rotation. RESOLVED this cycle: **the 129-census** — the loud operand
contract pinned for the 82 reachable-maskers, the 47 pinned honest;
**the agent-index drift** — llms.txt from the generators, the bridge
byte-identity, the blocking gate wired.

### 6.44. Wave 59 post-wave record (naryad №769): the audit-driven wave — the external audit 21.10 landed as five naryads the same day (the S-1b XSS closure, the recursive diff-fuzzer, the READBACK_PAIRS paper trail, the 0.31 narrative + the loud mirror counter); the audit episode fixed in the record (main @ the naryad's PR, 2026-10-22)

The wave composition №765 → №766 → №767 → №768 → №769 (strictly last;
the dispatch gh#1423; the trigger — the direct owner trigger 22.10
«Проверь замечания по аудиту, напиши и залей наряды» + the external
audit METALOGOS_AUDIT_9fa9821.md, 21.10.2026, v0.33.2, оценка 9.5/10;
метод: чтение кода + прогон CI-скриптов):

- **№765** (gh#1418, squash `3d75b1b`, CI 50/50 GREEN): the [security]
  naryad — the **S-1b XSS closure** (the audit H-1): the audit
  scenario `let page = render("Comment", j.author, j.text)` →
  `respond_html(page)` produced no finding and the raw slot carried
  request data into the page. Three layers: `builtin_render` computes
  the RAW-slot param names from the template body (the ONE
  `template_scan::raw_param_positions` scan — four consumers) and
  requires `Value::Html` for them like the safe slots (the
  `RAW_SLOT_NOT_HTML` refusal); the html_label BINDING walk resolves
  the raw positions through the shared resolver and answers
  `Labeled(String, Untrusted)` at raw positions; render is OUT of the
  blanket sanitizer lists and POSITION-AWARE in the label engines.
  Pins: `tests/naryad_765_s1b_let_form.rs` (7) + 2 inline units; the
  interlocks hold (№685 24/24, №695 4/4, №686 3/3, №448 11/11, n98
  8/8). AFFECTS: the render-as-sanitizer reliance on RAW slots is
  refused (static findings + runtime refusals); the all-escaped
  templates byte-for-byte unchanged.
- **№766** (gh#1419, squash `3d0dcdc`, CI 50/50 GREEN): the [testing]
  naryad — the **recursive diff-fuzzer** (the audit H-2: `gen_stmts`
  carried `_depth` it never used; the «form A inside form B» class
  was unmeasurable by construction). `gen_stmts(depth)` picks among
  the SIX nesting forms and recurses (п.1); every generated block
  appends a deterministic TAG to the trace accumulator — a dropped
  block is a TW/VM divergence by construction (п.2); the
  each-in-match pins land in `tests/n1336_vm_each_in_if.rs` (п.3);
  the nesting-pairs metric {top, if, while, each, each_idx, match} ×
  {flat, if, while, each, each_idx, match} = **36/36 achieved**, the
  floor in `scripts/ci/fuzz_nesting_pairs_baseline.txt` (the NEW
  only-up ratchet, п.4); the Each-arm mutation probe asserts the
  sensitive share ≥40% (п.5).
- **№767** (gh#1420, squash `75e144a`, CI 50/50 GREEN): the [process]
  naryad — the **READBACK_PAIRS paper trail** (the audit M-1): the
  limitations row (the Static Analysis section; the live anchor
  `semantic.rs:4876` — the issue's 4721 was the audit snapshot @
  `9fa9821`, the №765/№766 merges shifted it, the fact 1-в-1) + the
  risk-review verdict **ACCEPTED with the standing boundary** in
  ADR-0154 §11 (position-only, domain-matched, provenance-
  conservative, quarantine-proof; the revisit conditions pinned) +
  the cross-links both ways + the [Unreleased] Changed row + the
  README regen. **Honest episode:** the round-1 CI agent-index
  (blocking) RED — the NEW №763 gate caught the llms.txt range drift
  (№1-№766 vs the machine's №1-№767 — the naryad's own commit); the
  same-PR refresh (959ddfe), the gate green — the discipline works
  as designed, zero-delay by substance.
- **№768** (gh#1421, squash `f4a7750`, CI 50/50 GREEN): the [hygiene]
  naryad — the **0.31 narrative** (the audit M-2, the third carry:
  «это пять минут работы») — the Note subsection in the historical
  [0.32.0] (the gh#1350 Note form) + the same note in the §6.29
  record (the historical sections gain a line, not a rewrite) — and
  the **mirror_counter CWD-independence** (the audit L-1, reproduced
  1-в-1: from `scripts/ci/` the counter printed a silent 0/exit 0 —
  a false green for an only-down metric): the scan root from
  `Path(__file__).parents[2]`, any CWD prints the honest 6, a
  missing src/ fails LOUD (exit 2); the CI invocation and the metric
  value untouched.

**The audit episode — the wave's origin, the verification, and the
process lesson:**

- The external audit (METALOGOS_AUDIT_9fa9821.md, 21.10.2026, v0.33.2,
  9.5/10) landed five findings: H-1 (High — the S-1b XSS let-form
  gap), H-2 (High — the fuzzer's flat generator), M-1 (Medium — the
  READBACK_PAIRS exemption without the limitations/risk-review
  trace), M-2 (Medium — the 0.31 narrative missing), L-1 (Low — the
  mirror_counter CWD false green).
- The r30 verification contour (GET-only + the shallow clone + the
  CI-script runs at HEAD `9fa9821` = the audit's revision 1-в-1)
  confirmed ALL findings; the §2 metrics reproduced 1-в-1; the two
  line-drifts (expr_is_string_shaped 10079→10238, READBACK_PAIRS
  4848→4721) — the snapshot noise, the facts intact. The auditor's
  amendment to the 16.10 report is ACCEPTED in the same record: the
  sensitive_surface_share.py item is WITHDRAWN (the script works from
  the root; nothing fixed there — the №768 boundary honored).
- **The findings became issues the same day** (gh#1418–gh#1421, the
  dispatch gh#1423) — the wave's process precedent (the audit §1
  lesson, below).
- **The process lesson (the audit §1, accepted):** the 0.32 goal was
  OVERSHOT (35/67 at the ≥20 target — the machine), yet the S-1b
  (High) reached 5 days and 3 releases without movement, and the
  gh#1336 nesting class was found by a PRODUCTION CONSUMER, not by
  the fuzzer. The counters' honest wall is not the whole honesty: a
  coverage number can rise while a HIGH finding idles. The В59
  precedent: **external/audit findings become issues IMMEDIATELY**
  (the №562 tag↔main gate sees only the «### Security» [Unreleased]
  records — a finding outside the tracker is invisible to every
  gate; the tracker is the sensory organ).
- **Limitations (форма №736): no NEW boundaries this wave.** The
  №765 S-1b closure extended the EXISTING raw-contract enforcement —
  the №685 limitations row already carries the raw-positions-conduct
  contract; the READBACK_PAIRS row (№767) documents the existing
  exemption (the audit M-1 demanded the trace, not a new boundary).

**The wave numbers (the machines @ the merge head; the ratchet
floors only-up):** typed 372/526 = 70.72%; precise 242/526 = 46.00%;
parameterized 97/127 = 76.37%; field-label 59/61 = 96.72%; sensitive
110/145 = 7586 bp (the floor 7586 only-up); goal032 **35/67**
BLOCKING (ADR-0193; the fixed-list counter — the audit's overshoot
fact); untyped 32; долг 8/0/15; ops-pairs 6; mirror 6; REFERENCE
526/526 = 100%; the NEW ratchet (№766): the fuzzer nesting-pairs
**36/36** only-up. The consistency gates: gen_metrics --check OK,
gen_reference --check OK (no drift — no signature surface moved),
agent_index OK, honest_boundary EXIT 0, gate-facts-sync OK.

**The owner gates remaining (the dispatch gh#1423 §5, the status on
the record date):** the **02.11 window** — the release figure
(0.33.3 tail-ride vs 0.34.0 feature-cut; the §2.5 map
gh#1398/gh#1409) + the tag → publication (ADR-0186 §4.5 — the
owner's act); the **goal032 lanes (β)/(γ)** — the owner verdicts
(the package in the №751 report; gate 7); **NLnet M1 03.11**; the
**zip-ignore review 17.11**; **№648 01.12**; the **second
maintainer** — УСИЛЕН аудитом (§6 п.7: the CODEOWNERS coverage for
src/audit.rs, src/html_label.rs, src/compiler.rs — the critical
files uncovered, the candidate is the owner's); the №561 pool
refill (9/10); the PAT rotation. PARKED: the №569 threshold (ADR-0195
(c)). RESOLVED this cycle: the **S-1b XSS class** (the 5 code anchors
+ the let-form battery); the **fuzzer recursion blindspot** (the
36/36 nesting floor); the **READBACK_PAIRS trace**; the **0.31
narrative** (the third carry); the **mirror counter false green**.

### 6.45. Wave 60 post-wave record (naryad №775): the prod-bug wave — gh#1430 fixed same-day (the consumer's TW/VM divergence, the second case of the gh#1336 class: the try×pattern-call unwind wiped the frame locals on the VM) + the release line refreshed (the 02.11 window: the 8-row draft, the tail-ride recommendation) + the second-maintainer record prepared verified-wait (main @ the naryad's PR, 2026-10-22)

The wave composition №772 → №773 → №774 → №775 (strictly last; the
dispatch gh#1439; the trigger — the direct owner trigger 22.10 «Где
волны исполнены, проверяй и заводи следующую волну нарядов»):

- **№772** (gh#1435, squash `7069d4c`, CI 50/50 GREEN раунд 2): the
  [compiler] naryad — the **gh#1430 prod-bug fix** (fosved-acc
  АСС-1.9): `try` catching a pattern-call error wiped the caller's
  frame locals on the VM — the unwound CallPattern left its CallFrame
  installed (the `?` propagation skipped the cleanup the success path
  performs), every later LoadLocal/StoreLocal resolved against the
  stale base_bp, the pre-try locals read Unit (`Unit Sub Float` /
  `Float Sub Unit`; the fosved-acc duty monitor went dark during an
  outage on the serve VM lane; TW kept the values). The fix: BOTH
  TryEval arms (the run loop + the inner loop — форма №762) snapshot
  the stack/call_stack depths before the guarded code and restore both
  on catch — the `run_deny_handler` (№392) discipline; the loud
  contract (ADR-0194 (c)) unchanged, TW untouched. Pins:
  `tests/n1430_try_pattern_unwind.rs` — 6 tests, **4 RED before the
  fix** (verified by a stash-run on unpatched main) + 2 negative
  controls; the differential fuzzer carries the class shape (the bug's
  ask 3: `try boom()` → catch → a pre-try trace append; with the fix
  stashed it diverged loudly, with the fix 13/13). AFFECTS (форма №756
  in the row): programs relying on the Unit wipe see the TW values.
  **The №772 CI episode:** раунд 1 docs-metrics RED — the README
  generated block went stale (the CHANGELOG size 97→99 KB from the
  onaryad row); the same-PR regen `9f58208` (the fact key WITH checker
  form, №735).
- **№773** (gh#1436, squash `3eef378`, CI 50/50 GREEN раунд 2): the
  [release] naryad — the **02.11 draft VERBATIM-refresh** ([Unreleased]
  @ `7069d4c`: 8 rows — №756, №762, №763, №765, №766, №767, №768,
  №772; the draft = `release_notes.py Unreleased` byte-verbatim, diff
  EXIT 0, №713) + the **§2.5 map posted to gh#1436**: 0.33.3
  tail-ride RECOMMENDED (the security S-1b + the prod bug gh#1430 ride
  before the EMPTY 0.34 roster; the ADR-0195 gate criteria are
  formally met — sensitive 7586 bp ≥ 6844, goal032 35/67 BLOCKING —
  but the (β)/(γ) lanes are behind owner gate 7) + the go/no-go (6
  criteria). **The №773 CI episode:** раунд 1 agent-index RED — the
  machine saw the naryad's own commit №773 vs llms.txt №1-№772; the
  same-PR refresh `4973fcf` (the №767 lesson applied).
- **№774** (gh#1437, no PR — **verified-wait**): the [process] naryad —
  the second-maintainer record: the ready ADDITIVE CODEOWNERS block
  (src/audit.rs, src/html_label.rs, src/compiler.rs — the audit 21.10
  §6 п.7; the №471 perimeter untouched) + the candidate criteria
  (Rust review + security surface + per-PR SLA + no-conflict) posted
  to gh#1437. The commit is FORBIDDEN without the owner verbatim
  (форма №755; the self-assignment forbidden) — the gate stands.

**The gh#1430 episode — the consumer finds the divergence, the second
case of the gh#1336 class («потребитель раньше машины»):**

- The bug (fosved-acc АСС-1.9, naryad fosved-acc#33, step A.2, opened
  22.10 02:58Z): the minimal p3/p2 shapes + the boundary (the
  inline-builtin try keeps the locals — the corruption is specific to
  the pattern-call unwind) + the serve e2e; reproduced by the executor
  on the EXACT v0.33.2 release binary (sha256 `48b98167…` verified
  against SHA256SUMS).
- The same-day pipeline: the issue 02:58Z → the dispatch 06:46Z (the
  owner trigger) → the fix merged ~14:25Z — **the class lifetime under
  a day** (the В59 process lesson applied: the tracker is the sensory
  organ; the external-consumer bugs go the same road as the audit
  findings).
- **The honest refutation (форма №716):** the secondary symptom (the
  double `[try] caught error` print on VM) does NOT reproduce — the
  exact v0.33.2 binary and main, the CLI (p3, the two-catch p3b, the
  nested p3c) and the serve path, catch-count = 1 everywhere; one
  eprintln per TryEval execution by construction. The likely origin:
  the consumer's merged stderr across two engine runs. No second root
  — no separate edit; the primary fix does not touch the print.
- **The honest finding (вне границ наряда, в отчёт):**
  `gen_classification.py --goal032 --list` on a clean HEAD produces a
  non-default diff (REFERENCE.md +1, builtins_classification.rs
  +9/−26 — the latent generator drift, the №744 class). The canonical
  goal032 gate (`sensitive_surface_share.py --goal032`) is GREEN — the
  blocking surface is not affected; the regeneration is a separate
  decision (not executed inside №772).
- **Limitations (форма №736): no NEW boundaries this wave.** The №772
  fix RESTORES parity (the AFFECTS note documents the behavior change
  for the programs relying on the Unit wipe); the docs surface of the
  wave is the draft refresh (№773) and the report (№774).

**The wave numbers (the machines @ the merge head; the ratchet floors
only-up):** typed 372/526 = 70.72%; precise 242/526 = 46.00%;
parameterized 97/127 = 76.37%; field-label 59/61 = 96.72%; sensitive
110/145 = 7586 bp (the floor 7586 only-up); goal032 **35/67** BLOCKING
(ADR-0193); untyped 32; долг 8/0/15; ops-pairs 6; mirror 6; дубли 0;
REFERENCE 526/526 = 100%; the fuzzer nesting-pairs **36/36** only-up.
The consistency gates: gen_metrics --check OK, gen_reference --check
OK (no drift — no signature surface moved), agent_index OK, honest_boundary
EXIT 0, gate-facts-sync OK.

**The owner gates remaining (the dispatch gh#1439 §5, the status on
the record date):** the **02.11 window** — the release figure (the §2.5
map gh#1436 recommends **0.33.3 tail-ride**: the 8-row [Unreleased],
the ADR-0195 criteria formally met, the 0.34 roster empty) + the tag →
publication (ADR-0186 §4.5 — the owner's act); the **second
maintainer** — №774's ready CODEOWNERS record + the candidate criteria
wait the owner verbatim (форма №755); the **goal032 lanes (β)/(γ)**
(gate 7); **NLnet M1 03.11**; the **zip-ignore review 17.11**; **№648
01.12**; the №561 pool refill (9/10); the PAT rotation. PARKED: the
№569 threshold (ADR-0195 (c)). RESOLVED this cycle: the **gh#1430
unwind class** (the try×pattern-call frame wipe — found by the
consumer, fixed same-day, the fuzzer covers the shape); the **02.11
draft staleness** (the 8-row verbatim truth); the **second-maintainer
record** (the ready form, the gate stands).
