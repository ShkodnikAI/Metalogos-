# Naryad #212 — Go/No-Go Report (Vision R3 e2e)

**Date:** 2026-09-08
**Author:** Agent executing naryad #212
**Decision authority:** Coordinator (per §0 of naryad spec — "the Go/No-Go decision is made by the coordinator, not the executor")

**Owner gate (2026-09-09, Vision plan §6 — synchronized by naryad #246):** conditional GO on code with the real-weights run PARKED (runbook #237). An assumption is an assumption, not verification: production claims about image generation and the start of a new pillar (Voice) are blocked until the real-run line below is filled in (PNG path, SHA-256, timings, hardware).

## Status

**Code-complete, env-gated run PENDING.** All CI-visible tests pass (5/5). Env-gated real-weights tests are coded and skip loudly when `MLOG_VISION_WEIGHTS_DIR` is unset; they have NOT been executed with real weights in this delivery environment.

## What's verified (CI-visible, bit-exact)

| Component | Test | Result |
|-----------|------|--------|
| VAE decoder (tiny) | `vae_tiny_decode_golden` | ✅ pinned hash + 4 anchor bits, 3 bit-identical runs |
| VAE decoder (tiny) | `vae_tiny_decode_determinism` | ✅ same seed → bit-exact identical |
| DiT (tiny) | `dit_tiny_forward_golden` | ✅ pinned (#231, hash=e686167b2e82ee7be9fe3408ed9e619953e774d49e224310f2d0541af3c10257) |
| Sampler | `sampler_sigmas_pinned` | ✅ 9 sigmas pinned: [1.0, 0.955, 0.900, 0.834, 0.751, 0.644, 0.501, 0.302, 0.003] |
| Sampler | `sigmas_monotonic_decreasing` | ✅ |
| Sampler | `sigmas_first_close_to_one` | ✅ |
| Sampler | `sigmas_last_small_positive` | ✅ |
| Sampler | `sigmas_deterministic` | ✅ |
| R2 contract | `naryad_211` golden (6 tests) | ✅ unchanged (regression check) |
| Weights infra | `weights.rs` unit tests (4) | ✅ manifest load, SHA mismatch, missing index, missing tokenizer |
| Tokenizer | `tokenizer.rs` unit test (1) | ✅ missing tokenizer.json errors loudly |

> hash=e686167b in the table above — a snapshot of the #212 epoch; after the rebuild chain the actual pin is 860c85b3 (SSOT = `GOLDEN_DIT_TINY_HASH` in the test).

## What's NOT verified (env-gated, requires real weights)

These tests exist and compile but require `MLOG_VISION_WEIGHTS_DIR` to point at a downloaded Z-Image-Turbo weights directory (~24.6 GB total). They were not run in this delivery environment.

| Test | What it would verify | Status |
|------|----------------------|--------|
| `text_encoder_real_weights_forward` | Real Qwen3-4B forward pass (3 shards, ~7.5 GB) → `[seq, 2560]` hidden states | Coded, SKIP |
| `vae_real_weights_decode_fixed_latent` | Real flux-dev VAE decode (167 MB) → PNG 1024×1024 | Coded, SKIP |
| `clinical_e2e_first_image` | Full pipeline: prompt → tokens → Qwen3 → DiT 8 forward → VAE → PNG 1024×1024 | Coded, SKIP |

## Hardware (this delivery environment)

- Linux x86_64, CPU-only (no GPU)
- ~10 GB available disk space (insufficient for ~24.6 GB weights download)
- ~32 GB RAM (insufficient for F32 weight load — see dtype policy in research doc §8)

## Go/No-Go criteria (per naryad spec)

The spec defines Go/No-Go criteria in Block 5.2:
- Latency on CPU
- Necessity of GPU
- Quality acceptability (subjective — requires actual generated image)

These criteria cannot be evaluated without running the env-gated tests with real weights.

## Honest assessment

**The code is structurally complete** — all components compile, the architecture follows the diffusers reference (verified by direct HF config fetch in Block 0), and the CI-visible tiny goldens confirm the deterministic seeded-init path works bit-exactly.

**The real-weights run is a separate execution step** that requires:
1. Downloading ~24.6 GB of weights via `huggingface-cli download Tongyi-MAI/Z-Image-Turbo` (Block 0 §0.2 — explicitly the executor's manual step, NOT automated in R3).
2. Populating `docs/research/naryad-212-weights-manifest.md` with the actual SHA-256 values.
3. Setting `MLOG_VISION_WEIGHTS_DIR` and running `cargo test --features vision --test naryad_212_wedge_e2e -- --nocapture`.
4. Filling in the verbatim DoD entries (PNG path, PNG SHA-256, timings).

**R3 architecture status** (n234: all 12 discrepancies resolved, mid-attn placement fixed; n235: VAE decoder structure truth-up):
- VAE mid-block attention: loaded and computed in decode path when mid_block_add_attention=true.
  Placement: resnets[0] → attention → resnets[1] per UNetMidBlock2D.forward (n234 fix).
  Tiny config (mid_block_add_attention=false) does not use it.
- VAE decoder structure: 3 resnets per block (lpb+1), conv_norm_out → SiLU → conv_out, shortcuts on channel changes — all per real header (n235).
- Axial RoPE: fully wired — rope.apply(q, k) called in all attention paths after qk-norm.
  Clamp replaced with loud Err on out-of-range pos_ids.
- cap pos_ids: per-token (i+1, 0, 0) per create_coordinate_grid source.
- Loader guard: key-level check_tensor_coverage in all from_weights (n234 truth-up).
- Loader guard: all three expected-key generators (VAE/DiT/TE) extracted to standalone functions and called by both from_weights and unit tests — single source of truth. D2' (generator produced 146≠138 due to in_ch placement outside resnet loop; test-copy masked it) caught by coordinator verification before real-weights run. Guard would have failed loudly (Missing 8) on real weights — now fixed to 138.
- Status: architecture = reference by 12/12 points. Real-run awaits coordinator deployment.

## Recommendation to coordinator

**Conditional Go pending real-weights run.** The architecture is in place; the missing piece is the actual e2e execution with real weights on appropriate hardware (likely a 64+ GB RAM machine or GPU box). The coordinator should:

1. Designate an executor with access to download the weights and run the env-gated tests.
2. Verify the PNG output is a recognizable image (subjective quality check — Block 5.2).
3. Reconcile timings against the dtype policy (F32 → ~62 GB peak RAM, may need BF16 path — R4+ territory).

If the env-gated run produces a coherent image at acceptable latency, **Go for R4** (vision grammar + dispatch). If the image is corrupted or latency is unacceptable, **No-Go** with a targeted fix-forward PR addressing the R3 simplifications listed above.

## Verbatim DoD entries (env-gated section — left blank, requires real run)

```
- PNG path: <REQUIRES REAL RUN — see docs/research/naryad-212-weights-manifest.md>
- PNG SHA-256: <REQUIRES REAL RUN>
- PNG size: <REQUIRES REAL RUN>
- Timings (tokenize / encode / sampler / decode): <REQUIRES REAL RUN>
- Determinism (2 runs bit-exact): <REQUIRES REAL RUN>
- Hardware: <REQUIRES REAL RUN>
```

These entries are intentionally left blank rather than fabricated — per §3.8 of the naryad spec, faking the "first frame" is an explicit failure mode of the naryad. The loud-skip pattern of the env-gated tests is the §3-sanctioned form of permitted unfinishedness.

## Verification log (doc-sync from later naryads)

- #236 (PR #234): generators single-source, D2' closed, 15/15
- #237 (PR #235): real-weights run prep — fetch tool (`tools/fetch_vision_weights.sh`, sha256 discipline) + manifest HF-oid section + runbook `naryad-237-real-weights-runbook.md`; real-weights run **PARKED** (owner decision 2026-09-09 — no ≥40 GB machine in delivery env, 9.2 GB free). Size truth-up: 16 files = 32 848 304 654 B ≈ **32.85 GB** verified via HF models API — the "~24.6 GB" estimate above was an underestimate of the same source. Tokenizer 4-row SHAs filled with real (double-run-verified) values; heavy weights remain _TODO_ until the run.
- #238 (PR #236): R4.1 vision{} declarations (grammar+AST+parser+semantic, 12 tests), Block 0 = fix-forward of #237 (runbook golden `860c85b3`, TE 8.05 GB); re-scope of the issued docs-only naryad announced in the PR, 4 tails — in #239.
- #240 (PR #238): R4.2 dispatch — `Program::vision_decls` (stencil reflex), VM/interpreter registration, `VisionRegistry` a real artifact (PNG buffer), `vision_generate(decl, prompt)` a real 2-arg wedge (loud refusal without weights), `vision_list`/`vision_export` real, arity 3→2 (the #234 lesson), taint UserInput→prompt = audit-warning; env-gated `.mlog` e2e (loud-SKIP) closes the #237 Block 3.1 promise.
- #241 (PR #239): R5 security (ADR-0125) — category-A gates: `VISION_UNSIGNED_EXPORT` (audit Error + runtime backstop), `MODEL_WEIGHTS_UNSAFE` (audit Error; runtime — `vision_fetch_weights`: default-deny allowlist `MLOG_VISION_WEIGHTS_ALLOWLIST`, SSRF-guard `check_url_ssrf` with resolve pinning, manifest.json-class only, SHA-256 pinning via the reused `WeightsManifest`), `VISION_POLICY_MISSING` (audit Warning; parser relax policy-only, the remaining 6 fields required); Provenance MVP: LSB-watermark (MLGV + model-hash32, RGB LSB) in every `vision_generate`, manifest JSON (model-id+weights-SHA/`unpinned`, seed, prompt-hash, policy/`unspecified`, timestamp, SHA of the final PNG) + sidecar on `vision_export`; `vision_export_raw` — explicit opt-out (audit Warning `VISION_UNSIGNED_EXPORT_RAW`); registry 387→389. Tests: 4 category-A contracts = 3 new (naryad_241_vision_gates) + the #240 taint test; watermark roundtrip — provenance unit tests (vision-tests job). No network, no weights.
- #242 (PR #240): R6.1 SQLite persistence of artifacts (first third of R6) — `src/vision/store.rs` (the `vision_artifacts` table, verbatim manifest-roundtrip with a non-regenerable timestamp, broken JSON = loud Err, name collision = loud Err), `vision_save`/`vision_load` intercepts in the interpreter (eval+invoke) and VM (+ the program db_conn, no-db = loud Err with the `db { url: ... }` hint), last-resort stubs truth-up (#242 instead of the outdated "214/215"); roundtrip contract: PNG + sidecar byte-for-byte after save→load into a fresh register, the `VISION_UNSIGNED_EXPORT` backstop alive after persistence; registry 389 unchanged. No network, no weights.
- #243 (PR #241): R6.2 vision_edit — in-context editing (second third of R6). `VaeEncoder` in `src/vision/vae.rs` (the non-decoder prefixes of THE SAME pinned VAE file: 244 = 138 decoder + 106 encoder — quant_conv optional, absence/presence = a loud note, the actual list is verified against the header at the PARKED run; new_tiny for the tiny contracts; encode in posterior MODE = determinism); the in-context edit loop `flow_match_euler_edit` (EDIT_STEPS=8 — the distilled-NFE turbo, a loud constant; `forward_edit` in dit.rs — additive: reference tokens through the same x_embedder + noise_refiner, its own RoPE t-slot cap_len+2, euler_step on the noise branch only; no CFG — turbo); **wedge goldens bit-exact without edits** (85ef6a87/860c85b3 — documented in the PR). Dispatch `vision_edit(handle, prompt)`: the source MUST be signed (manifest: None = loud Err BEFORE the env check; unsigned remains usable in vision_export_raw), inheritance of model_id/policy/seed + fresh timestamp/prompt_sha256/png_sha256 (sign ALWAYS), dims contract (R4.1 256..=4096 ×16 + the VAE factor — loud, silent resize forbidden), taint arg-1 → VISION_PROMPT_USER_INPUT (same check-id, no new ones). Interpreter (eval+invoke) and VM intercepts; last-resort stub truth-up (the last "214/215" left the repo); naryad_210 truth-up (10→11 tests). Runbook §3.1 — edit-e2e for the PARKED run. No network, no weights.
- #244 (PR #242): R6.3 LoRA adapters — the final third of R6 (#215 closed entirely by #242–244). `src/vision/lora.rs` (both canonical name forms of diffusers-PEFT/ComfyUI + alpha; rank = the average dimension, scale = alpha/rank / loud default 1.0; loud F32 upcast; targets — attention projections only, per `zimage_expected_keys`; all negatives = loud Err with the full list); `from_weights_with_lora`/`merge_lora_in_place` (W' = W + scale·(up@down) in F32, deterministic order, missing base / exceeding n_layers = loud Err; zero-adapter identity byte-for-byte — wedge goldens 85ef6a87/860c85b3 green without edits); the `vision_lora_adapters` table (SQLite BLOB — the only home of an adapter, ADR-0124 §6); `vision_lora_load` (reads only inside weights_dir, the prescribed order of loud checks) + `vision_lora_generate` (integrity sha on every resolve from the DB, composite provenance `sha256("{base}\nlora:{name}:{sha}")` — honest even over "unpinned", watermark = base model, sign ALWAYS); intercepts ×3, taint arg-1 (same check-id), stub stencils. **Registry 389→391 — the vision family has reached the ADR-0124 §3 ceiling (10/10): the next vision builtin requires an ADR-0124 amendment** (loud in the CHANGELOG). Runbook §3.2 — lora-e2e for the PARKED run (the adapter is placed by the owner under weights_dir/lora/…). Tests: 21 tiny contracts of naryad_244 + 3 taint (naryad_240) + 2 stub groups (naryad_210, 11→13) + 4 merge units (dit.rs/lora.rs); the blob is built by the candle writer (the API is available — no deviations). No network, no weights.
