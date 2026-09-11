# Changelog

All notable changes to the Metalogos project.

## [Unreleased]

### Added — fuzz: real targets for the .mbc load path and the url decoder (Naryad #256 — part A; CI smoke job pending owner)

- fuzz: only `fuzz_target_1` (parser::parse) existed — the `.mbc` load path and the query-string percent-decoder, both fed by external bytes, had zero fuzz coverage, and CI never fuzzed (external audit 2026-09-11, issue #259). New targets: `fuzz_target_bytecode` — arbitrary bytes through `Program::deserialize` (the exact call `mlog run file.mbc` makes: bincode legacy + the №146 size limit; contract: Ok or loud Err, never a panic/hang; malicious-bytecode DISPATCH is intentionally not executed — the VM has no step-budget API, an infinite loop would hang the fuzzer; revisit when one lands); `fuzz_target_url_decode` — `url_decode_fallback` (now `pub`, the fuzz surface) with panic-freedom on any UTF-8 input + ASCII roundtrip; the NON-ASCII roundtrip is deliberately not asserted (the decoder reassembles bytes as chars — multibyte UTF-8 yields mojibake; that RFC 3986 correctness question is naryad №257's deliverable). `fuzz/Cargo.lock` committed for reproducible fuzz deps. The fuzz-smoke CI job (nightly + cargo-fuzz, 120s/target) is PART B and requires the owner: the session token lacks the `workflow` scope, git refuses to push `.github/workflows` changes — the ready-to-apply job block is in PR #284; issue #259 stays OPEN until the smoke job is green (the job is non-blocking by design: a 15th/16th BLOCKING job would break the 14-blocking invariant across README/docs). Merged as PR #284 (PR number ≠ naryad number).

### Added — server: explicit 2 MiB request body limit instead of the implicit axum default (Naryad #255)

- server: the request body limit is now an explicit, owned constant — `REQUEST_BODY_LIMIT_BYTES = 2 MiB` (2 097 152 bytes, `src/server.rs`), applied via `DefaultBodyLimit::max` in `build_router` (TW and VM backends alike; larger bodies get HTTP 413 Payload Too Large). Before №255 the cap was axum 0.8's implicit ~2 MB default — the source of truth lived in a foreign crate and would silently change on an upgrade, and neither threat-model nor REFERENCE answered "what is the max request size". Justification recorded in the constant's doc comment: 2 MiB fits JSON route bodies (configs, documents, LLM payloads); bigger uploads are a separate decision (streaming/multipart), not a silent ride along a dependency upgrade. Docs: threat-model (Runtime Protections) + REFERENCE §4.13 name the exact number; pinned by `tests/naryad_255_body_limit.rs` (real HTTP stack: N−1 → 200, N+1 → 413). Merged as PR #283 (PR number ≠ naryad number).

### Fixed — io: sandbox violations are loud [SANDBOX_VIOLATION] — read_file/delete_file no longer mask them (Naryad #254)

- io: `read_file("../secrets")` behaved identically to `read_file("typo.txt")` — any `sandbox_path` error became an empty string, silently swallowing programmer defects (external audit 2026-09-11, issue #257). Now the outcomes are split: a missing-or-unreadable path keeps the soft-failure contract (empty string, unchanged); a sandbox violation — absolute path, `..`, symlink escape, broken symlink — is a loud error carrying the stable code `SANDBOX_VIOLATION` (ADR-0131 convention; the code rides in the error text until the mlog-check diagnostic registry lands — loud deviation, same as №253). Classification lives in `sandbox_path_missing` (`src/builtins/io.rs`): textual violations are always loud; `symlink_metadata` failure (no such path / parent not traversable) is the soft case; an existing path (including a broken-symlink link itself) rejected by canonicalize/prefix is a violation — №131 rejections become loud instead of silent. `write_file`/`append_file` were loud since №252 — their errors now carry the code too (`sandbox_path_ex` + the escape/unresolvable arms of `open_sandbox_write`); OS-level write errors stay soft. `delete_file` gets the same split as `read_file`. `file_exists`/`list_dir` behavior unchanged (outside the naryad's named scope — decision recorded in PR). Docs: REFERENCE.md io-builtins table (README has no io-builtin section — loud deviation). Tests: +8 unit (`io::tests_n254`), +8 integration (`tests/naryad_254_sandbox_violation.rs`); io unit suite 27/27 (n131/n252 unbroken); `#[ignore]` delta 0. Merged as PR #273 (PR number ≠ naryad number).

### Fixed — security(io): write-path symlink TOCTOU — canonical return + two-phase O_NOFOLLOW open (Naryad #252)

- security(io): write-path symlink TOCTOU closed (planted-final-component escape, issue #255). Root cause: `sandbox_path_ex` validated the canonical path but returned the ORIGINAL string — the actual write re-traversed any symlink planted between the check and the use. Mechanics: `sandbox_path_ex` now returns a path that is safe to USE, not merely checked — `ForRead` returns the canonicalized file path (in-sandbox symlink reads still resolve — the sandbox is not a symlink ban), `ForWrite` returns `<canonical parent>/<final component>` with the parent prefix-verified; the new `open_sandbox_write` (`src/builtins/io.rs`) closes the final-component race with a two-phase open: `create_new` (fresh creation is atomic — no symlink can sit there) → on `AlreadyExists` canonicalize the full path, re-verify the sandbox prefix, reopen the CANONICAL path with `O_NOFOLLOW` (unix; on non-unix step 2 opens the canonical path without O_NOFOLLOW — documented boundary, symlink creation there requires elevated privileges). Honest boundary recorded in code: intermediate directory components swapped between canonicalize and open remain out of scope (would need per-component O_NOFOLLOW or Linux openat2 RESOLVE_BENEATH — revisit on a real use case). `http_download` (the third ForWrite site, `src/builtins/http.rs`) is closed by the same helper — its boolean soft-failure contract is unchanged (`Ok(false)` on any failure). Tests: 7 new in `src/builtins/io.rs` (`tests_n252`, unix-gated where planted symlinks are the repro'd class; full `write_file`/`append_file` call chains exercised, not just `sandbox_path_ex`): write/append through a planted symlink denied with the outside file untouched, broken-symlink write loud, directory write loud, in-sandbox symlink read still OK, new+overwrite+append regular-file path OK, canonical ForWrite return shape pinned. Merged as PR #261 (PR number ≠ naryad number).

### Changed — security(io): file-write sandbox violations are now loud errors (Naryad #252 — breaking)

- security(io): `write_file`/`append_file` — violating the file sandbox is now a LOUD error (`file I/O sandbox: ...` — escape past the sandbox base, unresolvable target, path with no file name; these errors also carry the stable code `SANDBOX_VIOLATION` since naryad #254, PR #273) instead of the silent soft-failure that returned an empty string; ordinary OS-level write errors KEEP the soft-failure contract (empty string) — sandbox violations are programmer errors, OS failures are environmental. Breaking for code that relied on the silent refusal (allowed pre-1.0 — owner decision on naryad #253 Variant A, issue #256; PR number ≠ naryad number). Migration: check return values / handle the error text instead of treating the empty result as the only failure mode. `http_download` keeps its boolean soft-failure contract (`Ok(false)` on any failure, sandbox violation included — the helper's loud error is consumed internally there; loud deviation from naryad №258's prescribed wording, which grouped http_download into the loud change — the merged PR #261 code documents the unchanged contract in place). Merged as PR #261.

### Changed — security: exec in serve route handlers gated by METALOGOS_SERVE_ALLOW_EXEC (Naryad #253, Variant A — breaking)

- security: `exec()`/`exec_argv()` in serve route bodies now require `METALOGOS_SERVE_ALLOW_EXEC=1` — route handlers no longer inherit the process-level `METALOGOS_ALLOW_EXEC` flag (replacement semantics, not AND; owner decision 2026-09-11, issue #256). Before this change, `mlog serve` with `METALOGOS_ALLOW_EXEC=1` gave every route handler a full `sh -c` — route code (frequently authored or generated by someone else) silently inherited the operator's shell pass. Mechanics: the gate duplicated in `exec`/`exec_argv` since №97 is collapsed into the SSOT `exec_gate(context)` (`src/builtins/io.rs`); a thread-local RAII `ServeRouteExecGuard` is set inside the spawn_blocking closures of BOTH route paths (`execute_route_body`, `execute_route_body_vm` in `src/server.rs`) — TW and VM backends are gated identically, while serve top-level (route registration) and `mlog run`/`check` keep the №97 process-flag behavior unchanged. Denials carry the stable diagnostic code `EXEC_NOT_PERMITTED` (ADR-0131 naming convention; the code rides in the error text until the mlog-check diagnostic registry lands — loud deviation, noted in the PR) and name the exact flag of the current context. The serve banner lists `METALOGOS_SERVE_ALLOW_EXEC=1` among danger flags and prints `route exec: ENABLED / denied` at startup (`src/main.rs`). Every allowed invocation is recorded in the subprocess audit log (`METALOGOS_AUDIT_LOG_PATH`). `exec_restricted` (html_render, pdf) intentionally stays flag-free: fixed binary, arguments built from code, never from request bodies (closure pinned by tests). Tests: 8 new in `tests/naryad_253_exec_gate.rs` (denial without flags TW+VM; the essence-of-A case — process flag set, still denied; serve-flag enables exec + audit record; process context unchanged; gate replacement-semantics unit; registry closure); n88 html_render contract 13/13. Docs: threat-model (Runtime Protections → exec gates), SECURITY.md, README (Runtime exec gates). Migration: set `METALOGOS_SERVE_ALLOW_EXEC=1` where route bodies must call `exec()`. Breaking by design — allowed pre-1.0; merged as PR #271 (PR number ≠ naryad number).

### Fixed — vm: route bodies dispatch user patterns with TW parity (Naryad #250, closes ADR-0122 #208)

- vm: route bodies dispatch user patterns with TW parity (query_param/json_body/respond) — closes ADR-0122 #208; 28 tests un-ignored. Four stacked roots repro'd then fixed: (1) the serve path never registered patterns — `load_program` now pre-registers them by scanning main_code's `RegisterPattern` instructions (index order 1:1, old-.mbc safe) and the handler is index-stable (rposition replace-in-place); (2) templates never reached the VM path — the compiler registers them at compile time via the n115 `GLOBAL_TEMPLATES` channel (overwrite-idempotent; Program schema frozen; .mbc-templates residual loud); (3) route bodies lost their value to the fall-through (a trailing ExprStmt `Pop` made `execute_code` return a leftover local — "hi" instead of the HttpResponse) — final-Pop suppression gives the TW semantics (body value = last statement's value); (4) bool literals compiled to `Float(1.0/0.0)` (VM `sort` diverged from TW on mixed lists) — now `Const(Value::Bool)`, .mbc format unchanged; plus the designed "404 Not Found" body was never wired into the axum router — a backend-agnostic `.fallback` added (TW/VM parity kept). Un-ignore 28 of the planned 31 (loud): vm_golden's 2 stay (their full-corpus blockers — match-in-VM and candle-gated reflex examples — are outside the naryad's §3); 1 kv test returned with the accurate root (KV_STORE is process-global by design). `#[ignore]` 136→108.

### Changed — adapt quality mock 0.95: revisit point formally recorded (Naryad #247, docs-only)

- docs: adapt quality mock 0.95 — revisit point formally recorded (ADR-0112 addendum; external audit 2026-09-10), REFERENCE §5.15 honesty note. The mock value 0.95 stays in the code (rollback tests depend on it; ADR-0112: "The mock value 0.95 stays in the code") — the rollback logic is real and tested, but does not yet respond to actual quality degradation. Revisit only on a real `mutate` use case where the mock value creates a concrete problem. README §5 and REFERENCE §5.15 now mark both the mock value and the revisit point; ADR-0112 gains a "Revisit point (2026-09-10)" addendum with the current code addresses (`src/interpreter/hooks.rs:60-61`, `src/vm.rs:2946-2947`). Docs-only: zero diff in `src/**` and `tests/**`; merged as PR #250 (PR number ≠ naryad number).

### Fixed — llm: full request cancellation by deadline on all paths (Naryad #248)

- llm: full request cancellation by deadline on all paths — legacy backend thread-wrapper replaced with `call_with_deadline` (README/REFERENCE promise closed; external audit 2026-09-10). The `LlmBackend` trait gains `call_with_deadline` (default → `call_with_model`, existing impls compatible): RealLlm builds a one-shot client with timeout = min(deadline, 120s) — a real TCP drop at the deadline (no retry loop, single attempt per deadline); MockLlm sleeps min(delay, deadline) and fails loudly when the deadline is tighter. The legacy `Some(timeout)` path in `learnable.rs` now calls the backend directly — the abandoned-thread wrapper (`thread::spawn` + `recv_timeout`, which left the HTTP request in flight) is removed together with its Disconnected arm. SmartRouter path (№156) unchanged. The README/REFERENCE caveat was rewritten to match the actual behavior; the `AbortHandle` promise is withdrawn as fulfilled (external on-demand abort remains out of scope — revisit on a real use case). Proven by a local accept-and-hang server test that observes the server-side TCP close at the deadline; naryad_126 semantics preserved 1:1 with no test edits; no new `#[ignore]`.

### Fixed — test: serialize session-memory contract tests — global store race (№239 family, 2nd round, Naryad #251)

- test: serialize session-memory contract tests — global store race (№239 family, 2nd round). All 10 `tests/session_memory_contract.rs` tests hold one poison-tolerant static mutex for the whole test body (template: `naryad_244_vision_lora.rs` `env_lock`); evidence: 2 CI failures 2026-09-10, `left: 0, right: 1` at `:204` (`contract_session_no_persistence`); test-only — `src/**`, deps, CI settings untouched; 30/30 consecutive green runs.

### Changed — security SSOT sync (Naryad #246, docs-only)

- docs: security SSOT sync — threat-model (Vision gates + `db_execute` in the SQL row + honest provenance boundary), SECURITY.md (0.19.x supported, generative-pillars paragraph), ADR-0122 truth-up (#213/#214 delivered, owner gate 2026-09-09), go-no-go gate line, README honest line.

### Added — Vision R6.3: LoRA adapters — SQLite BLOB + application to DiT (Naryad #244)

- **`vision_lora_load(name, path) -> String` (Block 2.2)**: reads a
  safetensors adapter ONCE from a file and persists it in the program's
  DB (`db { url: "sqlite:..." }`) — the `vision_lora_adapters` table (name
  TEXT PK / bytes BLOB NOT NULL / meta_json TEXT NOT NULL / saved_at
  RFC 3339). **ADR-0124 section 6: the adapter lives ONLY in an SQLite
  BLOB — not a new file format, not session state** (`VisionRegistry` is
  untouched). The order of loud checks is prescribed: arity/types → no-db
  (a hint pointing at `db { url: ... }`) → `MLOG_VISION_WEIGHTS_DIR` →
  path safety (relative, no `..`, the `.safetensors` extension,
  the file exists — reading is allowed ONLY inside
  weights_dir, no new file-reading surface; the contract function is
  `vision_lora_check_adapter_path`, modeled on `vision_edit_check_dims_r41`)
  → the feature gate (without `vision` = a loud refusal: validation requires
  candle, inserting unverified bytes would be silent garbage — forbidden)
  → reading + parsing + validation → `lora_save`. A name collision = a loud
  Err (no upsert, modeled on #242); returns the persistent key `name`.
- **`vision_lora_generate(decl_name, prompt, lora_name) -> Vision`
  (Block 2.3)**: the full `vision_generate` pipeline with the
  adapter applied. Prescribed order: arity/types → an empty prompt →
  resolving the decl (with a list of the declared ones) → re-checking model ∈
  `KNOWN_VISION_MODELS` → no-db / unknown `lora_name` = Err (with
  `lora_list`) → [gated] reading bytes from the DB plus **integrity**:
  `sha256(bytes) ≠ meta.sha256` = a loud Err BEFORE any compute → the
  Block 1.1 parse → env gates + components → the fixed 1024×1024 → the pipeline.
  `meta_json` is the fixed `LoraMeta` structure (sha256/rank/alpha/
  scale/targets); malformed JSON = a loud Err (modeled on #242).
- **`src/vision/lora.rs` (Block 1.1)**: parsing safetensors bytes
  (`candle_core::safetensors::load_buffer`, a reader modeled on Stage B/C).
  BOTH canonical naming forms are accepted — diffusers-PEFT
  (`<target>.lora_A.weight` / `.lora_B.weight`) and ComfyUI
  (`<target>.lora_down.weight` / `.lora_up.weight` plus an optional
  `<target>.alpha`); mixing forms within one target = a loud Err.
  `rank` is the average dimension; `scale = alpha/rank`; a missing alpha
  gives `scale = 1.0` with a loud eprintln note (in the style of #243's
  quant_conv); a non-F32 input is upcast to F32 LOUDLY. Validation: the
  target, after stripping its suffix, must be an
  attention projection of the base
  (`layers.N`/`noise_refiner.N`/`context_refiner.N` x
  `to_q/to_k/to_v/to_out.0` per `zimage_expected_keys`); a non-attention
  target (norms/FFN/embedders/final), an unknown prefix, a B with no A (and
  vice versa), orphaned keys, mismatched dimensions — ALL of these are loud
  Errs with a FULL list of the problems. Silently dropping keys is forbidden.
- **Merging into the DiT (Block 1.2)**: `ZImageTransformer::from_weights_with_lora`
  (dit.rs, additive) — the base is built by the untouched `from_weights`, then
  `merge_lora_in_place` runs per target: `W' = W + scale·(up@down)` in F32
  with a cast back to the base dtype; a deterministic (sorted) order of
  targets; a missing base or an index beyond n_layers = a loud Err. **A bit-exact
  obligation (Block 1.3)**: `from_weights`/`forward`/`forward_edit`/
  `new_tiny` — zero diff; the merge is called ONLY on the lora path;
  a control invariant: a zero up OR down means the output is byte-for-byte
  equal to the base (verified at both the weight and output level). **The wedge
  goldens of #212 stay green with NO edits** (85ef6a87/860c85b3).
- **Composite provenance (Block 2.4)**: when an adapter is applied,
  `model_sha256 = sha256("{base}\nlora:{name}:{lora_sha256}")`, where
  `base = weights_tree_sha256(weights_dir)` (may be "unpinned" — the
  composite is honest above the marker too), `lora_sha256` is the SHA of the
  adapter's bytes from the DB; `model_id` is the base from the decl; the
  watermark is the base model (the adapter is a delta, not the model). The
  formula is fixed in the code at its computation site
  (`vision_lora_composite_model_sha256`, pub — mechanically pinned by a
  test, precedent `verify_sha_pin`) and is mirrored in REFERENCE section 4.22
  and the doc comment on `VisionManifest::model_sha256` (the provenance
  structure/code is untouched — the 7 fields are not extended).
- **Intercepts x3 plus stubs plus taint (Blocks 2.5/2.6/3)**: `vision_lora_load`
  (db_conn) and `vision_lora_generate` (decls plus registry plus db_conn) in the
  interpreter (eval + invoke) and the VM; last-resort stubs
  `builtin_vision_lora_load_stub`/`builtin_vision_lora_generate_stub` —
  modeled on export_raw_stub (doc reference "R6.3, #244");
  `spec!` lines after `vision_load`, arities
  2 and 3. Taint: the positional check on arg 1 is extended to
  `vision_lora_generate` — the same check-id `VISION_PROMPT_USER_INPUT`
  (Warning; arg 0's decl name and arg 2's lora name are not flagged; no
  new check-ids/categories, and #241's gates are untouched).
- **Registry 389 → 391; THE FAMILY CEILING IS REACHED**: there are
  now 10 vision builtins (generate 2 / edit 2 / export 2 / export_raw 2 /
  fetch_weights 2 / list 0 / save 2 / load 1 / lora_load 2 /
  lora_generate 3) — the top of ADR-0124 section 3's predefined bound
  ("Expected family size: ~8-10 — a hard counter against builtins
  bloat"). **The next vision builtin requires amending ADR-0124** — loudly.
- **Tests (Block 4.1, no network/weights, no `#[ignore]`)**: 21 tiny
  contracts in `tests/naryad_244_vision_lora.rs` (parsing both forms,
  parse negatives with a full list, merging changes the tiny DiT's
  output, a zero adapter's byte-for-byte identity, determinism of two
  full tiny runs (merge → sample → tiny VAE → PNG) bit-for-bit, a store
  round trip/collision/malformed meta (a direct UPDATE), integrity SHA,
  dispatch negatives for lora_load/lora_generate, the signature: 7 fields
  plus the composite structure plus the watermark plus the policy from
  the decl; a minimal safetensors blob is built with candle's own writer —
  the API is available, no deviation). Merge unit contracts (a direct
  matmul on the weights, identity weights, out-of-range/missing base) are
  in `src/vision/dit.rs` (`mod lora_tests`). `tests/naryad_240_vision_dispatch.rs`
  is extended (+3 taint tests: arg 1 is flagged, arg 0/arg 2 are not, a
  literal is not flagged), as is
  `tests/naryad_210_vision_skeleton.rs` (+2 real-path tests: a no-db
  lora_load, an arity-3 lora_generate; 11 → 13 tests). The runbook gets
  section 3.2, a lora e2e (a PARKED run, the owner places the adapter in
  `weights_dir/lora/…`).
- **The wedge goldens of #212, weights.rs, the weights manifest (16 files /
  32,848,304,654 B), `tools/fetch_vision_weights.sh`, grammar.pest,
  `KNOWN_VISION_MODELS`, #242's store contract (`vision_artifacts`),
  `VisionRegistry`, ADR-0124 — untouched.**


### Added — Vision R6.2: `vision_edit` — in-context editing (Naryad #243)

- **`vision_edit(handle, prompt) -> Vision` (Block 2)**: the loud R1 stub
  became a real in-context editing path (the second third of R6
  "Edit + LoRA," plan section 7.1; R6 was loudly split: #242 = save/load —
  closed, #243 = edit, #244 = LoRA). Contract: arity 2, a typed
  handle (`[Vision#N]`, modeled on export/save); **the source MUST be
  signed** — an artifact with `manifest: None` is a loud Err (specifically
  BEFORE the env check: the contract refusal does not depend on the
  environment; editing an unsigned artifact cannot honestly be done —
  there is nothing to inherit, and producing an unsigned one through the
  real compute path is forbidden by #241 Block 1.3). Unsigned artifacts
  remain usable via `vision_export_raw` — this was NOT changed.
- **The in-context edit compute path (Block 1)**: `VaeEncoder` in
  `src/vision/vae.rs` (mirroring `VaeDecoder`) — loads the non-decoder
  prefixes of the SAME pinned `vae/diffusion_pytorch_model.safetensors`
  (the weights manifest is NOT extended, 16 files / 32,848,304,654 B —
  an invariant). The manifest's arithmetic of 244 = 138 decoder + 106 encoder means
  `quant_conv` is optional: its presence or absence is a loud note,
  and the actual non-decoder list is checked against the file's header at
  the PARKED run (runbook section 3.1), a mismatch against the generator
  `vae_expected_encoder_keys` being a loud Err listing what's missing.
  `encode(img F32 [-1..1]) → [1, C, H/f, W/f]` runs in posterior MODE
  (for determinism), the ritual being the algebraic inverse of #232's
  decoder direction (`z_model = (mean − shift) · scaling`); `decode_png` is
  the decoding half of `encode_png`. The cycle: `flow_match_euler_edit`
  (`sampler.rs`) plus `forward_edit` (`dit.rs`, additive — generate is
  untouched): the reference latent is concatenated as tokens with the noisy
  one at EVERY step (the same `x_embedder` plus `noise_refiner`; the
  reference's RoPE t-slot is cap_len+2 while the noise's is cap_len+1 —
  a loud check against `axes_lens`), `euler_step` applies only to the noise
  branch, the reference stays clean; **`EDIT_STEPS = 8`** — the distilled
  turbo NFE (a loud constant with a pinning test); there is no CFG (turbo).
  **The wedge goldens of #212 stay bit-exact with NO edits** (85ef6a87/860c85b3) —
  adding the edit path does not change generate by a single bit (Block 1.3).
- **Provenance inheritance (Block 2.3, always sign)**: an edited
  artifact is always signed — an LSB watermark (the source's model_id) plus 7 fields:
  `model_id`/`policy`/`seed` are inherited from the source's manifest
  (determinism: the same source plus prompt plus weights gives the same seed), `model_sha256` is
  the current run's `weights_tree_sha256`, `prompt_sha256` is the hash of the
  EDIT prompt, `timestamp`/`png_sha256` are fresh (the SHA is computed after the
  watermark — it describes exactly the bytes that are shipped).
- **The dims contract (Block 1.4)**: the R4.1 bounds on the source (256..=4096, x16)
  and being a multiple of the VAE factor are loud Errs; silent resizing is forbidden (it would
  corrupt the provenance chain): the output keeps the source's resolution.
- **Taint (Block 3)**: the positional check on arg 1 is extended to `vision_edit` —
  the same check-id `VISION_PROMPT_USER_INPUT` (Warning, arg 0's handle is not
  flagged); there are no new check-ids/categories, and #241's gates are untouched.
- **Intercepts plus truth-up (Block 2.5/2.6)**: the interpreter (eval + invoke) and
  the VM — the state-carrying pattern from #240-#242; the last-resort stub is
  modeled on #242's save/load stubs; the doc reference "214/215" is replaced by
  "R6.2, #243" — the last "214/215" left the repo.
  `tests/naryad_210_vision_skeleton.rs`: the stub test is replaced by
  real-path refusals (a typed handle plus arity) — 10 → 11 tests.
- **Tests (Block 4, no network/weights, no `#[ignore]`)**: 14 tiny contracts
  in `tests/naryad_243_vision_edit.rs` (modeled on the #212 wedge: the output's
  dependence on the source and on the edit prompt, the watermark plus the
  inheritance of 7 fields, an unsigned refusal, the R4.1/factor dims contract,
  a no-env refusal, the EDIT_STEPS pin, forward_edit's shape contract,
  determinism of the cycle) plus an env-gated
  `mlog_vision_edit_export_e2e` (a loud SKIP; generate → edit → export:
  preserving dims, the sidecar's inheritance, the watermark) — the edit e2e
  target lives in runbook section 3.1. The registry stays at 389; `KNOWN_VISION_MODELS`/the
  ADR-0124 enum/weights.rs/grammar.pest/the golden constants/LoRA/#241's gates are
  untouched; a dedicated env-gated CI step was NOT added (owner debt, a
  4th round of the reminder).


### Added — Vision R6.1: SQLite persistence of artifacts (Naryad #242)

- **`vision_save(handle, name) -> String` / `vision_load(name) -> Vision`
  (Block 2)**: the loud R1 stubs (`src/builtins/vision.rs:723/733`) became
  real persistence paths. Intercepted in the interpreter (eval + invoke) and
  the VM — the state-carrying pattern from #240/#241 plus a connection to the
  program's DB (`db_conn`: on the VM, a field from `program.db_url`; on the
  interpreter, an `Arc<Mutex<Option>>` from the `db { url: "sqlite:..." }`
  declaration). The registry id is a session-scoped handle (monotonic from
  zero, NOT persisted); the persistent key is `name`. No-db gives a loud Err
  with a hint at the declaration; an unknown handle gives a loud Err with
  `[Vision#N]`; an unknown name gives a loud Err with a list of what's saved
  (loud diagnostics).
- **A new module, `src/vision/store.rs` (Block 1)**: the
  `vision_artifacts` table (name TEXT PRIMARY KEY, png_bytes BLOB NOT NULL,
  manifest_json TEXT, saved_at TEXT NOT NULL RFC 3339 UTC; creation is
  modeled on `init_kv_persist`, WAL is left untouched — it's managed by the
  db layer). The `save`/`load`/`list` API is for tests and loud diagnostics;
  there is no built-in listing layer on top of the DB. **A verbatim
  manifest round trip**: `Some(m)` → sidecar JSON → `Some(m')`, `m' == m`
  field by field (including `timestamp` — provenance persistence does not
  regenerate it); `None` → `NULL` → `None`; **malformed manifest JSON is a
  loud Err** (silent degradation to unsigned would be a forbidden loss of
  provenance). **A name collision is a loud Err** (a plain INSERT — upsert/delete
  semantics are out of scope for #242, since a silent overwrite would break
  the provenance chain); an empty name is a loud Err; the PNG bytes only ever
  travel as a BLOB in the program's DB (no on-disk writes outside the export
  path).
- **The round-trip contract (Block 3, R6's planned "round-trip test"
  acceptance)**: registry A with a signed artifact → save → a new, empty
  registry B → load → a signed `vision_export` — the PNG and the sidecar
  `<path>.manifest.json` are byte-for-byte equal to the originals. **The #241
  backstop stays alive after persistence**: a loaded artifact with
  `manifest: None` is still refused by a signed `vision_export`
  (`VISION_UNSIGNED_EXPORT`) and works with `vision_export_raw` without a
  sidecar. Tests: 8 unit tests (store) plus 8 integration tests
  (naryad_242_vision_save_load); no network, no weights, no
  `#[serial]` (the env is untouched), `sqlite::memory:` per test.
- **The registry stays at 389**: the `vision_save`/`vision_load` stubs
  have existed in the registry since #210 — the naryad replaced their
  bodies/intercepts, without adding builtins. The last-resort stubs were
  updated loudly (modeled on `vision_export_raw_stub`, using the #242
  numbering instead of the outdated "214/215" R0-era one). `vision_edit`
  remains a loud R6 stub (its turn is #243).

### Added — Vision R5: Security — Category A gates + a Provenance MVP (Naryad #241, ADR-0125)

- **A Provenance MVP (Block 1)**: `vision_generate` ALWAYS signs — an
  LSB watermark is embedded in the PNG (the payload is the `"MLGV"` magic
  bytes plus a model-hash32 = the first 4 bytes of SHA-256(model_id), 64
  bits in the RGB channels' LSBs; force-set is idempotent; detection reads the
  decoded pixels; a signing failure is a loud `Err` BEFORE the artifact
  reaches the registry), and the artifact carries a `VisionManifest`
  (the model id plus the model's SHA-256 — a fingerprint of the weights
  tree from a pinned `manifest.json`, with an honest `"unpinned"` marker
  when it's absent; the seed; the prompt hash; the policy/`"unspecified"`;
  an RFC 3339 timestamp; the final PNG's SHA-256 — computed after the
  watermark). `vision_export` is now a signed export: PNG plus a sidecar
  `<path>.manifest.json`; #240's unsigned WARN is lifted (export is signed
  by construction). A new module, `src/vision/provenance.rs` — the
  manifest+hash layer is not feature-gated, the watermark is gated behind
  `vision` (it needs a PNG codec). An honest boundary (ADR-0125): the MVP
  watermark is detectable by us, but is NOT adversarially robust
  (robust watermarking/C2PA are a research backlog, not a promise).
- **`vision_export_raw` (Block 2.1, registry 387→388)**: an explicit
  opt-out per ADR-0125 — raw bytes with no watermark/manifest, no sidecar
  written. Intercepted in the interpreter (eval + invoke) and the VM — the
  same state-carrying pattern as `vision_export`.
- **The `VISION_UNSIGNED_EXPORT` gate (Block 2.2 — Category A, an audit
  Error)**: calling `vision_export` in a file with not a single `vision { }`
  declaration — a manifest source is impossible, the artifact cannot be
  signed by construction (modeled on SECRET_LEAK; runs via
  `audit_category_a` → a compile error). A runtime backstop: exporting an
  artifact with no manifest (a hand-built registry) is a loud `Err` with the
  same check-id. **`VISION_UNSIGNED_EXPORT_RAW` (Block 2.3 — an audit
  Warning, advisory)**: fires on every `vision_export_raw` call; the check-id's
  name is fixed by this release (the ADR does not set it). The warning is
  deliberately NOT in the compile path: semantic naryad #98 promotes every
  Warning from `audit_category_a` to an error — that would contradict
  ADR-0125's advisory semantics.
- **The `VISION_POLICY_MISSING` gate (Block 3.1 — an audit Warning) plus
  a parser relaxation ONLY for policy (LOUDLY: the R4.1 contract changes
  per ADR-0125's SSOT, adopted BEFORE R4.1)**: `policy:` is no longer
  required — a missing one parses as `None` (the "field required" error for
  policy is gone), audit warns with `VISION_POLICY_MISSING`, and the
  manifest records `"policy": "unspecified"`; a present value is still
  loudly enum-checked (only `safe`). The other 6 fields remain required;
  the other R4.1 negatives (duplicates, unknowns, the enum, the remaining
  required fields) are untouched. Note: the negative test "missing policy
  → parse error" from R4.1 did not exist in the codebase (#238's 6 parser
  tests did not include it) — the new contract is closed by new tests
  (`test_parse_vision_missing_policy_parses_as_none`,
  `test_parse_vision_unknown_policy_value_still_loud`).
- **The `MODEL_WEIGHTS_UNSAFE` gate (Block 3.2 — Category A, an audit
  Error) plus `vision_fetch_weights(manifest_url, dest_dir)` (registry
  388→389, a real handler)**: an SSRF guard via `check_url_ssrf` (modeled
  on #130, pinning resolutions against DNS rebinding, the kill switch is not
  weakened); an allowlist via the env var `MLOG_VISION_WEIGHTS_ALLOWLIST` —
  **default-deny**: empty/unset means a loud refusal, downloading is
  forbidden; only `manifest.json`-class URLs (a bare `.safetensors` has
  "no pin" — refused; the pickle-RCE class is refused by extension); SHA-256
  pinning of every manifest entry (reusing `WeightsManifest`, `src/vision/weights.rs`
  is not rewritten) — a mismatch is a loud refusal, the file is NOT written;
  entry names are bare `.safetensors` only (no paths/traversal). The static
  gate catches statically visible violations: a literal URL of the
  SSRF-blocked class, a literal `.safetensors`/pickle-class, a literal
  manifest-class URL is statically valid (the actual allowlist is a runtime
  env, unreadable statically — both layers are kept, and the exact names
  are documented in PR #239). The gate is written reusably in `audit.rs` —
  a shared SSOT for the future Voice gates (ADR-0125).
- **4 Category A contract tests closed (Block 4)**: 3 new tests in
  `tests/naryad_241_vision_gates.rs` (exact check-ids plus severities;
  negatives — allowlist default-deny, a host outside the allowlist, an
  empty allowlist, a SHA mismatch on synthetic bytes, the raw warning being
  advisory, positive controls) plus #240's taint test
  `user_input_prompt_emits_audit_warning`. Watermark round trip plus
  manifest presence are unit tests in `provenance.rs` (the vision-tests
  job). No network in the tests, no weights needed.
- **Test infrastructure**: `tests/registry_arity_check.rs` gains a full
  vision section (generate/edit/export/export_raw/fetch_weights/save/load —
  the vision lines were previously absent from the exhaustive list); test
  #210's `vision_export_wrong_handle_type_loud_error` now has a
  `vision { }` declaration in its source (the Category A gate would
  otherwise reject the program at compile time; the test's actual subject —
  a runtime refusal — remains reachable; the adaptation is loud).
- README numbers synced with the artifacts: builtins 387→389; the new
  gates are listed under Category A.

### Added — Vision R4.2: dispatch — `vision { }` -> VM -> builtins + taint (Naryad #240)

- **dispatch pipeline (modeled on reflex_decls)**: `Program::vision_decls: Vec<CompiledVisionDecl>`
  (`#[serde(default)]`, fields 1:1 with AST R4.1: name/model/steps/width/height/seed/policy/profile;
  serde-serializable `CompiledVisionPolicy`/`CompiledVisionProfile` enums; single conversion point
  `CompiledVisionDecl::from_ast`). Compiler pass1 populates the vec; pass2 emits no bytecode
  (reflex precedent). `Vm::load_program` registers name → parameters; the interpreter's
  declaration pass does the same from AST. `vision_registry: SharedVisionRegistry` (Mutex) on the
  interpreter, plain `VisionRegistry` on the single-threaded VM.
- **VisionRegistry real artifact type (№240)**: R1's `()` placeholder → `VisionArtifact`
  (encoded PNG bytes, produced by the real pipeline). `insert/get/remove/list_ids` API preserved
  in spirit; IDs remain monotonically increasing.
- **`vision_generate("decl_name", "prompt")` — REAL path (§3.5: zero silent stubs)**:
  declaration resolution (unknown name → loud `Err` with the declared-names list), runtime
  re-check `model ∈ KNOWN_VISION_MODELS` (defense-in-depth for hand-built/deserialized
  `Program`s), weights from `MLOG_VISION_WEIGHTS_DIR` (missing env/component → loud `Err`
  naming the env var and the missing component — honest environment refusal, NOT a stub),
  full clip tokenizer → Qwen3-4B text encoder → Z-Image DiT + `flow_match_euler_sample`
  (steps and seed from the declaration; sampler sigmas = steps + 1) → VAE decode → PNG encode
  → artifact in the registry → `Value::Vision(id)`. The R4.2 z-image-turbo pipeline generates
  a fixed 1024×1024 (sampler derives the latent from the DiT config); other sizes = loud `Err`
  (size parameterization is R5 manifest territory).
- **`vision_list()`** — real registry handles, sorted by id (determinism), `[Vision#N]` display
  form. **`vision_export(handle, path)`** — writes the artifact's real PNG bytes; every export
  is unsigned → loud stderr WARN + static audit-warning (watermark/manifest/Category-A gate = R5).
  **`vision_edit`/`vision_save`/`vision_load` remain loud stubs** (R6: edit + LoRA/SQLite).
- **Arity truth-up (№240, lesson from #234 — the conflict was resolved before writing this up)**: `BUILTIN_REGISTRY`
  `vision_generate` arity **3→2** per the R4 contract (plan §3: `vision_generate("poster", "…")`;
  the R1 stub doc "(model_name, prompt, seed)" predates the declaration language and was never
  the contract). Total builtin count unchanged (387 — no new builtins).
- **Taint integration (plan §4, modeled on n201)**: `UserInput`-tainted expression in position 2 of
  `vision_generate` → audit-**warning** `VISION_PROMPT_USER_INPUT` (NOT Category A — a
  user-typed prompt is a legitimate use case; the prompt will be recorded in the generation
  manifest, R5). Arg 0 (declaration name) is not data — not flagged. No taint on
  `Value::Vision` (opaque handle; print-guard already stands).
- **Dispatch intercepts (modeled on reflex)**: interpreter (expression evaluation + flow-step
  `invoke`) and VM (`call_vision_builtin` before the generic fallback) route to the shared
  dispatch functions in `src/builtins/vision.rs` — inference logic is NOT reimplemented per
  backend. Registry stubs remain the last resort for direct registry calls (loud refusal).
- **Tests**: `tests/naryad_240_vision_dispatch.rs` (13, non-gated): plan-§3 example parse+compile
  with 1:1 field check; declaration emits no bytecode; dispatch negatives with exact loud
  messages (unknown declaration, wrong arity, missing `MLOG_VISION_WEIGHTS_DIR`, runtime model
  re-check); `vision_list` empty/after-insert sorted; taint warning + three negatives (literal
  prompt, arg-0 taint, sanitized prompt). `tests/naryad_240_vision_mlog_e2e.rs` — env-gated
  `.mlog` e2e (declaration → generate → export → PNG on disk, SHA-256 in output) WITHOUT
  `#[ignore]` — loud-SKIP pattern; **closes the №237 Block 3.1 promise "+ one generation from .mlog"** (loud-gap note in the runbook §3). CI: new `vision-tests` step for the e2e.

### Added — Vision R4.1: `vision { }` declarations — grammar, AST, parser, semantic (Naryad #238)

- **grammar.pest**: `vision_decl` registered in the top-level `declaration`
  rule. `vision "name" { … }` — the name is a STRING (plan-pillar §3 example).
  Seven named fields: `model` (STRING), `steps`/`width`/`height`/`seed` (INT),
  `policy`/`profile` (enum-valued). Unknown field shapes are captured loudly
  by `vision_unknown_field` — named errors with position, no silent skipping.
  `vision_ident_val` (IDENT extended with '-') exists so ADR-0124's `gguf-q4`
  parses — the ADR enum is the SSOT, the token rule bends to fit it.
- **ast.rs**: `Declaration::Vision(VisionDecl)` + `VisionPolicy { Safe }` +
  `VisionProfile { Fp16, Fp8, GgufQ4 }` (ADR-0124 SSOT). `kind_str() => "vision"`,
  name accessor, type_info, span — mirroring neighboring declarations.
- **parser**: duplicate field inside the block = loud parse error pointing at
  the second occurrence; unknown field = loud error naming the field; a known
  field with a wrong value shape gets its own message; policy/profile values
  outside the enums = parse-stage errors (ADR-0124: fp16 | fp8 | gguf-q4;
  R4.1 policy = safe). All seven fields required — no silent defaults.
- **semantic**: `model` must be in the SSOT list `KNOWN_VISION_MODELS`
  (`src/vision/mod.rs`, NOT feature-gated, next to `VisionRegistry`;
  R4.1 = exactly `["z-image-turbo"]`); `steps >= 1` (`steps != 8` →
  audit-warning, NOT error — 8 is the recommended distilled-NFE);
  width/height multiples of 16 in 256..=4096 (VAE latent constraint);
  duplicate vision declaration name in a module = error. All errors carry
  the declaration span and name the field.
- **Tests** (parser + semantic): plan-§3 example parses field-by-field;
  vision + `flow main` parse together; all three ADR-0124 profile values
  parse; 7 negatives (unknown model / unknown profile / unknown field /
  duplicate name / width %16 / steps 0 / duplicate field) — each loud with
  position; valid program = no errors and no warnings; steps != 8 = warning
  (not error).
- **Dispatch NOT touched (R4.2)**: builtins/registry arity and VM are
  zero-diff; vision declarations carry no bytecode and no runtime semantics
  yet. Minimal no-op match arms were added in compiler.rs / execution.rs /
  modules.rs — forced by exhaustive matches (compile requirement),
  documented in naryad #238's PR description.

### Added — Vision R3.7: real-weights run preparation (Naryad #237)

- **fetch tool**: `tools/fetch_vision_weights.sh` — manifest-driven weight
  fetcher (SSOT = №212 manifest tables): `curl -L -C -` per-file resume,
  reference sha = manifest value → else HF LFS oid, sha-verified loud SKIP on
  re-run, POST-DOWNLOAD REFUSAL on mismatch (file not consumed), loud
  non-zero exit on network failure, `--dry-run` offline plan, `--only
  <subdir>` component-scoped fetch. Verified without heavy weights:
  `bash -n`; dry-run plan (16 files); `--only tokenizer` real fetch (4 files,
  15881072 B) + SKIP re-run + truncated-file resume-repair +
  same-size-corruption loud refusal.
- **manifest №212**: section "How to verify against the source" — HF LFS oid = SHA-256 of the file, mismatch = loud refusal;
  layout sizes truth-up from HF models API (real total 32 848 304 654 B ≈
  32.85 GB — the "~24.6 GB" go-no-go estimate was an underestimate);
  tokenizer table filled with real sha256/bytes (download run + SKIP re-run,
  identical values). Heavy weights remain _TODO_ — Branch (b) of Block 2.2
  (no ≥40 GB machine in the delivery environment, 9.2 GB free); loud gap in
  the PR description.
- **runbook**: `docs/research/naryad-237-real-weights-runbook.md` — pre-run
  checklist (≥40 GB disk; ≥64 GB RAM per F32 dtype policy ~62 GB peak; BF16
  honestly flagged R4+ territory), exact env-gated commands (the three №212
  tests), result-fixation table (PNG path/SHA/size, stage timings, 2-run
  bit-exact determinism), Go/No-Go criteria verbatim from go-no-go,
  REAL-RUN-only rule for "REQUIRES REAL RUN" slots.
- **Ignore-count invariant truth-up (Block 2.3)**: the "96/0" figure in older
  naryad templates is not reproducible. Formula fixed from №237 on:
  `git grep -c '#\[ignore' HEAD -- src tests` = N (write the actual N; = 129
  at base 1f26f41/396b1df — 125 tests + 4 src), delta to base = 0.
- **No src changes by design**: scope-freeze — code is GO-ready after №236;
  `git diff --stat <base>..HEAD -- src/` is empty for this naryad. The
  real-weights run is PARKED (owner decision 2026-09-09) until hardware
  appears; the run itself = one session per the runbook, reported separately
  (§3.5 of the naryad spec).

### Added — Vision R3: end-to-end Z-Image-Turbo wedge (Naryad #212, completed #231, rebuilt to reference #232, fix-forward #233, micro-fix #234, VAE structure truth-up #235, expected-key generators extracted #236)

- **VAE structure truth-up (№235)**: decoder structure per real safetensors header —
  layers_per_block+1 resnets per ALL blocks (was: only last), conv_norm_out (GroupNorm→SiLU→conv_out)
  added to decode path, shortcuts on channel changes. VAE tiny golden re-pinned.
- **Expected-key generators extracted (№236)**: VAE/DiT/TE expected-key generators
  extracted to standalone functions (vae_expected_decoder_keys, zimage_expected_keys,
  te_expected_keys). from_weights calls these functions; unit tests call the SAME
  functions (not algorithm copies). Fixed VAE generator bug: in_ch updated inside
  resnet loop (was outside → 146 instead of 138). VAE/DiT/TE goldens unchanged.
- **VAE mid-attn placement fixed (№234)**: attention now applied between
  resnets[0] and resnets[1] per `UNetMidBlock2D.forward` (diffusers
  unet_2d_blocks.py L737-748). Was after both resnets — mathematically wrong.
- **Guard truth-up (№234)**: `check_tensor_coverage` upgraded from count-based
  to key-level (Vec<String>) in all three `from_weights` (DiT, VAE, TE).
  Errors now name the specific missing/extra tensor keys. New unit test
  `loader_guard_tiny_map_coverage` verifies tiny DiT key set.
- **Doc-sync (№234)**: README badge 12→15 blocking jobs; version v0.17→v0.19;
  REFERENCE size ~86→~88 KB; stale v0.18→v0.19 reference.
- **RoPE wire-in (№233)**: `AxialRoPE::apply(q, k)` now called in all attention
  paths (noise_refiner, context_refiner, layers) after qk-norm — was a TODO
  stub. Clamp on out-of-range pos_ids replaced with loud `bail!`. Cap pos_ids
  corrected to per-token `(i+1, 0, 0)` per `create_coordinate_grid` source.
- **Loader tensor-coverage guard wired (№233)**: `check_tensor_coverage` called
  in `ZImageTransformer::from_weights` (expected 521), `VaeDecoder::from_weights`
  (expected 138 decoder), `TextEncoder::from_weights` (expected 398). Detects
  missing/extra tensors at load time.
- **VAE mid-block attention (№233)**: `VaeAttention` struct with GroupNorm →
  spatial self-attention (q/k/v/out_proj) → residual. Loaded from
  `decoder.mid_block.attentions.0.*` in `from_weights`; computed in `decode`
  when `mid_block_add_attention=true`. Tiny config (false) unaffected.
- **DiT rebuilt to diffusers reference (№232)**: 12 discrepancies fixed against
  `transformer_z_image.py` (fetched 2026-09-08):
  - t-embedder: sinusoidal(256) → Linear(256→1024) → SiLU → Linear(1024→min(dim,256))
  - Block adaLN: Linear(min(dim,256)→4*dim), 4 chunks (scale_msa, gate_msa, scale_mlp,
    gate_mlp), gate=tanh(gate), scale=1+scale, NO shift
  - Block norms: 4 RMSNorms (attention_norm1 on input*scale_msa, attention_norm2 on
    attn output before residual, ffn_norm1 on input*scale_mlp, ffn_norm2 on FFN output
    before residual)
  - Final layer: LayerNorm(dim, affine=False, eps=1e-6) → ×(1+scale) → Linear. No gate,
    no shift, no residual. SiLU applied to adaln_input BEFORE Linear (Sequential(SiLU, Linear))
  - cap_embedder: RMSNorm(cap_feat_dim) → Linear(cap_feat_dim→dim). No SiLU.
  - Refiners BEFORE main: noise_refiner (modulation=True) on x-tokens, context_refiner
    (modulation=False) on cap-tokens, THEN main layers
  - Unified sequence: [x, cap] (x first, basic mode)
  - FeedForward: hidden_dim = int(dim/3*8) = 10240 (real) / 170 (tiny)
  - VAE decode ritual: latent / scaling + shift (NOT (latent-shift)/scaling) — pipeline_z_image.py L589
  - Loader guard: `check_tensor_coverage(expected, loaded)` — detects missing/extra tensors
- **DiT tiny golden pinned (№231)**: SHA-256 + anchor bits, pinning ×3, seed
  determinism. Hash `e686167b2e82ee7be9fe3408ed9e619953e774d49e224310f2d0541af3c10257`.
  Fixed n212 forward-path bugs (linear_seeded arg order, broadcasting, refiner
  adaLN, final layer gate/residual) discovered when replacing the
  `assert!(true)` placeholder.

- **Weights infrastructure** (`src/vision/weights.rs`):
  - `WeightsManifest` — record of expected files + SHA-256 (loaded from
    `{weights_dir}/manifest.json` if present; template at
    `docs/research/naryad-212-weights-manifest.md`).
  - `load_safetensors_sharded(dir, stem, device)` — reads `{stem}.safetensors.index.json`,
    loads shards via `candle_core::safetensors`. SHA-256 verification of each
    shard against manifest (loud error on mismatch — silent fallback forbidden).
  - `load_safetensors_single(dir, stem, device)` — for unsharded checkpoints (VAE).
  - ZERO network access (auto-download is R5/ADR-0125).
- **Tokenizer** (`src/vision/tokenizer.rs`):
  - `Tokenizer::from_dir(tokenizer_dir)` — loads HF `tokenizer.json` via the
    canonical `tokenizers` crate. Hand-rolling Qwen2 byte-level BPE with GPT-2
    pre-tokenizer + 119 special tokens is high-risk for silent mis-tokenization;
    `tokenizers` is HF's verified reference (see ADR-0124 update).
  - `encode(text)` — no chat template applied (per diffusers ZImagePipeline).
- **TextEncoder::from_weights** (`src/vision/text_encoder.rs`):
  - New constructor parallel to existing `new(config, seed)`. Loads from
    `HashMap<String, Tensor>` with HF Qwen3 naming. All tensors cast to F32;
    shape-checked against `QWEN3_4B_CONFIG`. R2 contract UNTOUCHED.
- **VAE decoder** (`src/vision/vae.rs`):
  - `VaeDecoder::new_tiny` — seeded tiny-init via SSOT PRNG. Pinned golden
    SHA-256 + 4 anchor bits (3 bit-identical runs).
  - `VaeDecoder::from_weights` — real flux-dev-style AutoencoderKL weights loader.
  - `decode(latent)` — flux-dev ritual `z = (latent - shift) / scaling` then
    decoder then `(sample/2 + 0.5).clamp(0, 1)`. Returns `[3, H, W]` in [0,1].
  - `save_png(img, path)` — PNG encode via `image` crate.
  - `fixed_latent(seed, c, h, w)` — seeded randn via Box-Muller over SSOT-PRNG.
- **ZImageTransformer** (`src/vision/dit.rs`):
  - `ZImageTransformer::new_tiny` — seeded tiny-init via SSOT PRNG.
  - `ZImageTransformer::from_weights` — real DiT loader (cap_embedder, t_embedder,
    30 layers, 2 refiner blocks, final layer with adaLN).
  - `forward(latent, cap, t)` — patchify 2×2 → cap_embed → concat → 30 layers
    (MHA + qk_norm + SwiGLU + adaLN) → split → refiner → final adaLN + unpatchify.
  - Architecture follows diffusers `ZImageTransformer2DModel` (verified by direct
    HF config fetch in Block 0).
- **FlowMatchEuler sampler** (`src/vision/sampler.rs`):
  - `flow_match_euler_sigmas(N, shift, num_train)` — sigma schedule per
    diffusers `FlowMatchEulerDiscreteScheduler`.
  - `euler_step(x, velocity, sigma, sigma_next)` — `x += (sigma_next - sigma) * v`.
  - `flow_match_euler_sample(dit, cap, seed, 9, 0.0)` — full sampling loop.
- **Two-tier test architecture** (`tests/naryad_212_wedge_e2e.rs`):
  - CI-visible (5 tests, no env-gate): VAE tiny golden (pinned), VAE determinism,
    DiT placeholder, sampler sigmas pinned + 4 scheduler unit tests in `sampler.rs`.
  - env-gated (3 tests, loud SKIP when `MLOG_VISION_WEIGHTS_DIR` unset — NOT
    `#[ignore]`): TextEncoder real-weights forward, VAE real-weights decode,
    clinical e2e first image.
- **Dependencies** (gated under `vision`, NOT in default/full):
  - `tokenizers` = 0.22 (HF canonical BPE)
  - `image` = 0.25 with `png` feature only
  - Both added to `vision = ["candle", "dep:tokenizers", "dep:image"]`.
- **Research docs**:
  - `docs/research/naryad-212-wedge-e2e-facts.md` — 13-section fact sheet:
    configs, tensor map (521 transformer tensors + 398 text encoder tensors),
    dtype policy, mechanics (axial RoPE, t-embed, cap-embed, adaLN, refiner),
    R2 contract invariant.
  - `docs/research/naryad-212-weights-manifest.md` — template manifest for
    SHA-256 verification (executor fills at download time).
  - `docs/research/naryad-212-go-no-go.md` — Go/No-Go report (code-complete,
    env-gated run pending real-weights execution on appropriate hardware).

### Fixed — fix-forward #237: runbook doc figures not reconciled with test constants (#238 Block 0)

- `docs/research/naryad-237-real-weights-runbook.md` sections 3 and 6: the DiT tiny golden
  `e686167b…` (a stale n231 hash) → the current `860c85b311905f6c23b90a4e9e3192928027a24bf3e4a00a08096336abad4b3c`
  (SSOT = the `GOLDEN_DIT_TINY_HASH` constant in `tests/naryad_212_wedge_e2e.rs`);
  `e686167b` is kept alongside it as n231's historical hash (the pre-rebuild architecture).
- Also in section 3: the TE size "3 shards, ~7.5 GB" → "3 shards, 8,044,982,000 B ≈ 8.05 GB"
  (3,957,900,840 + 3,987,450,520 + 99,630,640; reconciled by the verifier against the HF API on 2026-09-09).

### Fixed — Vision R2 hotfix (Naryad #230): PRNG SSOT + stream hygiene + golden pinning

- **PRNG SSOT**: the divergent local `generate_uniform_f32` copy in
  `src/vision/text_encoder.rs` (an xorshift64 core without the `seed_to_state`
  XOR ritual, an f32-vs-f64 mapping path that produced divergent value streams from
  the `src/nn` SSOT) is removed. The text encoder now imports
  `crate::nn::attention::generate_uniform_f32` — the project's SSOT for
  weight-init PRNG (documented in `src/nn/attention.rs`).
- **Stream hygiene**: `param_seed(master, layer, param)` — a splitmix64
  finalizer over `(master_seed, layer, param)` — derives per-parameter
  seeds, eliminating naryad #211's stream-overlap bug (layer i's k was
  identical to layer i+1's q; the embedding was identical to layer 0's q).
  The `PARAM_*` constants are fixed (`PARAM_EMBEDDING=0` through `PARAM_DOWN=7`) —
  do NOT renumber them: the derivation is part of the golden contract.
- **Feature implication corrected**: the `vision` feature in `Cargo.toml`
  changed from `["dep:candle-core", "dep:candle-nn"]` (parallel —
  it enabled the dependencies but NOT the `candle` feature flag, so
  `#[cfg(feature = "candle")]` modules in `src/nn/` were not compiled
  under `--features vision`) to `vision = ["candle"]`. This makes
  vision actually imply candle (as the comments throughout the codebase
  already claimed), so `crate::nn::attention` is now accessible from
  vision-only builds. The local copy was the workaround; the implication
  is the fix.
- **The golden contract is pinned**: `tests/naryad_211_text_encoder_golden.rs`
  gains `GOLDEN_HASH_P1/P2/P3` (SHA-256 of the F32 bytes) and
  `GOLDEN_ANCHOR_BITS_P1/P2/P3` (4 corner `f32::to_bits()` values per prompt,
  integer-exact — immune to float-printing drift). Test 1 asserts
  against these. Pinned after 3 bit-identical local runs (2026-09-08).
  The known-debt comments were removed — replaced by "PRNG: SSOT via crate::nn;
  golden records pinned".
- **A derivation test**: a new test, `param_seed_derivation_is_pairwise_distinct`
  — verifies that 32 seeds (4 layers x 8 params) are pairwise distinct plus
  `param_seed(20711, 0, 1) != 20711` (non-identity).
- **CI**: the vision-tests job's "Vision R2 text encoder golden contract"
  step gains `--nocapture` so the eprintln hash/anchor output is visible
  in CI logs — mandatory infrastructure for the re-pinning procedure
  that will recur in R3 (dtype/init changes).


### Added — Vision R2: text encoder (Naryad #211)

- **`vision` feature now implies `candle`** — see hotfix (naryad №230)
  above; the original №211 delivery documented the implication but
  implemented it as parallel `dep:candle-*` enablement without the
  `candle` feature flag.
- **Qwen3-architecture text encoder** (`src/vision/text_encoder.rs`):
  - `TextEncoderConfig` + `QWEN3_4B_CONFIG` (pinned from config.json: 36
    layers, 2560 hidden, 32/8 GQA, head_dim=128, intermediate 9728, SwiGLU,
    RmsNorm eps=1e-6, RoPE theta=1e6, max_position 40960 — corrected
    fix-forward after the initial delivery pinned fabricated dims).
  - `TextEncoder::new(config, seed)` — deterministic seeded init via
    `crate::nn::attention::generate_uniform_f32` (SSOT, naryad №230).
  - `forward(token_ids) -> [seq_len, hidden]` — final-layer hidden states,
    with causal mask, RoPE, QK-norm, GQA.
  - RoPE + QK-norm + causal mask implemented in `src/vision/` — `src/nn/*`
    NOT modified.
- **Golden embedding contract** (`tests/naryad_211_text_encoder_golden.rs`):
  6 tests, all `#![cfg(feature = "vision")]`. SHA-256 records + anchor
  bits are pinned as consts (naryad №230):
  - `golden_embeddings_shape_and_hash` — 3 prompts, shape + hash + anchor
    bits asserted bit-exact.
  - `determinism_same_seed_same_output` — same seed = identical hash.
  - `determinism_different_seed_different_output` — different seed =
    different hash.
  - `causal_property_prefix_match` — first N positions of long prompt
    match short prompt (1e-6 tolerance).
  - `qwen3_4b_config_matches_pinned_values` — constants-assert.
  - `param_seed_derivation_is_pairwise_distinct` — naryad №230 Block 1.6.
- **CI**: `vision-tests` job gains golden-contract step (with
  `--nocapture` since naryad №230).
- **Research**: `docs/research/naryad-211-text-encoder-facts.md` — 3
  independent sources confirming Qwen3-4B as Z-Image encoder, pinned
  config.json dimensions, dtype policy (F32 for R2), hidden-states
  question documented.
- **ADR-0123 item 1 resolved** — text-encoder identity confirmed.

### Added — Server test infrastructure (Naryad #207)

- **`run_test_server_with_backend_in_dir(source, backend, base_dir)`** — new
  test server function that accepts an explicit `base_dir` parameter. This
  controls BOTH import resolution paths:
  - TW: `Interpreter::set_base_dir(base_dir)` (module loading)
  - VM: `Compiler::with_std_root(base_dir)` (import resolution)
- Backward-compatible wrapper `run_test_server_with_backend(source, backend)`
  preserved — delegates via `current_dir()`, matching `Compiler::new()` semantics.
  ~40 existing callers unchanged.
- Test unblocking (n161 Block 3 in PR #221; dept_parity in fix-forward commit):
  - n161 Block 3: 4 tests rewired to `examples/debug` base_dir. 1 TW test
    active (`block3_tw_serves_imported_pattern`), 3 VM tests re-ignored
    with n207/n208 anchor — VM route body divergence: pattern calls in
    route bodies return 500 on VM (key finding of №207).
  - dept_parity: 3 tests rewired to `examples` base_dir. 1 TW test active
    (`tw_serves_all_dept_branches_correctly`), 2 VM tests re-ignored with
    n207/n208 anchor (same root cause: RouteByDept user-pattern calls +
    query_param in route bodies).
- Total `#[ignore]` count: 126 → 124 (2 tests now active; 5 re-anchored
  to n207/n208).
- Scope note for №208 (recorded in ADR-0122): root cause is VM lacking
  user-pattern dispatch in route bodies (HTTP 500) — broader than the
  previously recorded query_param/json_body/respond gaps. 31 tests
  un-ignore when fixed.

### Added — Vision pillar skeleton (Naryad #210, ADR-0124)

- **Feature gate `vision`** (off-by-default, not in `default`/`full`).
  Enable with `cargo build --features vision`. The inference stack
  (model loading, generation) lands in R2/R3 (naryads 211/212).
- **`Value::Vision(VisionId)`** — opaque handle (same pattern as
  `Value::Reflex`). Display: `[Vision#N]`. type_name: `"vision"`.
  Vision artifacts never enter `Value` — only an index.
- **`VisionRegistry`** — owns vision artifacts behind `Mutex`
  (mirrors `ReflexRegistry`). API: insert/get/remove/len/is_empty/list_ids.
- **6 SSOT loud-stub builtins**: `vision_generate(3)`, `vision_edit(2)`,
  `vision_export(2)`, `vision_list(0)`, `vision_save(2)`, `vision_load(1)`.
  Each returns a loud error with naryad + ADR reference — not a
  placeholder value. `vision_list()` returns honest empty list.
  Registered append-only in `BUILTIN_REGISTRY` (383 → 389 spec! lines).
- **`vision-tests` blocking CI job**: builds with `--features vision`,
  runs tests + clippy. Guard step verifies `vision` is NOT in
  `default`/`full`.
- **7 contract tests** in `tests/naryad_210_vision_skeleton.rs`:
  vision_list empty, loud errors (TW + VM parity byte-for-byte),
  handle display, type_name, registry index stability.

## [0.19.0] - 2026-09-07

**The eighth semantic pillar — Reflex — is now complete: neural networks
as a first-class language construct. The VM backend gains Reflex parity
(stage 1 of ADR-0121). Security audit covers Reflex taint flows. mlogpkg
gains full dependency resolution + lockfile + local audit. The
self-hosted parser bootstraps. ~1000 commits since v0.18.0.**

### Added — Reflex pillar (complete: train/predict, sequence, generation, distillation)

- **reflex_gen — text generation with KV-cache** (Naryad №193, ADR-0120):
  `reflex_gen Name { input: embedding(dim) vocab_size: V layers: [transformer_block(...)] seed: N }`.
  Autoregressive generation with O(N) KV-cache (`forward_step` per layer).
  `reflex_generate(model, prompt, max_tokens, temperature)` — greedy and
  temperature-sampled decoding. 4-layer transformer generates coherent
  patterns on toy datasets.
- **reflex_tokenize / reflex_detokenize** (Naryad №194): character-level
  tokenization — simplest deterministic scheme, no vocabulary training.
  Each Unicode char → code point as Float.
- **BPE tokenization** (Naryad №195): `reflex_bpe_train`, `reflex_bpe_encode`,
  `reflex_bpe_decode`, `reflex_bpe_save`, `reflex_bpe_load`. Opaque
  `Value::BpeVocab` handle, `BPE_REGISTRY` global Mutex, deterministic
  training with lexicographic tie-break, binary serialize/deserialize.
- **Batched training** (Naryad №196): `[batch, seq_len, dim]` tensor with
  padding mask. Padding tokens excluded from loss and attention.
  `batch_size=1` matches single-sequence path byte-for-byte. Measured
  2-3x speedup on batch sizes 4-8.
- **Grouped-Query Attention (GQA)** (Naryad №188): `n_kv_heads` parameter
  on `attention` and `transformer_block`. K/V weights `[dim, kv_dim]`
  (not `[dim, dim]`). `repeat_kv()` for GQA. Backward compatible when
  `n_kv_heads == n_heads`.
- **Stacked transformer_blocks** (Naryad №190): multiple blocks in a
  `reflex_seq`/`reflex_gen` layers list. VarMap prefixing prevents
  weight collision (`block0_attn_w_q`, `block1_attn_w_q`, ...).
- **RmsNorm + SwiGLU + transformer_block** (Naryad №184): modular
  SequenceLayer types. `SEQUENCE_LAYER_REGISTRY` for name→constructor
  dispatch. `reflex_seq` uses sequence-only layers, `reflex` uses
  dense-only — mixing is a compile-time error (ADR-0119).
- **Attention layer** (Naryad №183): trainable attention with causal mask,
  RoPE positional encoding, `TrainableAttention` for autograd.
- **reflex_seq — sequence classification** (Naryad №185): mean pooling +
  Dense classifier head. `reflex_train`/`reflex_predict` dispatch through
  `ModelKind` enum (Dense | Sequence | Gen).
- **Reflex distillation** (Naryad №181, ADR-0117): `distill_to`,
  `distill_after`, `fallback_if` fields on `learnable pattern`. LLM
  traffic distilled into a local reflex model after N examples.
  TEACHING→DISTILLED→FALLBACK cycle.
- **Reflex persistence** (Naryad №180, ADR-0116): `reflex_save` /
  `reflex_load` — serialize model weights to SQLite. Version-tagged,
  shape-mismatch detection.
- **Reflex introspection** (Naryad №187): `reflex_metrics(model)` →
  Struct { param_count, last_metric, layers, input_size, labels }.
  `reflex_list()` → List of registered model names.
- **candle ML framework** (Naryad №175/183, ADR-0118): optional feature
  `--features candle`. CPU-only, no GPU. `VarBuilder`/`VarMap`/`Var`
  autograd. Language works without candle (default build) — Dense
  classification is pure Rust.
- **Reflex declaration** (Naryad №178, ADR-0114): `reflex Name { input:
  embedding(dim) layers: [...] labels: [...] seed: N }`. Opaque
  `Value::Reflex(ReflexId)` handle — weights never enter `Value`.
  `ReflexRegistry` owns models. Deterministic weight init via
  xorshift64 PRNG (ADR-0115).
- **Reflex training/prediction** (Naryads №177/179/179b): `reflex_train(model,
  data, epochs, metric, threshold)` → Struct { loss, accuracy, metric,
  threshold_met }. `reflex_predict(model, input)` → Fluid (label with
  confidence). 80/20 holdout split, cross-entropy loss, SGD.

### Added — VM Reflex parity (ADR-0121, stage 1 of 6)

- **VM-owned ReflexRegistry** (Naryad №199): `Vm` struct gains
  `reflex_registry: ReflexRegistry` + `reflex_names: HashMap<String,
  ReflexId>`. `reflex_train`/`reflex_predict` intercepted in
  `call_builtin` before the stub fallback. Same shared dispatch
  functions as the interpreter — neural-network logic not duplicated.
  Determinism verified: same seed → same output byte-for-byte across
  both backends. `crosscheck_backends` no longer excludes
  `reflex_train_predict.mlog`.

### Added — Security (Reflex + learnable taint model)

- **UNTRUSTED_TRAINING_DATA check** (Naryad №201, OWASP A09):
  `json_body()`/`query_param()`/`form_data()` → `reflex_train` data/labels
  → Error (model poisoning / PII baked into weights). New Category-A
  check_id, blocking.
- **SECRET_LEAK extended to reflex_train** (Naryad №201): `env()` →
  `reflex_train` data/labels → SECRET_LEAK Error. Weights persist via
  `reflex_save` (ADR-0116), bypassing file-level sinks. Interception
  on `reflex_train` args (not `reflex_save` — taint cannot sit on
  `Value::Reflex` opaque handle per ADR-0114).
- **HTML_INJECTION extended to reflex_generate + learnable patterns**
  (Naryad №201): `reflex_generate` output treated as `LlmOutput` taint
  (model trained on data that may include LLM-tainted content per
  ADR-0117). Learnable patterns (declared with `learnable pattern`)
  are also taint sources — their output is the result of an LLM call.
  `respond(Classify(x))` → HTML_INJECTION Warning.
- **List literal taint propagation** (Naryad №201): `get_expr_taint`
  now propagates taint through `Expr::List` — needed for
  `[[env("K"), 0.0]]` in `reflex_train` data.
- **max_tokens ceiling** (Naryad №203 Block 4): `reflex_generate`
  `max_tokens` capped at 4096 — explicit error, not silent truncation.
  Prevents resource exhaustion when `mlog serve` receives external
  request with `max_tokens=1e9`.
- **bind 127.0.0.1** (Naryad №164): server binds to localhost by default.
- **secret() builtin** (Naryad №172): `secret("KEY")` returns
  `Value::Secret` directly (hard-failure if env var missing, unlike
  `env()` which returns empty string).
- **SSOT audit** (Naryad №170): `BUILTIN_REGISTRY` is the single source
  of truth — compiler, VM, and semantic analysis all derive from it.

### Added — Tooling

- **candle-tests blocking CI job** (Naryad №200): new blocking job in
  `.github/workflows/ci.yml`. Runs `cargo test --workspace --features
  candle` (lib + 20 candle-gated integration tests) and
  `cargo clippy --features candle`. Existing `test-lib` job verifies
  ADR-0118 (language works without candle).
- **mlogpkg dependency resolution + lockfile + audit** (Naryad №198):
  Full transitive dependency graph resolution with version conflict
  detection and cycle detection. `mlogpkg.lock` (deterministic TOML,
  alphabetical). `mlogpkg audit` — checks dependencies against local
  advisory database (manually maintained, NOT external CVE integration).
  `mlogpkg add` pre-flight resolves before writing `mlog.toml`.
- **Self-hosted parser** (Naryad №197): `self-host/parser.mlog` —
  Metalogos parser written in Metalogos itself. Bootstraps (parses its
  own source). 4 lexer bugs fixed in local Tokenize copy. Structural
  equivalence with Rust parser verified on 12 .mlog files.
- **module-size-guard** CI job: per-module LOC limits (5000 hard,
  4000 warning).
- **vscode-extension** CI job: compiles TypeScript, verifies
  `out/extension.js` exists.
- **AGENT.md**: methodology document — code is source of truth, PR
  mandatory (ADR-0110), proofs by real CI runs.

### Fixed

- **VarMap collision in stacked blocks** (Naryad №190): TrainableAttention
  registered Vars under fixed names → stacked blocks overwrote each other.
  Fixed by adding `prefix` parameter.
- **GQA K tensor reshape** (Naryad №192): `apply_rope` had hardcoded
  `reshape((seq_len, self.dim))` — failed for GQA K tensor (smaller
  dim). Fixed to `reshape((seq_len, n_h * head_dim))`.
- **cross_entropy_loss scalar** (Naryad №193b): returned `[1,1]` tensor
  instead of scalar → `to_scalar` failed. Fixed with
  `.squeeze(0).squeeze(0)`.
- **KV-cache mismatch** (Naryad №193b): prompt processed via full
  `forward()`, but caches were empty → cache vs no-cache mismatch.
  Fixed: `forward_step` now used for ALL prompt positions.
- **golden error test divergence under candle** (Naryad №200):
  `collect_error_pairs` in `tests/golden.rs` skipped reflex_*.error
  pairs when `cfg!(feature = "candle")` is active. The .error files
  describe the candle-OFF message; under candle-ON the message differs.
- **stray .mlog files** (Naryad №203 Block 3): p161_deep_b/c,
  p161_route_helper moved from repo root to `examples/debug/`.

### Security

- All Reflex taint flows (Naryad №201) described above.
- `docs/threat-model.md` updated with 3 new risk rows:
  weights exfiltration, untrusted training data, model output as
  untrusted HTML.

## [0.18.0] - 2026-08-29

**Security hardening, SVG/graphics subsystem (44 builtins), VM backend parity,
office automation (PDF, email, calendar, contacts), code quality, and 60+ naryads of
improvements since v0.12.0.**

### Security — НАРЯД №131: `sandbox_path` symlink escape via `canonicalize()`
- `sandbox_path()` blocked absolute paths and `..` in text but did NOT
  resolve symlinks. A symlink inside the CWD pointing outside would pass
  both text checks and allow reading/writing arbitrary files.
- Added three-layer defence: (1) text checks preserved, (2) `canonicalize()`
  with `starts_with(canonical_base)` prefix verification, (3) ForWrite
  mode that canonicalizes only the parent directory (so `write_file` to
  a new file still works).
- `write_file`, `append_file`, `http_download` use `ForWrite` mode;
  `read_file`, `delete_file`, `file_exists`, `list_dir` use `ForRead`.
- 9 unit tests: normal read/write pass, symlink-to-outside rejected
  (file + subdir + dir symlink), write-to-new-file passes, absolute/`..`
  still rejected, broken symlink rejected.

### Fixed — НАРЯД №134: `collect_error_pairs` blind spot — 5 error contracts never ran in CI
- `collect_error_pairs` in `tests/golden.rs` had a hardcoded `p30_/p31_` prefix
  filter that silently skipped ALL other `.error` contracts, including
  `p114_secret_no_print.error` (Secret protection contract).
- Removed the prefix filter; now uses the same "pair exists → include" logic
  as `collect_pairs` for `.expected` files. Added deterministic sort order.
- Verified and updated all previously-skipped error contracts:
  - `err_undef.error`: updated message ("undefined entity" → "undefined variable")
  - `p2_multi_errors.error`: updated ("2 errors" → "unknown struct type: FakeType")
  - `p2_type_mismatch.error`: updated ("type mismatch" → "upper() expected String argument, got Float")
  - `p114_secret_no_print.error`: confirmed correct, no change needed
  - `err_unknown_step.mlog`/`.error`: **deleted** — program succeeds (soft-error
    pattern in `invoke()`), not an error contract. The pair was fundamentally
    wrong for the current architecture.
- Block 3 check: no other functions in `golden.rs` have the same hard-coded
  prefix pattern. `collect_pairs` uses justified exclusions (p7_, p88) with ADRs.
- **CI fix**: `p114_secret_no_print.mlog` pattern had 0 params but flow dispatch
  always passes 1 arg (arity mismatch). Fixed: pattern now accepts Secret
  param, flow passes entity through. Also fixed compilation errors in
  `naryad_128_secret_tests.rs` (wrong types: `Secret` takes `SecretString`,
  `Hash` takes `String`). Applied `cargo fmt` to all files.

### Fixed — НАРЯД №128: misleading `#[ignore]` Secret tests removed
- Two tests in `phase19_22_constraints.rs` (`test_z19_print_secret_forbidden`,
  `test_z19_to_string_secret_forbidden`) were marked `#[ignore]` with a comment
  claiming "Secret type constraints removed" — **incorrect and misleading**.
- The comment already provoked one incorrect external audit conclusion
  ("типовая защита секретов удалена").
- Investigation found Secret protection works through *different* mechanisms than
  the obsolete semantic checker the old tests targeted:
  - `print(secret)` → runtime `is_nonprintable()` + audit `SECRET_LEAK`
  - `to_string(secret)` → `Display` returns `[Secret]` (not the real value);
    audit taint tracker still propagates Secret taint to downstream sinks
- No actual vulnerability found for `to_string()` — it is safe by design.
- Old tests deleted; replacement contract tests added in `naryad_128_secret_tests.rs`
  documenting the *actual* protection mechanisms (5 tests).

### Fixed — НАРЯД №127: Dockerfile stub build silently failed — dependency cache never worked
- Root cause: `|| true` hid TWO failures in the stub build step: missing
  `src/lib.rs` (needed by mlogpkg/mlog-lsp that depend on metalogos lib)
  AND missing `benches/core_benchmarks.rs` (needed by `[[bench]]` manifest entry).
- Fix: added `src/lib.rs` and `benches/core_benchmarks.rs` stubs, removed `|| true`.
- Dependency caching layer now actually compiles — verified by simulating the
  stub build locally (`cargo build --release` succeeds in ~5 min).
- `2>/dev/null` kept: suppresses noisy dep compilation output (expected),
  but build failures now correctly surface (non-zero exit code).

### Security — НАРЯД №130: SSRF guard for http_get/http_post/http_post_multipart
- Outgoing HTTP requests now resolve DNS **before** connecting and block
  requests to loopback, private, link-local, and cloud metadata addresses.
- DNS rebinding protection: resolved IPs are pinned via `reqwest::ClientBuilder::resolve()`,
  preventing TOCTOU between check and connection.
- Opt-out: `METALOGOS_HTTP_ALLOW_PRIVATE=1` disables the guard for local dev
  and internal integrations. Guard is on by default.
- Protected ranges: `127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`,
  `169.254.0.0/16` (link-local + cloud metadata `169.254.169.254`), `::1`, `fe80::/10`.
- 8 contract tests: C1 loopback blocked, C2 cloud metadata blocked, C3 IP
  classification + public IP passes, C4 opt-out allows private.

### Fixed — Наряд №124: honestly document mock accuracy metric in `adapt`
- README §5: replaced unconditional "quality metrics, and automatic rollback
  on degradation. No analogues exist" with honest description — rollback
  mechanism is real, quality metric is a fixed mock (0.95). See ADR-0112.
- ADR-0112: documented open question — real quality metric requires semantic
  decision (what to measure, what to compare against), not mechanical addition.
  Revisit only when mock value creates a concrete problem in real `mutate` usage.

### Fixed — Наряд №126: sandbox timeout is now truly preemptive, not post-factum
- `invoke_learnable_with_env`: LLM calls in a sandbox with `timeout > 0` now run
  in a separate thread with `mpsc::recv_timeout`. The calling thread stops
  waiting at the deadline instead of detecting the timeout after the call
  already returned.
- Known limitation (honestly documented): the background HTTP request to
  the LLM provider may still be running — only the *wait* is cancelled.
  Full request cancellation requires `reqwest::AbortHandle`, a separate наряд.
- `MockLlm`: added `set_delay_ms`/`reset_delay` for timeout contract tests.
- 4 contract tests: C1 preemptive timeout within budget, C2 call completes
  within timeout, C3 no sandbox no timeout, C4 timeout=0 backward compat.

### Fixed — Наряд №125: CSRF cookie missing HttpOnly so JS can double-submit
- `_mlog_csrf` cookie: removed `HttpOnly` flag — JS clients must read this
  cookie to perform double-submit. Session cookie `_mlog_session` retains
  `HttpOnly; Secure` (correctly, opposite requirement).
- 4 integration tests: cookie lacks HttpOnly, double-submit accept,
  reject missing token, reject wrong token.
- README OWASP wording verified — already correct.

### Fixed — Наряд №123: Taint checks now catch nested calls, not only variables
- `check_html_injection`: `respond(call_llm(...))` and `respond(call_claude(...))` now flagged (previously only `respond(x)` where `x` is a variable was caught).
- `check_secret_leak`: `http_post(url, env("KEY"), headers)` now flagged
  (previously only variable references in http_post body were checked).
- `check_sql_dynamic`: verified — no gap (checks literal vs non-literal, not taint).
- New helpers: `expr_is_llm_tainted()`, `is_llm_source()`.
- 5 contract tests added (2 HTML_INJECTION + 2 SECRET_LEAK + 1 regression).
- README: removed `respond(call_llm(p, i))` from Known boundaries table;
  added `respond_html(query_param("url"))` (open-redirect inline nesting) as remaining gap.
- Note: `check_open_redirect` has the same inline-nesting gap; tracked separately.

### Fixed — Наряд №129: BlockIfElse in VM now produces loud compile error
- `Expr::BlockIfElse` (block if/else used as expression: `let x = if c then { ... } else { ... }`)
  previously compiled to `Const(Value::Unit)` in the VM, silently producing wrong results.
  Now returns a clear compile error: "block if/else expression not yet supported in VM bytecode".
- `Statement::IfElseBlock` (block if/else used as statement) is **not affected** — still fully supported.
- Two contract tests added: expression form fails, statement form still compiles.
- ADR-0105 updated: BlockIfElse gap description corrected, Наряд №129 referenced.
- No golden examples were masking this defect (verified).


### Наряд №121 — Отслеживание позиций в AST (span infrastructure, ADR-0111)

- **Feature:** Каждый узел АСТ теперь хранит своё положение в исходном коде
  (`Span { start_line, start_col, end_line, end_col }`). Все 15 вариантов
  `Expr`, 9 вариантов `Statement` и 41 структура `Declaration` содержат
  поле `span`.
- **Feature:** `Span::from_pest()` — метод для прямого преобразования
  `pest::Span` в `ast::Span`. Улучшен `Display`: однострочные спаны
  показывают `"строка:столбец"` вместо полного диапазона.
- **Feature:** `Expr::span()` и `Declaration::span()` — методы для получения
  ссылки на `span` из любого варианта перечисления.
- **Feature:** Ошибки семантического анализа показывают номер строки:
  `"строка N: duplicate entity type: User"` вместо `"duplicate entity type: User"`.
- **Refactor:** Все 15 кортежных вариантов `Expr` преобразованы в
  структурные с именованными полями (например, `StringLit(String)` →
  `StringLit { value, span }`). Аналогично `IfThen`, `Return`, `ExprStmt`
  в `Statement`.
- **Tests:** 5 новых тестов в `parser/tests.rs` проверяют реальные позиции
  в сообщениях об ошибках. Все 539 тестов проходят, crosscheck — 1 passed,
  0 failed.
- **ADR-0111:** Архитектурное решение: inline `span`-поле вместо обёртки
  `Spanned<T>`, 0-indexed столбцы / 1-indexed строки.

### Наряд №55 — Дыры в реестре блокируют перенос офиса

- **db_execute arity 1..2:** Registry updated to reflect ADR-0068 parameterised queries. Office 86 call sites now pass `mlog check`.
- **5 missing functions added to registry:** `to_int` (string, arity 1), `cron_add` (cron, 2), `cron_list` (cron, variadic), `cron_remove` (cron, 1), `cron_run` (cron, 1).
- **Automated sync test:** `registry_sync_check.rs` verifies builtin_count()/builtin_names()/builtin_name_set() agree with BUILTIN_REGISTRY. Catches funcs.insert without matching spec!.
- **ADR-0098:** Documents registry–dispatcher sync decision and registry-only categories.

### Наряд №52 — Перенос работы наряда №51 на актуальный main

- **Cherry-pick onto 74a1631:** 6 commits from naryad-51-workers ported to fresh branch from origin/main (which includes Наряды 49+50).
- **ADR-0096 merged:** Combined single-core worker diagnosis (№50) with nested block_in_place panic finding (№51) into one comprehensive ADR.
- **22 arity registry fixes:** Verified against actual implementations: `format`→variadic, `zip`→2, `filter`/`reduce`→3, `http_get`→1..3, `call_claude`→4, `call_llm`→1..2, `send_message`→2..3, `tts_send`→4..5, `geo_ip`→0..1, `web_search`→1..2, `weather_forecast`→1..3, `graph_query`→1..3, `graph_path`→2, `mtree_retrieve`→1..2, `estimate_tokens`/`extract_param`/`read_file_tokens`→exact, `answer_callback_query`→1..3, `edit_message_text`→3..4, `sort_by`→2..3.
- **Exhaustive arity test:** `registry_arity_check.rs` now tests every non-variadic builtin at min/max/boundary, plus all variadic builtins.
- **Block 2 (concurrency benchmark):** Not executed — Rust toolchain not available in this session. Owner can measure on live deployment.

### Наряд №51 — Конкурентность: воркеры tokio

- **Explicit worker count:** `METALOGOS_WORKERS` env var overrides default `max(4, available_parallelism)` workers. Logged at startup. Invalid values produce warning, not panic.
- **spawn_blocking fix:** Replaced 6 `block_in_place()` calls with `spawn_blocking()` in server.rs. `reqwest::blocking` inside `block_in_place` caused nested runtime drop panic. Interpreter and Vm verified as Send.
- **ADR-0097:** Documents spawn_blocking decision.
- **Branch cleanup:** Deleted 5 stale remote branches (naryad-41/42/43/49/50).

### Наряд №50 — Требования эксплуатации FOSVED

- **ADR-0071 honest re-accounting:** Of 92 original integration test failures, ~22 genuinely fixed, ~67 converted to `#[ignore]`.
- **Builtin arity range:** Added `max_arity: Option<usize>` to `BuiltinSpec` with `spec!` macro. ~59 registry entries corrected.
- **Unknown function detection:** `mlog check` now catches calls to nonexistent functions.
- **HMAC/SHA-256 builtins:** `sha256`, `hmac_sha256`, `hex_encode`, `hex_decode` with RFC vectors verified.
- **Concurrency root cause:** `block_in_place()` on single-core tokio (ADR-0096).
- **Registry sync test:** Spot-check test for key builtins with range arities.

### Typed memory with FTS5 BM25 + cosine RRF hybrid recall (ADR-0093, ADR-0094)
- Feature: `memorize(text, priority, type)` now accepts 3rd argument — type tag
  (`persona`, `episodic`, `instruction`, `fact`). Backward compatible: 2-arg form
  uses empty type.
- Feature: `recall_top_k(query, k, type)` — hybrid search returning JSON array of
  scored entries. FTS5 BM25 keyword index + cosine similarity, merged via
  Reciprocal Rank Fusion (k=60). Type filtering optional.
- Feature: FTS5 virtual table with content-synced triggers in SQLite schema.
- Feature: `mem_type` column + index on `memories` table.
- Fix: `load_all()` bug — SELECT now includes `mem_type` (was hardcoded empty).
- ADR-0093: memory typology + FTS5 foundation design decisions.
- ADR-0094: type-aware recall with RRF merge replacing weighted blend.

### PDF processing via pdf-inspector (Наряд №48)
- Feature: 4 native PDF builtins — `pdf_classify`, `pdf_to_markdown`,
  `pdf_extract_regions`, `pdf_ocr`. Pure Rust, zero IPC.
- Feature: `pdf_classify(path)` — classify PDF type (TextBased/Scanned/ImageBased/Mixed).
- Feature: `pdf_to_markdown(path)` — full extraction pipeline to Markdown.
- Feature: `pdf_extract_regions(path, filter)` — text items with coordinates and OCR status.
- Feature: `pdf_ocr(path)` — OCR fallback via tesseract-rs (requires `--features pdf-ocr`
  and system tesseract-ocr + CJK training data).
- Feature: optional `pdf-ocr` feature flag + tesseract 0.15 dependency.
- Test: 8 unit tests in pdf.rs + integration test file phase48_pdf_inspector.rs.
- Test: CJK fixture tests (5 files from pdf-inspector repo, verify no U+FFFD).

### Rule priority fix and golden cleanup — Наряд №43
- Fix: `execute_rules()` in both interpreter and VM now implements
  priority-ordered, first-wins semantics (ADR-0090). Previously all
  matching rules executed with last-write-wins, inverting priority.
- Fix: per-field deduplication via `HashSet<(String, String)>` — rules
  targeting different fields of the same entity all fire; only same-field
  conflicts are resolved by priority.
- Test: `p42_rule_priority_order.expected` updated 0.5 → 0.9.
- Test: `p42_rule_equal_priority.expected` updated 0.3 → 0.9.
- Test: `p43_rule_different_fields` added — both fields written.
- Fix: 5 stale golden expected files corrected (v05_integration,
  v05_string_ops, v05_let_bindings, p30_db_params, p31_malformed.error).
  Golden coverage: 66/70 pass (4 remaining are server/env-dependent p7_*).
- ADR-0090: rule priority semantics with prior art (CLIPS, Drools).

### Fluid Types, confidence, and rule tests — Наряд №42
- Test: 6 golden test pairs (`p42_fluid_*`) covering Fluid collapse
  semantics — type-directed selection, max confidence wins, threshold
  boundary (0.1), no matching variant soft-failure, non-Fluid passthrough.
- Test: 8 unit tests on `maybe_collapse` directly — threshold boundary,
  empty Fluid, type selection, non-Fluid passthrough.
- Test: 4 golden test pairs (`p42_rule_*`) covering rule engine — priority
  ordering (last-write-wins), equal priority (declaration order), probabilistic
  value assignment, condition-not-met no-change.
- Docs: renamed `p1_confidence_propagation` → `p1_fluid_collapse` —
  the example tested collapse, not propagation.
- ADR-0089: documents actual confidence semantics — no propagation through
  pattern calls; after collapse value is concrete; `confidence()` returns
  1.0 on concrete values. Open question: future propagation approaches.
- README verified: no false claims of confidence propagation.

### VM backend parity — Наряд №41
- Compiler: `match` statement returns compile error (`Err`) instead of
  silent Unit placeholder. Routes with `match` cannot compile for VM —
  prevents silently wrong results.
- VM audit log: `Vm` now has `audit_log: Mutex<Vec<String>>` with
  `push_audit()` / `take_audit_log()`. Adapt, Relate, Mutate instructions
  write audit entries matching interpreter format.
- Server: `flush_vm_audit_entries_to_db()` flushes VM audit to SQLite
  and in-memory log after each request. Closes OWASP A09 regression.
- Session hooks: investigated — `hooks_session_start`/`hooks_session_end`
  are startup-time only (in `run()`), NOT per-request. No discrepancy
  exists between backends. FOSVED does not use session hooks.
- Test: `test_n41_side_effect_parity` verifies both backends return same
  HTTP status for OK and crash cases.
- Test: `test_n41_match_not_compilable_in_vm` and
  `test_n41_non_match_routes_compile_in_vm` verify match compile error.
- `load_program()` benchmark on `app.mlog`: ~134 µs per request (debug).
  Acceptable for LLM-heavy routes; negates VM advantage for micro-routes.
- ADR-0088 updated: Block 1-5 results, corrected session hooks claim.

### VM backend for mlog serve (Наряд №40)
- `METALOGOS_SERVE_BACKEND` env var: `interpreter` (default) or `vm`.
  Unknown value → warning in log + fallback to interpreter, no panic.
- `CompiledRoute` struct in `bytecode.rs`: compiled route body bytecode.
- `Compiler::compile_routes()`: compiles route body statements to bytecode
  (reuses pattern body compilation pipeline).
- `Vm::load_program()`: initializes VM state without executing main_code
  (prevents flow execution during per-request VM setup).
- `Vm::execute_route_code()`: per-request route execution with fresh stack.
- `Vm` server-context: `set_server_json_body`, `set_server_query_params`,
  `set_server_user_roles` + `clear_server_context`.
- `Vm::call_builtin` intercepts `query_param`, `json_body`, `form_data`,
  `require` — same semantics as interpreter's FnCall dispatch.
- `ServerState` extended: `backend`, `vm_program`, `vm_routes`.
- Route compilation at startup (not per-request). Backend logged at start.
- `execute_route_body_vm()` in server.rs: VM route execution path
  with `block_in_place` for `!Send` Vm.
- Tests: env flag default/fallback, crash route → 500, OK route → 200,
  query param isolation, kv_set cross-request visibility.
- ADR-0088: VM backend for mlog serve — feasibility and implementation notes.

### And/Or in VM bytecode (Наряд №39)
- VM compiler: implemented `and`/`or` short-circuit evaluation using
  `JumpIfNot`/`Jump`/`Const` instructions. Semantics match interpreter:
  result is always `Value::Bool`, right operand not evaluated when left
  decides the result.
- Golden test `p39_and_or.mlog`: truth table, short-circuit verification
  via side effects, nesting, is_truthy on empty string/list.
- ADR renumbering: resolved 5 duplicate ADR numbers (0072-0076). Second
  instances moved to 0082-0087. Protected 0073/0075/0076 referenced in code.
- Created `docs/adr/README.md` with full index and numbering rule.

### Module size policy (Наряд №38)
- ADR-0080: module size policy — production files ≤2,000 lines, tests exempt.
  Supersedes the 800-line rule from №37.
- Interpreter: extracted `execution.rs` (1,645 lines) from `mod.rs` (2,178 → 539).
  Moved `run()`, `eval_expr()`, `eval_statements()`, `eval_binop()`, `invoke()`.
- Builtins: extracted `office.rs` (1,724 lines) from `server.rs` (2,579 → 865).
  Moved human, goal, todo, recipe, DAG, semantic search, config builtins.
- Builtin form audit: confirmed №37 split preserved `fn builtin_xxx()` form
  in all 8 extracted modules. No closure re-registration occurred.

### Code quality (Наряд №38)
- Clippy: zero warnings on `--all-targets` (was: compilation failure).
  Fixed unused imports, missing struct fields, private function access,
  bool_assert_comparison, len_zero, unnecessary_mut, cloned_ref_to_slice_refs,
  useless_conversion, redundant_closure across 15 files.
- CI: `clippy` job promoted from advisory to blocking. `continue-on-error` removed.
- Session store test helpers (`reset_session_store`, `session_key_count`,
  `session_store_count`) made `pub` for integration test access.

### VM feasibility assessment (Наряд №38)
- ADR-0081: VM-for-serve feasibility with FOSVED-office-v2 data.
  22/23 .mlog files pass `mlog check`. 4 files blocked by missing `And`/`Or`
  short-circuit evaluation in VM (92 combined occurrences). Single well-scoped
  fix needed before `mlog serve` can switch to VM backend.

### Code quality (Наряд №37)
- Clippy: zero warnings (was 192). Categories fixed: get(0)→first(), doc formatting,
  redundant closures, unnecessary mut/return/clone, new_without_default (11 types),
  dead_code cleanup, matches!/sort_by_key/clamp/flatten/Entry API, and more.
- CI: `fmt` gate restored to green with `cargo fmt` commit.
- ADR: resolved 8 numbering collisions (0076 duplicate, 071 missing leading zero).
  Renamed 0076-vm-dispatch-paths.md → 0077-vm-dispatch-paths.md.
- ADR-0075: clarified 58/58 crosscheck — zero both-error cases (all 58 are genuine
  both-success matches).
- Builtins: split builtins.rs (10,838 lines) into 15 modules: mod, registry, core,
  string, math, collections, crypto, llm, http, json, io, memory, cron, server, tests.
  mod.rs reduced to 581 lines.
- Interpreter: split interpreter.rs (5,073 lines) into 10 modules: mod, values, types,
  events, db, conversations, learnable, modules, flow, memory, hooks.
  mod.rs reduced to 2,172 lines.
- Parser: split parser.rs (4,921 lines) into 5 modules: mod, helpers, expr, stmt,
  decl, tests. mod.rs reduced to 128 lines.
- Documentation: added docs/refactoring-split-plan.md with per-function module mapping.
- No logic changes in any split — pure code moves.

### VM backend (Наряд №36)
- VM: crosscheck 58/58 — all golden examples match between tree-walking interpreter
  and bytecode VM. Zero mismatches, zero VM errors. `assert!(mismatches.is_empty())`
  now enabled in crosscheck test.
- VM: `find()` entity store query handler added — searches globals for structs
  matching type, field, and comparison operator.
- VM: `resolve_skill_index()` handler added — skill_index declarations now compiled
  into Program (CompiledSkillIndex/CompiledSkillTier/CompiledSkillTriggerRule).
- VM: database support — `db_conn`, `db_insert`, `query_scalar`, `query`, `db_execute`
  handlers added. DB URL extracted from `db` declaration at compile time.
- VM: schema DDL generation — `schema` declarations now generate
  `CREATE TABLE IF NOT EXISTS` SQL, executed at VM startup.
- VM: `context: recall(text, limit=N)` and `context: auto` now work in VM.
  Added `CompiledContextMode` enum (None/Auto/Recall/Literal) and `recall_top()`
  for multi-entry memory retrieval with `format_context_block` formatting.
- Compiler: `call_builtin` and `execute_code` changed to `&mut self` for DB support.
  Name cloning resolves borrow conflicts in CallBuiltin dispatch.
- ADR-0075 updated: all 9 remaining cases resolved. Crosscheck assertion enabled.

### VM backend (Наряд №35)
- VM: `eval_cmp()` now handles String-String comparisons (was Float-only via
  `as_float()`). `"" == ""` now correctly returns true. Fixes while/each loops
  that checked `result == ""` — crosscheck 45/58 → 48/58 (3 cases).
- VM: `MakeStruct` and `Contains` added to `execute_code()` (were only in `run()`).
  Pattern calls from flow pipelines now correctly handle struct literals.
  Fixes dag_demo.mlog — crosscheck 48/58 → 49/58 (1 case).
- VM: `CmpNe` added to `eval_cmp()` numeric path (was `_ => false`).
- ADR-0075 updated: 4 cases resolved in №35, 9 remaining documented with root causes.
  Crosscheck threshold raised to 49/58.
- Remaining VM divergences: memory subsystem (3), rule/find (1), flow source
  expression BinOp limitation (1), skill_index (1), DB builtins (2), modules (1).

### VM backend (Наряд №34)
- Compiler: While, Each, EachWithIndex, Assign, IfThen, IfElseBlock, Break,
  Continue, ExprStmt now compiled to bytecode (were silently dropped).
- Compiler: function-level scoping for LetBinding — `let` inside blocks overwrites
  outer variable, matching interpreter semantics (per p30_scope_let).
- Compiler: Expr::List now emits MakeList(count) (was broken — pushed Float(len)).
- VM: implemented MakeList, ListLen, Pop, StartsWith in both run() and
  execute_code() (were unimplemented!/silently skipped).
- VM: is_truthy() now handles Value::Bool correctly (Bool(true) was always false).
- Crosscheck TW vs VM baseline raised from 37/58 to 45/58 (8 cases closed).
  ADR-0075 documents all 21 remaining divergences.

### Documentation
- README: "Three Execution Backends" → "Two Execution Backends". JIT declared
  experimental (scaffold only, see ADR-0073). Cranelift removed from Prior Art.
- ADR-0075: full list of 21 TW vs VM divergences with categories and root causes.
- ADR-0086: performance baseline benchmarks (parser 178µs, interpreter 272µs,
  compiler 218µs, VM 36µs — VM 7.5× faster).

### Надёжность
- Парсер возвращает Result<_, ParseError> вместо аварийного завершения.
  27 вызовов std::process::abort() убраны, ошибка разбора теперь даёт
  диагностику с позицией line:col и код возврата 1 (ADR-0070)
- Golden test runner собирает ВСЕ failures перед panic — сломанные примеры
  не маскируют последующие тесты (Блок 2)
- p31_* error contracts покрыты автоматическими тестами (Блок 2)
- dag_demo.mlog исправлен: Demo() → Demo(input: String) (arity mismatch)

### Диагностика
- Триаж 92 integration test failures: 8 категорий (Блок 3, ADR-0071).
  219/311 integration tests pass. Ключевые группы: missing builtins (Phase 23),
  BUILTIN_REGISTRY gaps (8 Telegram/Voice entries), VM unimplemented (5 instructions),
  server-dependent (11 tests), immutable variable (4 tests).

### Added — SVG primitives (наряд №77, ADR-0102)
- `svg_rect`, `svg_circle`, `svg_line`, `svg_text`, `svg_path`,
  `svg_group`, `svg_canvas`, `svg_icon` (10 built-in glyphs),
  `svg_callout`, `svg_sketchy_filter`
- `chart_bar`, `diagram_style` (5-token `DiagramStyle`: paper/ink/
  accent/muted/rule)
- `svg_security_lint` static analysis pass in `semantic.rs` —
  `SVG_AUTO_ESCAPE_BUILTINS` / `SVG_NO_ESCAPE_BUILTINS`, catches XSS
  attempts (including string-concatenation evasion) at `mlog check`
  time

### Added — Palette + first composition (наряд №77)
- `color_palette(intent, mode)` — HSL-cascade generator, 5 intents ×
  2 modes, outputs `DiagramStyle`-compatible tokens
- `chart_donut`
- `std/infographic.mlog` — `InfographicPoster` pattern (MVP)

### Added — Chart types (наряды №78–79)
- `chart_line`, `chart_scatter` (independent two-axis scaling),
  `chart_area`
- `chart_heatmap` (HSL interpolation, no user text — intentionally
  excluded from the lint), `chart_radar` (multi-series, polar
  coordinates), `chart_boxplot` (real quartile computation, linear
  interpolation / R-7 method)

### Added — Procedural backgrounds + canvas presets (наряд №80)
- `svg_generate("flow"/"grid"/"noise", intent, w, h)` — deterministic,
  hash-based noise (no external noise crate)
- `svg_canvas_preset` — named viewBox presets (`doc_inline`,
  `slide_16x9`, `social_og`, `print_a4_landscape`, `print_a4_portrait`)

### Added — Diagram types, 22 total (наряды №81–84)
- Hierarchies/flow: `diagram_tree`, `diagram_org_chart`,
  `diagram_flowchart` (topological layering, cycle detection with a
  clear error), `diagram_layers`
- Temporal/process: `diagram_sequence`, `diagram_timeline`,
  `diagram_gantt`, `diagram_process`, `diagram_loop` (closed cycle via
  `polar_to_xy`)
- Sets/comparison: `diagram_venn` (2 or 3 circles, fixed symmetric
  geometry — general N-circle Venn intentionally out of scope),
  `diagram_quadrant`, `diagram_pyramid`, `diagram_nested`,
  `diagram_medallion` (reuses `svg_icon` validation)
- Data/state: `diagram_er`, `diagram_state` (cycles and self-loops are
  valid, unlike flowchart), `diagram_swimlane`, `diagram_data_flow`,
  `diagram_high_level`, `diagram_architecture` — all three graph-based
  types share a generalized `topological_layers`
- Shared primitive: `draw_connector` (arrow with computed head angle)

### Added — Retroactive crosscheck coverage (наряд №85)
- 36 `.expected` files generated for every example from наряды №77–84
  — none had been covered by `crosscheck_backends` before this naryad
- Found and fixed one real contract bug during the backfill
  (`p83_diagram_venn_2.mlog` used C-style `&&` instead of `and` —
  TW/VM had been "passing" only because both backends produced the
  same parse error)

### Added — Template engine (наряд №86)
- `template_render(template, data) -> Html` — new dedicated engine,
  built from scratch (the existing `render()` does not parse `{{ }}`
  at all and was left untouched)
- `{{ var }}` (auto-escaped), `{{{ var }}}` (raw, added ahead of
  schedule for naryад №90's SVG-in-HTML composition needs),
  `{{#if}}/{{else}}`, `{{#each}}` with `{{ this }}` context, verified
  nesting (`{{#each}}` inside `{{#if}}`)
- Template content itself is intentionally NOT auto-escaped — treated
  as trusted `.mlog`-author code, not end-user input

### Added — Anti-overlap engine (наряд №87)
- `estimate_text_width`, `resolve_overlaps` — iterative pairwise
  bounding-box displacement (not force-directed simulation)
- Wired into `diagram_timeline`, replacing the parity-alternation
  stopgap (kept as the initial seed position, refined by the real
  algorithm)

### Added — `html_render` + `exec()` hardening (наряд №88)
- `exec()`: configurable timeout (default 30s, ceiling 300s, real
  process kill on expiry), file-based audit log
  (`METALOGOS_AUDIT_LOG_PATH`) — added without moving `exec`/
  `html_render` into interpreter-special-cased dispatch
- `exec_restricted` — `Command::new(binary).args(args)`, no shell
  interpretation, closes a class of injection by construction
- `html_render(html, width, height)` — headless-browser screenshot via
  `METALOGOS_BROWSER_BIN` (no hardcoded path; clear error if unset or
  missing). Network isolation is NOT enforced at the OS level —
  documented, not hidden: caller is responsible for self-contained
  HTML (inline styles, `data:` URIs)

### Added — `infographic_qa` (наряд №89)
- WCAG-style contrast ratio check, saturation-discipline check
  (counts high-saturation colors in generated SVG), density check
  (element count / canvas area) — advisory only, `passed: false` is a
  suggestion, not a gate

### Added — Full `std/infographic.mlog` suite (наряд №90)
- `InfographicDashboard` (KPI cards + 2×2 chart grid),
  `InfographicComparison` (side-by-side, shared `chart_type`
  required), `InfographicTimeline` (thin wrapper over
  `diagram_timeline`, anti-overlap applies automatically)
- All three reuse `InfographicPoster`'s header/footer visual grammar

### Fixed — Critical: VM discarded `try`'s result on the success path
- `src/compiler.rs` compiled `Expr::Try(_)` as `Const(Unit)`
  unconditionally since наряд №14 — the wrapped expression was never
  evaluated by the VM at all
- New `Instruction::TryEval(Vec<Instruction>)` — compiles the inner
  expression into its own block, executes it, pushes the real value on
  success or `Unit` on error (matching tree-walking semantics exactly)
- Found by accident during наряд №90; masked for the entire project
  history because all 30 pre-existing `try`-using golden examples only
  tested the error path, where `Unit` happened to be correct either
  way — first golden contract testing the success path is
  `p91_try_success_path.mlog`

### Added — Security audit sweep (наряд №92)
- Classified all 44 SVG/graphics builtins: 0 real gaps found (23
  initial suspects from a naive array-membership grep were false
  positives — either legitimately excluded, e.g. `template_render`,
  `infographic_qa`, `chart_heatmap`, or covered via `SVG_NO_ESCAPE_BUILTINS`
  and dedicated per-function scanners not visible to a literal-array search)
- Added 26 injection tests for the `diagram_*` family — 0 existed
  before this naryад, despite наряд №84's report claiming coverage was
  confirmed (the scanners were real and wired correctly; the tests
  proving they fire were simply never written)

### Changed
- Cargo.toml version 0.17.0 → 0.18.0
- `registry_arity_check.rs` promoted from `test-integration` (advisory)
  to its own `registry-arity-check` (blocking) CI job — the same
  regression class that let a stale `http_get`/`http_post` arity
  assertion sit unnoticed for days (see наряд №73)

## [0.16.0] - 2026-08-13

### Added
- card_connect — подключение к CardDAV-серверу (PROPFIND, addressbook-home-set discovery)
- card_list — список адресных книг (PROPFIND Depth:1)
- card_contacts — контакты из адресной книги с фильтрацией (CardDAV REPORT addressbook-query, RFC 6352 §8.6)
- card_read — чтение одного контакта по URL
- card_create — создание контакта (PUT .vcf, возвращает UID, arity 3..7)
- card_update — обновление полей контакта (GET+PUT с ETag/If-Match)
- card_delete — удаление контакта (DELETE с If-Match)
- card_search — поиск по всем адресным книгам (FN + EMAIL)
- vcard_parse — парсинг vCard текста в JSON (RFC 6350, hand-rolled parser)
- vcard_generate — генерация vCard текста из JSON (v4.0)
- 14 inline-тестов в contacts.rs (UUID, vCard parse/generate/roundtrip, folding, escaping)
- Интеграционные тесты tests/phase_mlg6_contacts.rs (10 тестов)
- CardDAV config через env vars: CARDDAV_URL/USER/PASS

### Changed
- Cargo.toml version 0.15.0 → 0.16.0
- BUILTIN_REGISTRY: +10 contacts functions (category "contacts")
- Builtins::new(): +10 dispatcher entries for card_*/vcard_*

## [0.15.0] - 2026-08-13

### Added
- cal_connect — подключение к CalDAV-серверу (PROPFIND, calendar-home-set discovery)
- cal_list — список календарей (PROPFIND Depth:1)
- cal_events — события в диапазоне дат (CalDAV REPORT calendar-query, RFC 4791 §7.8)
- cal_read — чтение одного события по URL
- cal_create — создание события (PUT .ics, возвращает UID)
- cal_update — обновление полей события (GET+PUT с ETag/If-Match)
- cal_delete — удаление события (DELETE с If-Match)
- cal_freebusy — запрос занятости (CalDAV REPORT free-busy-query, RFC 4791 §7.10)
- ical_parse — парсинг iCalendar текста в JSON (ical crate, RFC 5545)
- ical_generate — генерация iCalendar текста из JSON (VEVENT + VCALENDAR)
- ical (v0.8), chrono-tz (v0.10) dependencies
- 10 inline-тестов в calendar.rs (datetime formatting, iCal escaping, parse, generate, roundtrip)
- Интеграционные тесты tests/phase_mlg5_calendar.rs (10 тестов)
- CalDAV config через env vars: CALDAV_URL/USER/PASS

### Changed
- Cargo.toml version 0.14.0 → 0.15.0
- BUILTIN_REGISTRY: +10 calendar functions (category "calendar")
- Builtins::new(): +10 dispatcher entries for cal_*/ical_*

## [0.14.0] - 2026-08-13

### Added
- smtp_send — отправка plain-text email через SMTP (lettre crate, TLS/STARTTLS)
- smtp_send_html — отправка HTML email через SMTP
- imap_list — список входящих писем (IMAP, envelope + flags)
- imap_read — чтение полного письма (заголовки, тело, вложения)
- imap_search — поиск писем по тексту (TEXT criteria)
- imap_mark_read — пометка письма как прочитанного
- imap_move — перемещение письма в другую папку (RFC 6851 MOVE / fallback COPY+DELETE)
- lettre (v0.11), imap (v3.0.0-alpha.15), imap-proto (v0.16), native-tls (v0.2) dependencies
- 6 inline-тестов в email.rs (env guard, content type, header parsing, flag detection)
- Интеграционные тесты tests/phase_mlg4_email.rs (10 тестов)
- Email config через env vars: SMTP_HOST/PORT/USER/PASS/FROM, IMAP_HOST/PORT/USER/PASS

### Changed
- Cargo.toml version 0.13.0 → 0.14.0
- BUILTIN_REGISTRY: +7 email functions (category "email")
- Builtins::new(): +7 dispatcher entries for smtp_*/imap_*

## [0.13.0] - 2026-08-12

### Added
- pdf_draw_table — таблицы в PDF (Наряд MLG-3)
- pdf_add_image — вставка PNG/JPEG изображений
- pdf_set_page_header / pdf_set_page_footer — колонтитулы
- pdf_page_numbers — автоматическая нумерация страниц
- pdf_watermark — водяные знаки (диагональный текст с прозрачностью)
- pdf_fill_form — заполнение AcroForm-полей
- pdf_rotate_page — поворот страниц (90/180/270°)
- pdf_delete_pages — удаление страниц
- pdf_extract_images — извлечение изображений из PDF
- html_to_pdf улучшен: базовый рендер на чистом Rust с fallback на wkhtmltopdf
- png crate dependency (v0.17) для декодирования PNG-изображений
- 18 inline-тестов в pdf.rs для новых функций
- Интеграционные тесты tests/phase_mlg3_pdf_office.rs (13 тестов)
- Пример examples/p_pdf_office.mlog

### Changed
- PdfDocument struct: добавлены поля header, footer, watermark, page_number_format, page_number_pos
- PdfElement enum: добавлены вариации Table, Image, Watermark
- html_to_pdf: приоритет Rust-рендера (простой HTML) над wkhtmltopdf (сложный HTML)
- render_pdf: поддерживает Table/Image/Watermark элементы, рендерит header/footer/page_numbers/watermark на каждую страницу

## [0.12.0] - 2026-07-30

**Production hardening (наряды №29 и №30).**

### Безопасность
- .env вычищен из истории git и из всех веток
- HMAC-ключ сессий читается из METALOGOS_HMAC_KEY (раньше генерировался
  при каждом старте — сессии слетали при рестарте)
- CSRF-токены получили TTL 15 минут и фоновую очистку (раньше росли без границ)
- SECRET_LEAK: обнаружение секрета в теле http_post по позиции аргумента
  (ADR-0064) — заголовки остаются штатной авторизацией
- unsafe-блоков: 5 -> 1 (остался только Cranelift JIT, задокументирован)

### Надёжность
- Сессии, CSRF и rate limits переведены на DashMap
- Конкурентная обработка запросов: вызовы интерпретатора обёрнуты в
  tokio::task::block_in_place, лок планировщика сокращён (ADR-0067)
- Граф памяти переведён на StableDiGraph: удаление узла больше не портит
  индексы остальных (ADR-0066)
- Типизированные ошибки: RuntimeError через thiserror, хелпер lock_or_err

### Язык
- +slice(list, start, end) — срез списка, семантика зеркалит substring (ADR-0069)
- db_execute принимает необязательный список параметров — паритет с query()
  (ADR-0068). Склейка SQL больше не единственный способ
- Семантика зафиксирована golden-контрактами: let во вложенном блоке
  присваивает внешней переменной; присваивание требует let mut;
  kv_get на отсутствующем ключе возвращает пустую строку

### Тесты и CI
- Unit-тесты: 233 -> 373
- GitHub Actions: блокирующие test-lib и fmt, advisory test-integration и clippy
- Устранена гонка env-переменных в параллельных тестах llm.rs
- Cargo.lock взят под контроль версий, сборки воспроизводимы

### Сборка
- Dockerfile: rust 1.85, запуск от непривилегированного пользователя

### Известные ограничения
- 92 из 310 интеграционных тестов красные (накопленный долг, триаж — наряд №31)
- 191 clippy-предупреждение (джоб advisory)
- Fluid Types и confidence propagation не покрыты тестами
- BUILTIN_REGISTRY и Builtins::new() рассинхронизированы: 67 вызываемых
  функций отсутствуют в реестре, 44 записи реестра не имеют обработчика

## [0.11.0] — 2026-07-23

**Lifecycle hooks + YAML config (Наряд O-2).**

Расширение lifecycle hooks с 2 до 5 точек и поддержка YAML в config_load. Концепции вдохновлены [obsidian-mind](https://github.com/breferrari/obsidian-mind) (TypeScript, 3.5k★, MIT — код НЕ копировался, только архитектурные концепции).

### Lifecycle hooks (2 → 5)

- `hook on_session_start { ... }` — срабатывает один раз в начале `run()`, после регистрации всех деклараций.
- `hook on_write { ... }` — срабатывает перед каждым мутирующим билтином (mem_set, mtree_store, db_execute, write_file, append_file). Переменные: `target` (String), `args` (List).
- `hook on_session_end { ... }` — срабатывает один раз в конце `run()`.
- Существующие `before_pattern` / `after_pattern` без изменений (ADR-0045).

### config_load — поддержка YAML

- `config_load(path)` теперь автоматически определяет формат по расширению: `.yaml`/`.yml` → YAML, иначе → JSON.

### Новые зависимости

- `serde_yaml = "0.9"` — парсинг YAML конфигов.

### Изменённые файлы

- `src/grammar.pest` — 3 новых токена (on_session_start, on_write, on_session_end), расширен hook_kind, step_ident negative lookahead
- `src/ast.rs` — HookPhase: 2 → 5 вариантов (OnSessionStart, OnWrite, OnSessionEnd)
- `src/parser.rs` — parse_hook_decl: обработка 5 точек
- `src/interpreter.rs` — 3 новых поля, two-phase run(), fire_on_write_hooks() в 3 точках вызова
- `src/builtins.rs` — config_load: YAML поддержка + yaml_to_json_value() helper
- `Cargo.toml` — версия 0.11.0, serde_yaml
- `docs/adr/0064-obsidian-mind-lifecycle-hooks.md` — АДР
- `docs/adr/0065-config-load-yaml.md` — АДР
- `examples/hooks_lifecycle.mlog` — демо всех 5 lifecycle hooks

## [0.10.0] — 2026-07-23

**Vault/memory builtins inspired by [obsidian-mind](https://github.com/breferrari/obsidian-mind) (MIT — код НЕ копировался, только архитектурные концепции).**

### Новые builtins (3)

**Семантический поиск:**

- `semantic_search(query, documents, top_k)` — семантический поиск по списку документов. Возвращает список `SearchResult{index, text, score}`. Использует EmbeddingManager: OpenAI text-embedding-3-small если `METALOGOS_EMBEDDING_API_KEY` задан, иначе TF-IDF fallback. Вдохновлён QMD semantic search из obsidian-mind.

**Конфигурация и валидация:**

- `config_load(path)` — загрузка JSON-файла конфигурации в struct. Имя типа берётся из имени файла (stem). Вдохновлён vault-manifest.json — coordination point pattern из obsidian-mind.
- `vault_validate(config, required_fields)` — проверка, что struct содержит все указанные обязательные поля. Возвращает `ValidationResult{valid, missing}`. Вдохновлён frontmatter_required из obsidian-mind.

### Изменённые файлы

- `src/builtins.rs` — 3 новых builtin (semantic_search, config_load, vault_validate), импорт EmbeddingManager, BUILTIN_REGISTRY entries

## [0.9.6] — 2026-07-23

**Narad ML-1: host key in mlogserver + json_get NULL fix.**

### Bug fixes

- **mlogserver `host:` key** (баг №2): блок `mlogserver` теперь принимает опциональный ключ `host: "127.0.0.1"` для биндинга на указанный адрес вместо жёстко зашитого `0.0.0.0`. Закрывает гонку портов на Render. Обратная совместимость: отсутствие `host:` → дефолт `"0.0.0.0"`.
- **`json_get` SQL NULL** (баг №1): `json_get(row, key, default)` теперь возвращает `default`, когда значение поля — SQL NULL (`Value::Unit`). Раньше возвращал `Unit`, что вызывало `type mismatch` при конкатенации `String + Unit`. Двухаргументная форма (без default) не изменена.

### Изменённые файлы

- `src/grammar.pest` — правило `mlogserver_host`, `"host"` в `step_ident` исключениях
- `src/ast.rs` — поле `host: Option<String>` в `MlogServerDecl`
- `src/parser.rs` — разбор `host` в `parse_mlogserver_decl`
- `src/server.rs` — биндинг на `config.host` с fallback `"0.0.0.0"`
- `src/builtins.rs` — проверка `Value::Unit` в 3-аргументной ветке `json_get`

## [0.9.5] — 2026-07-21

**OpenPlanter-inspired: Agent utility builtins (ADR-0063).**

Концепции заимствованы из https://github.com/ShinMegamiBoson/OpenPlanter (MIT — код НЕ копировался, только идеи).

### Новые зависимости

- `strsim = "0.11"` — Jaro-Winkler нечёткое сравнение строк
- `crc32fast = "1.4"` — быстрая CRC32-хеширование

### Новые builtins (8)

**Нечёткое сравнение (fuzzy matching):**

- `fuzzy_match(a, b)` — Jaro-Winkler сходство двух строк (0.0..1.0). Основано на OpenPlanter `wiki/matching.rs::NameRegistry`.
- `fuzzy_find_best(query, candidates)` — лучший матч из списка кандидатов → `FuzzyMatch{index, candidate, score}`.

**Контент-верифицированное редактирование (hashlines):**

- `hashline_read(text)` — аннотировать строки 2-символьным CRC32-хешем: `N:HH|content`. Предотвращает LLM-редактирование устаревшего контента.
- `hashline_edit(text, edits)` — редактирование с верификацией хешей. 3 операции: `set_line`, `replace_lines`, `insert_after`. Ошибка при несовпадении хеша.

**Утилиты агента:**

- `compact_list(items, keep_first, keep_last)` — контекстная компактификация: защита головных/хвостовых элементов, среда схлопывается в `Compacted{compacted: true, removed_count: N}`. Аналог OpenPlanter `compact_messages()`.
- `budget_check(step, total_steps)` — осведомлённость о бюджете → `BudgetStatus{step, total, remaining, pct_remaining, level}`. Уровни: "ok" (≥50%), "warning" (≥25%), "critical" (<25%).
- `replay_snapshot(data)` — дельта-логирование: seq 0 = полный снапшот → `ReplaySnapshot{seq, count, snapshot}`. Аналог OpenPlanter `ReplayLogger`.
- `policy_check(command)` — проверка безопасности shell-команды → `PolicyResult{allowed, reason}`. Блокирует heredoc (`<<`) и интерактивные программы (vim, nano, less и т.д.).

### Изменённые файлы

- `src/builtins.rs` — 8 новых builtin'ов + 2 helper'а + 20 тестов (~590 строк).
- `Cargo.toml` — версия 0.9.5, зависимости `strsim`, `crc32fast`.
- `docs/adr/0063-openplanter-agent-utilities.md` — ADR.
- `examples/openplanter_demo.mlog` — демонстрация всех 8 builtin'ов.

## [0.9.4] — 2026-07-16

**AgentSkillOS-inspired: Recipe system + DAG orchestration builtins (ADR-0062).**

Концепции заимствованы из https://github.com/ynulihao/AgentSkillOS (MIT — код НЕ копировался, только идеи).

### Новые builtins (5)

- `recipe_save(name, description, skills, plan)` — построить рецепт (struct с key + recipe), для сохранения через `kv_set`. Возвращает `{key: "__recipe:<name>", recipe: {...}}`.
- `recipe_search(query)` — placeholder для семантического поиска рецептов. Возвращает пустой список (требует embedding infrastructure).
- `recipe_list()` — placeholder для списка рецептов. Возвращает пустой список (требует KV access из builtin context).
- `dag_phases(dag)` — извлечь параллельные фазы выполнения из DAG. Вход: список `{id, depends_on}`. Выход: список фаз (списков ID). Kahn's algorithm + детекция циклов.
- `topo_sort(dag)` — топологическая сортировка DAG. Тот же формат входа. Выход: плоский список ID в порядке зависимостей.

### Изменённые файлы

- `src/builtins.rs` — 5 новых builtin'ов + 13 тестов (~300 строк).
- `docs/adr/0062-agentskillos-recipe-dag.md` — ADR с описанием архитектуры.
- `examples/dag_demo.mlog` + `.expected` — golden test для dag_phases/topo_sort.

### Ограничения

- `recipe_search`/`recipe_list` — placeholders, полная реализация требует доступа к KV-хранилищу из builtin context.
- Нет семантического поиска рецептов (требует embeddings).

## [0.9.3] — 2026-07-12

**sqz-inspired builtins и declaration (P1+P2+P3).**

Концепции заимствованы из https://github.com/ojuschugh1/sqz (ELv2 — код НЕ копировался, только идеи).

### P1 — Строковые/списковые утилиты (10 builtin'ов)

- `squeeze(s, chars)` — схлопнуть идентичные соседние символы (аналог Ruby String#squeeze).
- `dedup(list)` — удалить дубликаты, сохраняя порядок первого вхождения. Сравнение через JSON для сложных типов.
- `condense(list)` — схлопнуть идентичные соседние строки с подсчётом повторов (формат: элемент, "×N").
- `strip(s, chars)` — удалить символы с обоих концов строки (аналог Python str.strip).
- `chomp(s)` — удалить один trailing newline (\n или \r\n, аналог Ruby String#chomp).
- `repeat(s, n)` — повторить строку n раз. Проверка: n >= 0, целый.
- `pad_left(s, n, fill)` / `pad_right(s, n, fill)` — дополнить строку символом fill до длины n.
- `lines(s)` — разбить на список строк по \n, без trailing пустого элемента.
- `words(s)` — разбить на список слов по whitespace.

### P2 — TOON encoding + content-addressed refs

- `toon_encode(value)` — кодировать любое значение в TOON (Token-Optimized Object Notation). Префикс `TOON:`, ключи без кавычек, non-ASCII → `\u{XXXX}`. Lossless.
- `toon_decode(s)` — декодировать TOON обратно в Value. Recursive descent parser. Проверка префикса, валидация JSON-like синтаксиса.
- `ref(content)` — SHA-256 хэш, сохранить в KV-хранилище (`__ref:HASH`), вернуть hex-строку (64 символа). Idempotent (INSERT OR IGNORE).
- `deref(hash)` — восстановить содержимое по хэшу. Валидация формата (64 hex символов), ошибка если не найден.

### P3 — Token awareness

- `token_count(text)` — оценка количества токенов: кириллица chars/2, латиница chars/4, порог 50%.
- `context_budget` — новое объявление верхнего уровня: `context_budget { pattern: "name", limit: 4096 }`. Хранит токенный бюджет для learnable pattern'ов в `Interpreter.context_budgets` HashMap.

### Изменённые файлы

- `src/builtins.rs` — 15 новых функций + 52 теста.
- `src/grammar.pest` — правило `context_budget_decl`.
- `src/ast.rs` — `ContextBudgetDecl` struct + `Declaration::ContextBudget` variant.
- `src/parser.rs` — `parse_context_budget_decl`.
- `src/interpreter.rs` — обработка `ContextBudget` в `run()` и `clone_definitions_into()`, поле `context_budgets`.
- `src/compiler.rs` — `ContextBudget` в catch-all arms (pass1 + pass2).

### Тесты

- 52 новых теста в `mod tests_sqz_builtins`. Все pass.
- Итого: 196 passed, 3 failed (pre-existing), 3 ignored.

## [0.9.2] — 2026-07-12

**Заплатка: исправление 5 ошибок компиляции E0004 (non-exhaustive patterns) после Problem A/B/C/D/E.**

- `compiler.rs`: `BinOp::And`/`Or` — добавлена явная ветка с ошибкой компиляции (short-circuit evaluation не реализован в VM bytecode, требуется tree-walking interpreter).
- `vm.rs` main loop: `Instruction::MakeList`, `ListLen`, `Pop`, `StartsWith` — добавлены ветки `unimplemented!` с поясняющим сообщением (VM bytecode support отложен).
- `vm.rs` `eval_branch_condition`: `ConditionOp::Ne` — реализована семантика `!=` (по аналогии с `Eq`).
- `vm.rs` `eval_rule_condition`: `&ConditionOp::Ne` — реализована семантика `!=` (по аналогии с `Eq`).
- `vm.rs` `eval_binop` Float branch: `BinOp::And`/`Or` — добавлена ветка, возвращающая runtime-ошибку (булева логика некорректна для Float operands).

## [0.9.1] — 2026-07-12

**Наряд 4-примитивов: Problems B + D (Problem B: aggregation, Problem D: webhook diagnosis).**

### Problem B — Aggregation over list of structs (ADR-0059)

- **`map()` в VM** — `map(list, "pattern_name")` теперь работает во всех трёх бэкендах (tree-walking, bytecode/VM, JIT). Ранее — только tree-walking.
- **`map`, `zip`, `sort_by`, `filter`, `reduce` добавлены в BUILTIN_REGISTRY** — ранее отсутствовали, компилятор не мог создать `CallBuiltin` для них.
- **`IndexAccess` в execute_code** — паттерны в VM теперь могут использовать `list[N]` и `struct["key"]` (раньше инструкция обрабатывалась только в main loop).
- **`entity` как struct** — STOP Trigger #1 подтверждён: `entity TypeName { ... }` полностью покрывает потребность в `struct`. Новый ключевой код не добавлен (ADR-0059).

### Problem D — Webhook routing diagnosis (ADR-0061)

- Диагностика: `Hook` (ADR-0045) — AOP для паттернов, не для HTTP. `route` — полноценный HTTP-роутер, достаточный для Telegram webhook. Корень бага — архитектурный (reverse_proxy.py маршрутизирует `/webhook/*` в Python, mlog-обработчик физически недостижим).
- Golden test: `telegram_webhook_route.mlog` — проверяет `parse_json` + `json_get` на mock Telegram update JSON.

### Problem C — Schema-as-code (ADR-0060)

- Новая декларация `schema name { table T { ... } }` — DECLARE таблиц прямо в .mlog файлах
- Auto-migration при старте: `CREATE TABLE IF NOT EXISTS` (additive-only, никогда не drop/alter)
- Поддерживаемые типы: Int, Float, String, Text, Bool, DateTime
- Модификаторы: primary_key, auto_increment, nullable, references(table.field)
- Дефолты: default("value"), default(now())
- Интеграционные тесты: schema + db_insert + query round-trip, additive migration
- **Ограничение**: schema DDL и db_insert работают только в tree-walking режиме (требуют SQLite connection). VM/JIT путь отложен.

### Problem A — Tiered Skill Index (ADR-0058)

- Новая декларация `skill_index name { tier N always [...] | tier N when_matches [...] budget: N tokens truncation: mode }`
- AST: SkillIndexDecl, SkillTier, SkillTriggerRule, TruncationMode
- Grammar: 12 new PEG rules (skill_index_decl, skill_tier, tier_always_list, tier_matches_list, etc.)
- Parser: 2 new parse functions
- Interpreter: `skill_indices` HashMap, `resolve_skill_index` + `fit_to_budget` builtins
- `fit_to_budget` MVP: pass-through (полная реализация с file I/O отложена)
- 5 интеграционных тестов: базовая загрузка, trigger matching, budget/truncation, error handling, 3 tiers
- STOP Trigger #4 задокументирован: бюджет per-model, не глобальная константа (известное ограничение MVP)

---

## [0.9.0] — 2026-07-07

**Unified Builtin Registry — Single Source of Truth refactoring.**

### Architecture

- **`BuiltinSpec` struct + `BUILTIN_REGISTRY` const** — 135 builtins with name, arity, and category in a single master table (`builtins.rs`)
- **Helper functions** — `builtin_names()`, `builtin_indices()`, `builtin_name_set()`, `builtin_arity_map()`, `is_builtin()`, `builtin_count()` — all derived from the registry
- **compiler.rs** — hardcoded 26-entry builtin array replaced with `builtin_indices()` call
- **vm.rs** — hardcoded 26-entry `builtin_names` vec replaced with `builtin_names()` call
- **semantic.rs** — hardcoded 28-entry `builtin_names` set replaced with `builtin_name_set()` call
- **Debug sync check** — `Builtins::check_registry_sync()` asserts (in debug builds) that every non-stateful registry entry has a handler in `Builtins::new()`
- **Duplicate `env` registration removed** (was inserted twice at lines 28 and 70)
- **Before**: adding 1 builtin required editing 5 files; **After**: 1 row in `BUILTIN_REGISTRY` + 1 insert in `Builtins::new()`

### Registry categories

135 builtins organized into categories: string, convert, list, math, std, web, json, crypto, auth, db, llm, memory, io, time, bot, voice, stateful, graph, mtree, cron, test, encoding, stub, fluid, system

---

## [0.7.8] — 2026-06-15

**Наряд №17 closure: BlockIfElse expression in bytecode compiler, format() arity fix.**

### Bytecode compiler

- **`Expr::BlockIfElse` full bytecode compilation** — `if cond { ... } else { ... }` as expression now compiles to a proper conditional jump chain with result slot, instead of emitting `Const(Unit)` placeholder (Наряд 17 Б.1)
- New `compile_body_expr` method — compiles statement blocks in expression context, storing the last expression's value into a result local slot
- `format()` arity corrected from `-1` (variadic) to `1` (template-only) in semantic arity checks

### Bug fixes

- Block if/else expression in VM path no longer silently returns `Unit`; the value of the last expression in the matched branch is correctly propagated to the stack

---

## [0.7.7] — 2026-06-14

**Phase 7.7: Break/Continue, Match arms, compiler full-coverage, security constraints.**

### Language

- **`break` and `continue`** statements in `each`, `each_with_index`, and `while` loops (Наряд 17)
- **`MatchArm::StartsWith`** — bytecode instruction `StartsWith` + VM execution + compiler codegen (Наряд 17)
- **`MatchArm::Compare`** — threshold-based match arms with full compiler support
- **`Statement::IfElseBlock`** — multi-branch `if/else if/else` as statement with full compiler coverage (Наряд 18)
- **`Expr::BlockIfElse`** — block if/else as expression in interpreter (Наряд 14)
- **`Expr::Try`** — try/catch expression, catches errors and returns `Unit` (Наряд 14)

### Bytecode compiler

- Full statement compilation: `LetBinding`, `Assign`, `Return`, `ExprStmt`, `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Match`, `Break`, `Continue` (Наряд 18)
- Loop context (`LoopCtx`) for break/continue jump patching — continue jumps back to condition, break jumps to loop end
- `Match` with `Exact`, `StartsWith`, `Contains`, `Compare` arms — all compiled to conditional jump chains
- Global variable slots, `StoreGlobal` instruction (Наряд 22)
- 44 total VM instructions in the bytecode instruction set

### VM

- `StartsWith` instruction — string prefix check, pushes 1.0 (true) or 0.0 (false)
- `StoreGlobal` instruction — write to global variable slot
- `execute_code` method with `&mut self` for mutable global state in pattern execution
- `IndexAccess`, `ListLen`, `MakeList`, `MakeStruct`, `GetField` — collection and struct support

### Semantic analysis

- Opaque type enforcement across all statement types: `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Match` (all 4 arm variants)
- Tool declaration body analysis
- Static security audit (`mlog audit`) coverage for new statement forms

### Security constraints (Наряды 19–22)

- `inspect` builtin — introspect variable values without violating opaque types (Наряд 19)
- Context loading from `Entity`/`Memory`/`Fluid` declarations before pattern execution (Наряд 20)
- Event streaming: `emit`/`on` event hooks (Наряд 20)
- Conversation state: `Conversation` declaration with TTL and message limits (Наряд 21)
- LLM response cache with configurable TTL (Наряд 21)
- Model routing: `LlmConfig` declaration with provider failover (Наряд 21)
- Context compression for long conversations (Наряд 21)
- Tool abstraction: `Tool` declaration with typed methods (Наряд 22)
- `Hook` declaration: before/after pattern hooks (Наряд 22)
- Session memory: `session_set`/`session_get`/`session_clear` builtins (Наряд 22)

### Infrastructure

- 32 integration test files (7 000+ lines of tests)
- 63 Architecture Decision Records
- CI pipeline: build + release binary (Linux x86_64)

---

## [0.7.5] — 2026-06-13

**Phase 7.5–7.6: Memory persistence, tokens, eval harness, session memory, audit.**

- Memory persistence e2e tests (JSON file-based storage)
- JWT-style token generation and verification
- Eval harness for testing learnable patterns with golden-file assertions
- `session_set`/`session_get`/`session_clear` session memory builtins
- Audit parser integration tests
- Server JSON body parsing for POST routes

---

## [0.7.3] — 2026-06-12

**Phase 7.3–7.4: Context compression, lifecycle, tool abstraction, hooks, DoD.**

- Context compression for long conversations
- Lifecycle control for flows and patterns
- Tool abstraction (`Tool` declaration)
- `Hook` declaration for before/after pattern execution
- Definition of Done framework with automated checks

---

## [0.7.1] — 2026-06-10

**Phase 7.1–7.2: Inspect, context loading, events, conversation state, LLM cache, model routing.**

- `inspect()` builtin for safe value introspection
- Context loading from entity/memory/fluid declarations
- Event streaming (`emit`/`on`)
- `Conversation` declaration with TTL and message limits
- LLM response cache with configurable TTL
- `LlmConfig` declaration for multi-provider model routing

---

## [0.6.0] — 2025-06-03

**Phase 6: Full-stack web platform with security by design.**

### Security — 6 levels, OWASP Top 10 closed

- **Type-safe HTML templates** — `template` construct returns opaque `Html` type, auto-escaping prevents XSS
- **Parameterized database queries** — `query(sql_literal, params)`, opaque `Query` type, SQL injection syntactically impossible
- **Encryption primitives** — `Secret`, `Encrypted`, `Hash` opaque types; `env()` maps to `Secret`; `encrypt`/`decrypt` via AES-256-GCM; `hash_password`/`verify_password`
- **Authentication & authorization** — session management (HMAC-SHA256 signed cookies), role-based access (`requires=[role]`), `require` assertions, `authenticate`/`session_login`/`session_logout`
- **CSRF & security headers** — double-submit token pattern, CSP/HSTS/X-Frame-Options/X-Content-Type-Options middleware
- **LLM sandbox** — sandboxed execution for learnable patterns, no direct HTML injection from AI responses

### Web platform

- **HTTP server** — `mlogserver` block with `port`, `middleware`, `route` declarations (Axum 0.8 + Tokio)
- **Routing** — `route "/path" method=GET/POST requires=[roles] { handler }`
- **Request parsing** — `form_data()`, `json_body()` built-in functions
- **Response** — `respond(status)`, `render(template, args)` for HTML output
- **Bot integration** — Telegram/Discord webhook routes, `send_message(chat_id, text)` outbound HTTP
- **CLI** — `mlog serve <file>` starts the HTTP server

### Language additions

- **`db` block** — database configuration with `pool_size` and `migrate`
- **`template` construct** — type-safe HTML templates with `{{ var }}` auto-escaping
- **`require` statement** — runtime assertion for authorization checks
- **40+ built-in functions** across string, math, web, crypto, auth, and bot domains

### Examples

- `p6_full_app.mlog` — 170-line full-stack application demonstrating all 6 security levels

---

## [0.5.0] — Phase 5: Language completeness

**Control flow, collections, string operations, modules, bytecode VM, JIT.**

- `let` bindings with `if/else` expressions
- `each item in list { ... }` and `while cond { ... }` loops
- `break` and `continue` in loops
- `match` expression with `exact`, `starts_with`, `contains`, `compare` arms
- List literals `[1.0, 2.0, 3.0]` with `get`, `push`, `len`, `first`, `last`, `reverse`
- String operations: `index_of`, `substring`, `char_at`, `starts_with`, `ends_with`, `contains`, `split`, `join`, `trim`, `replace`
- Module system: `import std/string as str` with qualified calls (`str.trim(s)`)
- Bytecode VM: 44 instructions, stack-based execution
- JIT compiler via Cranelift
- Self-hosted lexer
- REPL integration tests, semantic check integration tests

---

## [0.8.x] — archived (not formalized in this CHANGELOG)

Versions 0.8.0 through 0.8.9 were released informally (no git tags) and
their highlights were not captured in this CHANGELOG at the time. The
README.md version-highlights table still references them; the entries
there are the best summary available without reconstructing from
individual commits.

If you need the precise per-commit history for 0.8.x:

```bash
git log --oneline --grep="0\.8\." --reverse
```

Future naryads may formalize 0.8.x sections here by extracting
highlights from the actual commit history (Наряд №166 Block 2 noted
this gap; recovery requires verifying each highlight against the
real commit, not invented descriptions — see ADR-0110 §2
"contract before code").

---

## [0.3.0] — Phases 1–4: Core language, types, ML, ecosystem

**Probabilistic types, ML backend, knowledge graph, vector recall, LSP, packages.**

- **Phase 1**: Fluid types with probabilistic superposition, confidence propagation, entity store queries (`find()`)
- **Phase 2**: Knowledge graph (`relate`), vector recall (semantic memory), full adapt system (sandbox/mutate/rollback), ML learn statement
- **Phase 3**: CLI (`mlog run/repl/check`), LSP server, `mlogpkg` package manager, mdbook documentation
- **Phase 4**: Bytecode VM, JIT compiler (Cranelift), self-hosted lexer, IR generation

---

## [0.1.0] — M1–M5: Seven pillars, basic interpreter

**The foundation — AI-native language with seven semantic primitives.**

- **M1**: Entity (simple, struct, instance), pure pattern, linear flow, built-in functions (`upper`, `lower`, `len`, etc.)
- **M2**: Struct entities, rule engine with priority and confidence-based flow branching
- **M3**: Learnable patterns (LLM backend trait + mock), prompt engineering, few-shot caching, `adapt` statement
- **M4**: Semantic memory (`memorize`/`recall`/`forget`), knowledge graph (`relate`), memory decay
- **M5**: Sandbox execution, `mutate` with rollback on degradation
- Pest PEG grammar, hand-written AST, tree-walking interpreter
- Golden-file test framework (`examples/*.expected`)