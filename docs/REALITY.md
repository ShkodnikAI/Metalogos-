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
