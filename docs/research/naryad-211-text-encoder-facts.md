# Naryad #211 — Research: Text Encoder Facts (Qwen3-4B)

**Date:** 2026-09-07
**Status:** Fact-check resolved (ADR-0123 item 1)

## 1. Encoder identification

Z-Image / Z-Image-Turbo (Alibaba) uses **Qwen3-4B** as its text
encoder. It is a pure text decoder-only LLM, NOT a vision-language model.

**Sources (3 independent):**

1. HF `Tongyi-MAI/Z-Image-Turbo` discussion #4 — "why they use Qwen3-4B
   pure text model" (developer's answer).
2. `github.com/fblissjr/ComfyUI-QwenImageWanBridge` → `nodes/docs/z_image_encoder.md`:
   "Z-Image is Alibaba's 6B parameter text-to-image model using Qwen3-4B
   as its text encoder".
3. mindstudio.ai / z-image.vip / docs.imagine.art — consistent
   secondary sources confirming Qwen3-4B.

## 2. config.json of Qwen/Qwen3-4B (exact values)

| Parameter | Value |
|---|---|
| `num_hidden_layers` | 36 |
| `hidden_size` | 2560 |
| `num_attention_heads` | 32 |
| `num_key_value_heads` | 8 |
| `head_dim` | 128 |
| `intermediate_size` | 9728 |
| `vocab_size` | 151936 |
| `rms_norm_eps` | 1e-6 |
| `rope_theta` | 1000000 |
| `max_position_embeddings` | 40960 |
| `attention_bias` | false |
| `hidden_act` | silu (SwiGLU) |
| `tie_word_embeddings` | true |

Source: `https://huggingface.co/Qwen/Qwen3-4B/raw/main/config.json`
(only config.json was downloaded, no weights — ADR-0123/0124).

## 3. Hidden states — what Z-Image consumes

The research shows conflicting data on which hidden states
Z-Image consumes from Qwen3-4B:

- **Version A (final layer):** most secondary sources and
  the ComfyUI-QwenImageWanBridge documentation indicate that Z-Image takes
  the final hidden states of the last layer (last_hidden_state) as
  the prompt embedding. This is the standard pattern for text-to-image: the encoder
  runs in full, the LM head is discarded, and `[seq_len, hidden]` is taken.

- **Version B (per-layer):** some diffusion-model implementations
  consume hidden states from several layers (deep conditioning).
  No confirmation of this pattern was found for Z-Image.

**Decision for R2:** the golden contract uses the final layer
(Version A) — `forward()` returns `[seq_len, hidden]` of the last
layer. If R3 shows that multi-layer is needed, this changes only
the loading interface, not the block architecture.

## 4. Dtype policy

**Decision:** F32 for all computations in the R2 golden contract.

**Rationale:**
- the candle CPU device supports F32 natively, without conversion;
- BF16 weight storage is R3 (loading safetensors with BF16),
  where `to_dtype(F32)` before computation is the standard candle pattern;
- the R2 golden contract runs in pure F32 — determinism is maximal;
- when loading real weights (R3) the dtype is converted at input,
  forward stays F32 (accumulation in F32, output in F32).

## 5. Architectural summary

Qwen3-4B maps exactly onto the `src/nn/` zoo:
- GQA: `Attention::new_with_kv_heads(heads, n_kv_heads, dim, seed, var_map, prefix)`
  (src/nn/attention.rs:129) — 32 Q-heads / 8 KV-heads, head_dim=128
- RmsNorm: `RmsNorm::with_weights(dim, weights, eps)` (src/nn/rmsnorm.rs:61)
- SwiGLU: `SwiGlu::new(dim, ff_dim, seed)` (src/nn/swiglu.rs:83)
- Deterministic initialization: `generate_uniform_f32(seed, n, lo, up)`
  (src/nn/attention.rs:508, xorshift64)

**Differences from existing blocks in src/nn/:**
- **RoPE:** not in `src/nn/` (only in `trainable_attention.rs` for
  trainable models, with a different interface). R2 implements RoPE inside
  `src/vision/text_encoder.rs`.
- **QK-norm:** RmsNorm over `head_dim`, applied per head separately for Q and K.
  Not covered by the existing `TransformerBlock` in `src/nn/`.
- **Causal mask:** standard lower-triangular, candle `tril`.

Conclusion: R2 assembles `Qwen3Block` from candle primitives inside
`src/vision/text_encoder.rs`, WITHOUT modifying `src/nn/*`.

## 6. Correction (fix-forward, 2026-09-08)

The original version of this document (delivered by PR #223) contained
**fabricated config.json values**: `num_attention_heads = 40`,
`head_dim = 64`, `intermediate_size = 6912`, `max_position_embeddings = 32768`
alongside the claim "config.json downloaded". The fact-check by the naryad
coordinator (direct fetch of `https://huggingface.co/Qwen/Qwen3-4B/raw/main/config.json`,
2026-09-08) produced: **32** attention heads (40 is Qwen3-14B), **head_dim 128**,
**intermediate 9728**, **max_position 40960**. The table above and
`QWEN3_4B_CONFIG` in `src/vision/text_encoder.rs` were corrected to the real
values; the constant test `qwen3_4b_config_matches_pinned_values` is anchored
to them.

Additionally, execution defects of #211 closed by naryad #230 were recorded:

1. ~~Golden SHA-256 records not pinned (Block 2.4 not done) — the test checks
   only internal determinism, not bit-for-bit correspondence to fixed
   records.~~ **CLOSED (#230, PR #225)**: `GOLDEN_HASH_P1/P2/P3`
   + `GOLDEN_ANCHOR_BITS_P1/P2/P3` (4 corner `f32::to_bits()` values per prompt —
   integer-exact, immunity to print-drift) are baked into
   `tests/naryad_211_text_encoder_golden.rs`. Test 1 asserts the hash + anchors
   bit-for-bit. Pinned based on 3 identical local runs (2026-09-08).
2. ~~Local copy of `generate_uniform_f32` diverges from the SSOT contract of
   `src/nn/attention.rs` (no `seed_to_state` ritual, a different f32 vs f64 mapping
   path → different value streams for the same seed).~~ **CLOSED (#230)**:
   the local copy was removed; import `use crate::nn::attention::generate_uniform_f32;`
   (SSOT). For the import to work correctly, the feature-flag implication was fixed:
   `vision = ["candle"]` (previously `["dep:candle-core", "dep:candle-nn"]` — without
   enabling the `candle` feature flag itself, so the `#[cfg(feature = "candle")]`
   modules of `src/nn/` did not compile under `--features vision`).
3. ~~Seed-stream overlap: `layer_seed + offset` shares offsets between layers
   (layer i's k ≡ layer i+1's q, etc.) — weights are correlated across layers.~~
   **CLOSED (#230)**: `param_seed(master, layer, param)` — a splitmix64 finalizer
   over `(master_seed, layer, param)`. `PARAM_EMBEDDING=0` through `PARAM_DOWN=7`
   are fixed constants (do not renumber: the derivation is part of the golden
   contract). The test `param_seed_derivation_is_pairwise_distinct` checks that
   32 seeds (4 layers × 8 parameters) are pairwise distinct + non-identity.

The order in #230 was followed: first the PRNG SSOT + stream hygiene (Block 1, changes
all values), then pinning of the golden records (Block 2, once, against the final
values after 3 bit-for-bit runs).
