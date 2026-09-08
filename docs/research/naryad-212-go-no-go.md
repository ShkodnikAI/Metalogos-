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
| DiT (tiny) | `dit_tiny_forward_shape` | ✅ compiles + API callable (placeholder — full pinning needs deeper work) |
| Sampler | `sampler_sigmas_pinned` | ✅ 9 sigmas pinned: [1.0, 0.955, 0.900, 0.834, 0.751, 0.644, 0.501, 0.302, 0.003] |
| Sampler | `sigmas_monotonic_decreasing` | ✅ |
| Sampler | `sigmas_first_close_to_one` | ✅ |
| Sampler | `sigmas_last_small_positive` | ✅ |
| Sampler | `sigmas_deterministic` | ✅ |
| R2 contract | `naryad_211` golden (6 tests) | ✅ unchanged (regression check) |
| Weights infra | `weights.rs` unit tests (4) | ✅ manifest load, SHA mismatch, missing index, missing tokenizer |
| Tokenizer | `tokenizer.rs` unit test (1) | ✅ missing tokenizer.json errors loudly |

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

**Known R3 simplifications** (documented in code, would need address before Go decision):
- DiT mid-attention block weights loading is stubbed (`mid_attn = None`) — real VAE has attention but loading is complex; tiny CI uses `mid_block_add_attention=false`.
- DiT axial RoPE: structure present, but the exact 3-axis rotation (32/48/48 split) needs deeper verification against diffusers source. R3 uses the basic RoPE pattern from R2 text encoder.
- DiT `attn_norm1_placeholder()` returns ones for the final layer norm weight — real model has a learned weight (`all_final_layer.2-1.linear.weight` preceding adaLN). This is a simplification.
- These would be addressed in a fix-forward PR (R3.1) if Go/No-Go requires bit-exactness against diffusers.

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
