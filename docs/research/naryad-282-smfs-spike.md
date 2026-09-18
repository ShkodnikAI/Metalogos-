# Naryad #282 — Spike "memory as a virtual FS" (SMFS analog): report and verdict

> **Status:** verdict **GO** (verdict gate issue #331: decided by the executor based on the spike result)
> **Date:** 2026-09-13 · **Tasking:** issue #331 · **Dispatch:** #332 (M2 package) · **Base:** main `8fb59bf`
> **Spike branch:** `naryad-282-smfs-profile` (`f5adc71`+2, draft PR #349 — never merged; only this document and the ADR-0139 draft land in main)
> **Environment:** rustc/clippy 1.98.1, Linux x86_64, 2 vCPU container
> **Executor:** Super Z (agent) under the naryad contract AGENTS.md §8; spike format #271/#275

## 1. Tasking and fact cross-check against the code (AGENTS.md §1)

Tasking facts confirmed against the main `8fb59bf` code:

- **#281 user_profile** (`src/builtins/profile.rs`) — a deterministic distillation of a container (`container:<c>:<bucket>:<key>` → Struct with `static`/`dynamic`/`buckets`), WITHOUT LLM, with a cache (KV-entry generation + mtime). The source for the SMFS profile — confirmed.
- **File builtins** (`src/builtins/io.rs`): `read_file`/`write_file`/`append_file`/`delete_file`/`file_exists`/`list_dir` — all go through `sandbox_path`/`sandbox_path_ex` (#131 text-checks + canonicalization + prefix; #252 safe-to-use path + O_NOFOLLOW; #254 soft/loud parsing). The interception point — before `sandbox_path` — found.
- **Sandbox gate** (`src/interpreter/execution.rs`, Phase 7.5): `forbidden=["filesystem"]` cuts off the six file builtins at two of the THREE builtin dispatch points — see §5 (finding).
- **Sandbox declarations** (`SandboxDecl`, `Interpreter::set_active_sandbox`) — template `tests/phase75_contract.rs` C1.
- **Taint/canary**: `canary_insert_core`/`canary_check_core` (#284) — public in `metalogos::builtins`; the redact carve-out for markers is in place.
- **ADR-0096** — spawn_blocking in serve routes (not about the language); "read-generate" is unrelated to the prototype (§4.4).

## 2. Prototype (spike branch)

A virtual read-only `sm:` space on top of kv memory; access via the **stock** file builtins, interception in `src/builtins/io.rs` **before** `sandbox_path`:

| Path | Operation | Result |
|---|---|---|
| `sm:` | `list_dir` | sandbox-root db files containing the `kv_store` table (scan ≤512 files, sorted) |
| `sm:<db>` | `list_dir` | DB containers (`SELECT DISTINCT`, sorted) |
| `sm:<db>/<container>` | `list_dir` | `profile.md` + buckets with a `/` suffix |
| `sm:<db>/<container>/profile.md` | `read_file` | a deterministic digest: keys + the first 160 chars of the value + `(full: sm:...)` pointers; empty sections are not rendered |
| `sm:<db>/<container>/<bucket>/<key>` | `read_file` | the full value of the entry (byte-for-byte); a missing one — softly `""` (#254 contract) |
| any `sm:*` | `write_file`/`append_file`/`delete_file` | loudly `[SMFS_READ_ONLY]` |

Properties closing the spike questions:

- **No sandbox extension**: virtual paths never touch the disk at all (except opening the profile DB — the stock `sandbox_path_ex(ForRead)`); the active-sandbox gate `forbidden=["filesystem"]` sits BEFORE the interception (test `sandbox_forbidden_filesystem_beats_smfs`).
- **Two-way prefix reservation**: a path with `sm:` is handled only virtually — real `sm:*` files are unreachable and uncreatable through the builtins (test). `..` — loudly `[SANDBOX_VIOLATION]`; empty components / `:` in a container / a third component other than `profile.md` / depth > 4 — loudly `[SMFS_BAD_PATH]`.
- **Determinism**: the render is one SQLite query + formatting, WITHOUT LLM; the #281 cache is reused.
- **Registry not extended** (404 builtins unchanged), the file-builtin arities were not changed — the prototype is invisible to bytecode indexes.

## 3. Measurements (Go criterion)

Demo container `demo`: 20 entries in 4 buckets (`static`/`dynamic`/`notes`/`projects`), values 500–1200 chars (~850 average). The "traverse all files" path — the full text of all 20 entries; the SMFS path — `cat profile.md` + 2 targeted reads of the entries of interest. Tokens ≈ chars/4 (a proxy without a tokenizer — documented honestly; the criterion is phrased in terms of context-size reduction).

```
[smfs-demo] records=20, traversal=17068 chars (~4267 tokens),
            smfs(profile.md + 2 reads)=4665 chars (~1166 tokens),
            reduction=3.66x
[smfs-demo] profile.md = 4164 chars
```

**Go criterion "reduction ≥ 2×": PASS (3.66×)** — consistent with the SMFS-claimed "3× fewer tokens" on Claude. Dependence on the data profile: the longer the entries, the larger the gain (160-char truncation); on short entries (<160 chars) there is no gain — the profile is almost equal to the traversal. This is an honest boundary: the SMFS gain exists where memory consists of substantive (long) entries.

**Go criterion "stock builtins without sandbox extension": PASS** — `read_file`/`list_dir`/`file_exists` work on top of `sm:` with unchanged signatures and no new transport; the sandbox disk path is not engaged.

## 4. Answers to the spike questions (issue #331)

### 4.1. Write semantics of a memory-path

In the prototype the mount is **read-only**: a write through the file builtins is loudly rejected (`[SMFS_READ_ONLY]`). The SMFS semantics "a write to a memory-path generates a memory" is redundant and dangerous for Metalogos: writing to memory already exists (`memorize`/`kv_set`) with ready-made boundaries (the #274 taint contour, #281 cache invalidation, the #280 forgetting ledger); a second writer via `write_file` would create a dual channel with split invariants. The spike answer: **reads are FS-native; writes remain with the memory API**. If the product wants sugar (`write_file("sm:db/c/b/k") ≡ kv_set`) — a separate naryad with an explicit gate and invalidation; not part of the Tier-3 prototype. This is NOT a No-Go criterion ("a separate transport is required" refers to reading — the transport is stock).

### 4.2. Conflict with the sandbox (virtual paths ≠ real files)

Resolution — **two-way prefix reservation**: (1) any `sm:*` path is handled virtually and never reaches the disk (interception before `sandbox_path` — no #252 TOCTOU surface by construction); (2) real `sm:*` files are unreachable and uncreatable through the builtins — the "virtual path vs real file" collision is excluded. `..` in a virtual path — loud, as in the real sandbox; the active sandbox `forbidden=["filesystem"]` is stronger than the interception (virtual access is filesystem access too). Incidentally see §5: the spike uncovered and closed a pre-existing gap of the same gate.

### 4.3. Taint boundaries (the #274 secrets do not get into sm-files)

The export reads the same DB rows as the stock `user_profile` #281 — **the prototype creates no new taint channel**. Verified: a canary marker written into a kv value is detected by `canary_check` both in the full entry read and in the `profile.md` digest (test `canary_detection_survives_smfs_export`) — the #284 detection works through the export. Honest boundaries: (a) a value written to memory is plaintext in the DB (the existing #281 boundary, not new); (b) `redact` is not applied on sm reads — just as on any `read_file` (the sanitizer sits at the #274 sinks, not at sources). ADR-0139 recommendation: taint marks on records created from tainted expressions — a product question for a future naryad.

### 4.4. Blocking ≤ 1 chunk (ADR-0096)

The profile render is synchronous, deterministic, without LLM and without streamed generation: blocking is bounded by the duration of a single builtin call (a SQLite query + formatting — milliseconds on the demo corpus). The ADR-0096 contour (spawn_blocking routes) is not engaged for sm reads; if the product later grows an LLM profile summary (a real "read-generate") — it must go through the #275 streaming contour and remain bounded-per-chunk; that is a separate ADR question, outside this spike.

## 5. Spike finding: sandbox-gate gap (pre-existing; loud)

The reconnaissance of the "active sandbox beats interception" test uncovered a **pre-existing Phase 7.5 gap**: of the interpreter's three builtin dispatch points, the call point INSIDE expressions (`eval_expr_with_env` → `Expr::FnCall` — `let x = read_file(...)`, arguments, return) **had no** filesystem gate — `sandbox forbidden=["filesystem"]` was bypassed by any file call inside an expression (the invoke- and ModuleAccess-paths were gated). The fix — gate alignment (commit `64e0a56` on the spike branch) + a test. Honest note: the fix reaches main only together with the owner's decision (the spike branch is never merged) — a follow-up naryad-hotfix is recommended. 4 clippy lint findings of the vec combination (`vector.rs:56` transmute, `learnable.rs` ×3) are pre-existing on main, CI does not gate them (no vec-clippy job), and they are unrelated to the spike.

## 6. Edge cases (covered by tests, tests/naryad_282_smfs_spike.rs — 13)

- empty container: digest `records: 0` with no sections; `file_exists` → false (a container exists ⟺ there are entries);
- a missing entry — softly `""` (#254 contract); a missing DB/container for `list_dir` — loud;
- non-sqlite files and sqlite without `kv_store` are not mounted into `sm:`;
- `..`/empty components/`:`-in-container/depth>4 — loud; `read_file` of a directory and `list_dir` of a file — loud with codes `SMFS_IS_DIR`/`SMFS_IS_FILE`;
- reservation: `write_file("sm:real.txt")` → loud, no file appears on disk;
- sandbox `forbidden=["filesystem"]` blocks `sm:` paths too (the gate sits before the interception);
- TW/VM parity: identical byte-for-byte output (`smfs_read_is_identical_in_tw_and_vm`).

Prototype boundaries (documented honestly): the db component — only a sandbox-root file (a single component); the root scan in `list_dir("sm:")` — prototype navigation (in the product — a registry of mounted DBs); the digest parameter 160 chars; the token measurement — a chars/4 proxy; the O-2 on_write hooks do get to see the event before the loud write rejection in `sm:` (an event on a rejected operation — a boundary).

## 7. Verification

- fmt ✅; clippy **bare/candle/vision** `--all-targets -- -D warnings` ✅ (exactly the CI form; lesson from #281); the vec combination — 4 pre-existing main lints (§5, not CI-gated)
- lib 640/640 ✅; n282 13/13 ✅
- regression: n281 23/23, n280 28/28, n272 9/9, n284 28/28, phase19_22 14/14, n254 8/8, n126 4/4, registry_arity ✅

## 8. Verdict: GO

| Criterion (issue #331) | Fact | Outcome |
|---|---|---|
| profile.md accessible via stock builtins without sandbox extension | interception before `sandbox_path`; the active sandbox is stronger; two-way reservation; arities/registry untouched | PASS |
| Measurements: token reduction ≥ 2× vs "traverse all files" | **3.66×** on the demo container (20 entries × 500–1200 chars) | PASS |
| No-Go "a separate transport is required" | the transport is the stock read_file/list_dir/file_exists | did not trigger |
| No-Go "the taint model breaks" | the export is the same class as user_profile #281; canary detection through the export (test) | did not trigger |

The executor's decision per the verdict-gate mechanics: **GO** — a "memory-native filesystem" is viable as a grant narrative; the **ADR-0139** draft is attached to this PR (docs branch `naryad-282-spike-report`). Product implementation (a separate naryad): a registry of mount points, the digest parameter, the write-sugar decision, taint marks on records, the follow-up sandbox-gate hotfix (§5).

## 9. Spike artifacts (stay on the branch, never merged)

- `src/builtins/smfs.rs` — the prototype core (the `sm:`-path parser, the digest render, navigation, the read-only gate);
- intercepts in `src/builtins/io.rs` (6 file builtins, before `sandbox_path`);
- the gate fix in `src/interpreter/execution.rs` (§5);
- `tests/naryad_282_smfs_spike.rs` — 13 tests + the demo measurement of the Go criterion;
- draft PR #349 — the proof artifact (never merged).
