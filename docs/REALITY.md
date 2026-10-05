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

- The pre-M1 delivery (№577, gh#983): the NLnet/Restack grants sync
  landed 15 days before the 2026-10-20 deadline — the metrics one-pager
  (the fresh snapshot, every number generator-read on the day), the CBM
  pilot extract (the 2026-10-03 measurement), the M1 checklist. The
  publication stays with the owner (M1 2026-11-03).
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
