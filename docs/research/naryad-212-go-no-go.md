# Наряд №212 — Go/No-Go Report (Vision R3 e2e)

**Date:** 2026-09-08
**Author:** Agent executing naryad №212
**Decision authority:** Coordinator (per §0 of naryad spec — "Решение Go/No-Go принимает координатор, не исполнитель")

## Status

**Code-complete, env-gated run PENDING.** All CI-visible tests pass (5/5). Env-gated real-weights tests are coded and skip loudly when `MLOG_VISION_WEIGHTS_DIR` is unset; they have NOT been executed with real weights in this delivery environment.

## What's verified (CI-visible, bit-exact)

| Component | Test | Result |
|-----------|------|--------|
| VAE decoder (tiny) | `vae_tiny_decode_golden` | ✅ pinned hash + 4 anchor bits, 3 bit-identical runs |
| VAE decoder (tiny) | `vae_tiny_decode_determinism` | ✅ same seed → bit-exact identical |
| DiT (tiny) | `dit_tiny_forward_golden` | ✅ pinned (№231, hash=e686167b2e82ee7be9fe3408ed9e619953e774d49e224310f2d0541af3c10257) |
| Sampler | `sampler_sigmas_pinned` | ✅ 9 sigmas pinned: [1.0, 0.955, 0.900, 0.834, 0.751, 0.644, 0.501, 0.302, 0.003] |
| Sampler | `sigmas_monotonic_decreasing` | ✅ |
| Sampler | `sigmas_first_close_to_one` | ✅ |
| Sampler | `sigmas_last_small_positive` | ✅ |
| Sampler | `sigmas_deterministic` | ✅ |
| R2 contract | `naryad_211` golden (6 tests) | ✅ unchanged (regression check) |
| Weights infra | `weights.rs` unit tests (4) | ✅ manifest load, SHA mismatch, missing index, missing tokenizer |
| Tokenizer | `tokenizer.rs` unit test (1) | ✅ missing tokenizer.json errors loudly |

> hash=e686167b в таблице выше — снимок эпохи №212; после rebuild-цепочки фактический пин — 860c85b3 (SSOT = `GOLDEN_DIT_TINY_HASH` в тесте).

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

- №236 (PR #234): generators single-source, D2' closed, 15/15
- №237 (PR #235): real-weights run prep — fetch tool (`tools/fetch_vision_weights.sh`, sha256 discipline) + manifest HF-oid section + runbook `naryad-237-real-weights-runbook.md`; real-weights run **PARKED** (owner decision 2026-09-09 — no ≥40 GB machine in delivery env, 9.2 GB free). Size truth-up: 16 files = 32 848 304 654 B ≈ **32.85 GB** verified via HF models API — the "~24.6 GB" estimate above was an underestimate of the same source. Tokenizer 4-row SHAs filled with real (double-run-verified) values; heavy weights remain _TODO_ until the run.
- №238 (PR #236): R4.1 vision{}-декларации (grammar+AST+parser+semantic, 12 тестов), Block 0 = fix-forward №237 (runbook golden `860c85b3`, TE 8.05 GB); пере-скоуп изданного docs-only наряда объявлен в PR, 4 хвоста — в №239.
- №240 (PR #238): R4.2 dispatch — `Program::vision_decls` (лекало reflex), VM/interpreter регистрация, `VisionRegistry` реальный артефакт (PNG-буфер), `vision_generate(decl, prompt)` 2-арг реальный клин (громкий отказ без weights), `vision_list`/`vision_export` реальные, арность 3→2 (урок №234), taint UserInput→prompt = audit-warning; env-gated `.mlog` e2e (loud-SKIP) закрывает обещание №237 Block 3.1.
- №241 (PR #239): R5 security (ADR-0125) — гейты категории A: `VISION_UNSIGNED_EXPORT` (audit Error + runtime backstop), `MODEL_WEIGHTS_UNSAFE` (audit Error; runtime — `vision_fetch_weights`: allowlist default-deny `MLOG_VISION_WEIGHTS_ALLOWLIST`, SSRF-guard `check_url_ssrf` с пиннингом резолвов, manifest.json-class only, SHA-256 pinning через переиспользованный `WeightsManifest`), `VISION_POLICY_MISSING` (audit Warning; parser relax policy-only, остальные 6 полей required); Provenance MVP: LSB-watermark (MLGV + model-hash32, RGB LSB) в каждом `vision_generate`, manifest JSON (model-id+weights-SHA/`unpinned`, seed, prompt-hash, policy/`unspecified`, timestamp, SHA итогового PNG) + sidecar на `vision_export`; `vision_export_raw` — явный opt-out (audit Warning `VISION_UNSIGNED_EXPORT_RAW`); реестр 387→389. Тесты: 4 контракта категории A = 3 новых (naryad_241_vision_gates) + taint-тест №240; watermark roundtrip — unit-тесты provenance (vision-tests job). Без сети и весов.
- №242 (PR #240): R6.1 SQLite-персистенция артефактов (первая треть R6) — `src/vision/store.rs` (таблица `vision_artifacts`, дословный manifest-roundtrip с неперегенерируемым timestamp, битый JSON = громкий Err, коллизия имени = громкий Err), перехваты `vision_save`/`vision_load` в interpreter (eval+invoke) и VM (+db_conn программы, no-db = громкий Err с подсказкой `db { url: ... }`), last-resort стабы truth-up (№242 вместо устаревшей «214/215»); roundtrip-контракт: PNG + sidecar байт-в-байт после save→load в новый рег, backstop `VISION_UNSIGNED_EXPORT` жив после персистенции; реестр 389 не меняется. Без сети и весов.
