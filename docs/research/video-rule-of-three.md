# Rule of Three — General Denoiser Assessment (Naryad #308, issue #388)

> **Date:** 2026-09-14. **Base:** main `8768de2`. **Status:** facts recorded, verdict deferred to Voice-A2.

## 1. Context

`euler_step` is a shared ODE primitive in `src/vision/sampler.rs`. The full sampling contour (`flow_match_euler_sample`) is tied to `ZImageTransformer` (Vision). Video is the third concrete consumer of the ODE primitive. Rule of three: extraction of a shared `Denoiser` interface is considered after the three concrete contours are proven.

## 2. Three contours (current state)

| Pillar | ODE primitive | Denoiser | Sampling | Status |
|---|---|---|---|---|
| Vision | `euler_step` (shared) | `ZImageTransformer` (vision-specific) | `flow_match_euler_sample` (vision-specific) | ✅ Real (tiny golden, CI green) |
| Voice | `euler_step` (shared, planned) | TBD (Voice-A2 not published) | TBD | ❌ Not started |
| Video | `euler_step` (shared, planned) | `VideoDit` (stub, #308) | `flow_match_euler_sample_video` (stub, #308) | ⏳ Skeleton |

## 3. Reusable code share

| Component | Vision | Video (stub) | Reuse |
|---|---|---|---|
| `euler_step` | ✅ | ✅ (planned) | **100%** — identical ODE step |
| Flow matching loop | Vision-specific (`ZImageTransformer`) | Video-specific (`VideoDit`) | **~30%** — the loop structure is the same, the model differs |
| VAE | `VisionVae` (2D conv) | `VideoVae` (3D conv, stub) | **~10%** — 2D vs 3D, different architecture |
| Attention | 2D spatial | 3D spatiotemporal + causal | **~20%** — shared pattern, different implementation |
| Positional encoding | 2D RoPE | 3D RoPE (temporal + spatial) | **~40%** — shared principle, different dimensionality |

**Common denominator**: only `euler_step` (the ODE primitive) — 100% reuse. Everything else — 10-40% — is too little for extracting a shared interface without leaking specifics.

## 4. Video-specific leakage

Extracting a shared `Denoiser` interface would require:
- a temporal axis (Video: 3D, Vision: 2D) → the shared interface must support N-dimensional inputs
- causal attention (Video: frame i attends to 0..=i; Vision: none) → a causal flag in the interface
- 3D positional encoding → the shared interface must support both 2D and 3D

**Assessment**: the shared interface would be either too generic (dyn Any, loss of typing) or polluted with video specifics (temporal/causal flags in Vision code). Neither option is acceptable.

## 5. Expressibility of MoE denoisers

Wan 2.2 A14B is a MoE architecture (mixture of experts). A shared `Denoiser` interface must:
- support dense and MoE models
- not break when expert-routing is added
- not require Vision/Voice to know about MoE

**Assessment**: MoE can be expressed via a trait object (`dyn Denoiser`), but that is a loss of typing + overhead. Before a real MoE model exists in Voice/Video — premature.

## 6. Verdict

**Deferred to Voice-A2.**

Facts:
- `euler_step` — 100% reuse (already shared).
- The full contour — 10-40% reuse — is below the extraction threshold.
- Video-specific leakage (temporal, causal, 3D) — unacceptable.
- MoE expressibility — premature.

Completion protocol:
1. Voice-A2 publishes its denoiser → the three contours are proven.
2. If Voice-A2 shows ≥50% reuse with Vision/Video → an ADR for the `Denoiser` trait.
3. If <50% → status quo: `euler_step` shared, contours separate.

## 7. Engineering Go/No-Go

**Engineering Go** — a contour on tiny weights, contracts, CI (this naryad, #308):
- VAE contract: 3D conv encode/decode, latent shape [B, 16, T/4, H/8, W/8].
- DiT contract: noisy latent + timestep + text → clean latent.
- Sampler: euler_step (shared) + VideoDit (stub).
- CI: skeleton tests green, no real weights needed.

**Quality Go** — RTF/quality on real weights — a separate naryad V4, following the #237 preflight runbook pattern.

Current status: **engineering Go** (skeleton + contracts). **Quality No-Go** (no hardware, #294).
