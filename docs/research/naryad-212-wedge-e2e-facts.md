# Наряд №212 — Wedge E2E Fact Sheet

**Pinned:** 2026-09-08, direct HF fetch of `Tongyi-MAI/Z-Image-Turbo` config files.
**Purpose:** SSOT for Block 1–4 (tensor map, dtype policy, mechanics).

## 1. Pipeline (model_index.json)

URL: `https://huggingface.co/Tongyi-MAI/Z-Image-Turbo/raw/main/model_index.json`

```json
{
  "_class_name": "ZImagePipeline",
  "_diffusers_version": "0.36.0.dev0",
  "scheduler":     ["diffusers", "FlowMatchEulerDiscreteScheduler"],
  "text_encoder":  ["transformers", "Qwen3Model"],
  "tokenizer":     ["transformers", "Qwen2Tokenizer"],
  "transformer":   ["diffusers", "ZImageTransformer2DModel"],
  "vae":           ["diffusers", "AutoencoderKL"]
}
```

Pipeline: `prompt → tokenizer → Qwen3Model → cap_embedder → DiT(8×flow steps) → VAE.decode → image`.

## 2. Transformer (transformer/config.json)

URL: `https://huggingface.co/Tongyi-MAI/Z-Image-Turbo/raw/main/transformer/config.json`

| field            | value |
|------------------|-------|
| `dim`            | 3840 |
| `n_layers`       | 30 |
| `n_heads`        | 30 |
| `n_kv_heads`     | 30 (no GQA — full MHA) |
| head_dim         | 128 (= dim / n_heads) |
| `axes_dims`      | [32, 48, 48] |
| `axes_lens`      | [1536, 512, 512] |
| `rope_theta`     | 256.0 |
| `in_channels`    | 16 |
| `all_patch_size` | [2] (2×2 spatial patch) |
| `all_f_patch_size` | [1] |
| `cap_feat_dim`   | 2560 (= Qwen3 hidden ✓) |
| `qk_norm`        | true |
| `norm_eps`       | 1e-5 |
| `t_scale`        | 1000.0 |
| `n_refiner_layers` | 2 (context_refiner + noise_refiner) |

Total tensors: **521**; total size: ~22.93 GB (`metadata.total_size` = 24,619,634,944 bytes); 3 shards (`diffusion_pytorch_model-0000{1,2,3}-of-00003.safetensors`).

### 2.1 Tensor map (transformer)

Group counts:

| group             | tensors | purpose |
|-------------------|---------|---------|
| `layers.*`        | 450 (30 × 15) | main DiT blocks |
| `context_refiner.*` | 26 (2 × 13) | refiner for cap (no adaLN) |
| `noise_refiner.*` | 30 (2 × 15) | refiner for noise (with adaLN) |
| `cap_embedder.*`  | 3 | Linear(cap_feat_dim → dim) + bias |
| `t_embedder.mlp.*` | 4 | 2-layer MLP for timestep |
| `all_x_embedder.2-1` | 2 (weight+bias) | patchify → dim |
| `x_pad_token`     | 1 | pad token embedding |
| `cap_pad_token`   | 1 | cap pad token |
| `all_final_layer.2-1` | 4 (linear w/b + adaLN w/b) | final norm + unpatchify |

#### Per-block tensor layout (layers.N, 15 tensors, adaLN blocks)

**Shapes below are verified against safetensors tensor headers (fetched 2026-09-08).** The №212 heuristic guesses (`[6*dim, dim]`, `[intermediate, dim]` with `intermediate=4*dim`, etc.) are corrected here — see §14 audit #1, #2, #9.

```
layers.N.adaLN_modulation.0.weight       # [4*dim=15360, 256]   Linear(256→4*dim); 4 chunks: scale_msa, gate_msa, scale_mlp, gate_mlp — NO shift (§14 #2)
layers.N.adaLN_modulation.0.bias         # [4*dim=15360]
layers.N.attention.to_q.weight           # [dim=3840, dim=3840]
layers.N.attention.to_k.weight           # [dim=3840, dim=3840]
layers.N.attention.to_v.weight           # [dim=3840, dim=3840]
layers.N.attention.to_out.0.weight       # [dim=3840, dim=3840]
layers.N.attention.norm_q.weight         # [head_dim=128]    # RmsNorm Q (eps=1e-5)
layers.N.attention.norm_k.weight         # [head_dim=128]    # RmsNorm K (eps=1e-5)
layers.N.attention_norm1.weight          # [dim=3840]        # RmsNorm on input×scale_msa (pre-attn); §14 #3
layers.N.attention_norm2.weight          # [dim=3840]        # RmsNorm on attn output (pre-residual); §14 #3
layers.N.feed_forward.w1.weight          # [hidden_dim=10240, dim=3840]  # SwiGLU gate; hidden_dim=int(dim/3*8) (§14 #9)
layers.N.feed_forward.w2.weight          # [dim=3840, hidden_dim=10240]  # down
layers.N.feed_forward.w3.weight          # [hidden_dim=10240, dim=3840]  # SwiGLU up
layers.N.ffn_norm1.weight                # [dim=3840]        # RmsNorm on input×scale_mlp (pre-FFN); §14 #3
layers.N.ffn_norm2.weight                # [dim=3840]        # RmsNorm on FFN output (pre-residual); §14 #3
```

**Note on intermediate_size (corrected in №232):** `hidden_dim = int(dim/3*8) = int(3840/3*8) = 10240` (NOT `4*dim=15360`). Verified via `transformer_z_image.py:213` and confirmed against safetensors headers (`layers.0.feed_forward.w1.weight=[10240, 3840]`, fetched 2026-09-08). The old №212 claim of `intermediate = 4 * dim` was a heuristic guess — discrepancy #9 in §14.

**Note on bias:** only `adaLN_modulation.0` and `all_x_embedder.2-1` and `t_embedder.mlp.{0,2}` and `cap_embedder.1` carry biases — Q/K/V/O proj and feed-forward w1/w2/w3 are bias-free. (Verified against safetensors headers, 2026-09-08.)

#### context_refiner (no adaLN, 13 tensors/block × 2 blocks)

```
context_refiner.N.attention.{to_q,to_k,to_v,to_out.0,norm_q,norm_k}.weight
context_refiner.N.{attention_norm1,attention_norm2,ffn_norm1,ffn_norm2}.weight
context_refiner.N.feed_forward.{w1,w2,w3}.weight
```

#### noise_refiner (with adaLN, 15 tensors/block × 2 blocks)

Identical layout to `layers.N.*` (adaLN_modulation + attention + ffn + 4 norms).

#### Embedders + final layer

```
# Verified shapes from safetensors headers (2026-09-08):
cap_embedder.0.weight     # [cap_feat_dim=2560]              RMSNorm gain (NOT a Linear — §14 #5)
cap_embedder.1.weight     # [dim=3840, cap_feat_dim=2560]    Linear(cap_feat_dim → dim)
cap_embedder.1.bias       # [dim=3840]
t_embedder.mlp.0.weight   # [mid_size=1024, min(dim,256)=256]  Linear(256 → 1024); §14 #1
t_embedder.mlp.0.bias     # [mid_size=1024]
t_embedder.mlp.2.weight   # [min(dim,256)=256, mid_size=1024]  Linear(1024 → 256); §14 #1
t_embedder.mlp.2.bias     # [min(dim,256)=256]
all_x_embedder.2-1.weight # [dim=3840, in_channels * patch_h * patch_w = 16*2*2 = 64]
all_x_embedder.2-1.bias   # [dim=3840]
x_pad_token               # [1, dim=3840]   (has batch dim — verified 2026-09-08)
cap_pad_token             # [1, dim=3840]   (has batch dim — verified 2026-09-08)
all_final_layer.2-1.linear.weight     # [in*ph*pw=64, dim=3840]  unpatchify
all_final_layer.2-1.linear.bias       # [64]
all_final_layer.2-1.adaLN_modulation.1.weight  # [dim=3840, min(dim,256)=256]  Linear(256 → dim); §14 #4
all_final_layer.2-1.adaLN_modulation.1.bias    # [dim=3840]
```

## 3. VAE (vae/config.json)

URL: `https://huggingface.co/Tongyi-MAI/Z-Image-Turbo/raw/main/vae/config.json`

```json
{
  "_class_name": "AutoencoderKL",
  "_name_or_path": "flux-dev",
  "act_fn": "silu",
  "block_out_channels": [128, 256, 512, 512],
  "down_block_types":   ["DownEncoderBlock2D", "DownEncoderBlock2D", "DownEncoderBlock2D", "DownEncoderBlock2D"],
  "up_block_types":     ["UpDecoderBlock2D", "UpDecoderBlock2D", "UpDecoderBlock2D", "UpDecoderBlock2D"],
  "force_upcast": true,
  "in_channels": 3,
  "latent_channels": 16,
  "latents_mean": null,
  "latents_std": null,
  "layers_per_block": 2,
  "mid_block_add_attention": true,
  "norm_num_groups": 32,
  "out_channels": 3,
  "sample_size": 1024,
  "scaling_factor": 0.3611,
  "shift_factor": 0.1159,
  "use_post_quant_conv": false,
  "use_quant_conv": false
}
```

Single file: `vae/diffusion_pytorch_model.safetensors` (167 MB).

### 3.1 Latent ritual (flux-dev style)

Confirmed by diffusers `AutoencoderKL.decode` source (`diffusers/models/autoencoders/vae.py`, `_decode`):

```python
# flux-dev style (shift_factor != 0):
latents = (latents - shift_factor) / scaling_factor
decoded = decoder(latents)
# post-quant_conv is None (use_post_quant_conv=false)
# image = (sample / 2 + 0.5).clamp(0, 1)
```

So: **`z = (latent - 0.1159) / 0.3611`** then decoder(z), then `(sample/2 + 0.5).clamp(0,1)`.

The encoder inverse is `latents = scaling_factor * sample + shift_factor` (NOT used in R3 — decode only).

### 3.2 VAE decoder tensor layout

Verified against the real VAE safetensors header (fetched 2026-09-09). **Total decoder tensors: 138** (not the old "by inspection" count, which omitted `conv_norm_out` and under-counted up-block resnets).

| group | tensors | purpose |
|-------|---------|---------|
| `decoder.conv_in.weight/bias` | 2 | latent 16ch → 4*base=512ch, 3×3 conv |
| `decoder.conv_norm_out.weight/bias` | 2 | GroupNorm(block_out_channels[0]=128) — applied in the decode() tail, before SiLU+conv_out |
| `decoder.conv_out.weight/bias` | 2 | base=128ch → 3ch, 3×3 conv |
| `decoder.mid_block.*` | 26 | 2 resnets × 8 + 10 attention (when `mid_block_add_attention=true`) |
| `decoder.up_blocks.*` | 106 | 4 blocks × 3 resnets × 8 + `conv_shortcut` (weight+bias) on `up_blocks.{2,3}.resnets.0` + upsamplers (weight+bias) on `up_blocks.{0,1,2}` |

`base = block_out_channels[0] = 128` (after conv_in: 4×base = 512).

**Per-block resnet count:** `layers_per_block + 1 = 3` resnets per ALL `up_blocks` (NOT only the last block). Source: diffusers `vae.py` L254 — `num_layers = self.layers_per_block + 1` is applied uniformly in the `up_blocks` construction loop (no special-case for the last block — verified 2026-09-09).

**decode() forward tail:** `conv_norm_out → SiLU → conv_out` — source: diffusers `vae.py` L304-311 (`hidden_states = self.conv_norm_out(hidden_states); hidden_states = nonlinearity(hidden_states); hidden_states = self.conv_out(hidden_states)`).

**Shortcuts:** `conv_shortcut` (1×1 conv with weight + bias) is added on `up_blocks.{2,3}.resnets.0` where input/output channel counts differ (512→256 for block 2, 256→128 for block 3). Each adds 2 tensors.

**Upsamplers:** `up_blocks.{0,1,2}.upsamplers.0.conv.weight/bias` — 3 Upsample2D (between blocks 0→1, 1→2, 2→3). Block 3 has no upsampler (it is the final one). Each adds 2 tensors.

> **Note (n235):** Old §3.2 schema was "by inspection" and not verified against the safetensors header — root cause of defect D1 (n235). The old schema claimed only the last `up_block` has 3 resnets (real: all 4 blocks have `lpb+1=3`), and omitted `conv_norm_out` (GroupNorm→SiLU→conv_out tail) entirely. Corrected by direct read of the real header (fetched 2026-09-09).

#### decoder.mid_block.attentions.0 tensor layout (real VAE safetensors header, fetched 2026-09-08)

BF16, 512 channels. 10 tensors. Note: `encoder.mid_block.attentions.0` has the same
layout; R3 only needs the decoder copy.

| tensor | shape | role |
|--------|-------|------|
| `decoder.mid_block.attentions.0.group_norm.weight` | [512] | GroupNorm gain |
| `decoder.mid_block.attentions.0.group_norm.bias` | [512] | GroupNorm bias |
| `decoder.mid_block.attentions.0.to_q.weight` | [512, 512] | Q projection weight |
| `decoder.mid_block.attentions.0.to_q.bias` | [512] | Q projection bias |
| `decoder.mid_block.attentions.0.to_k.weight` | [512, 512] | K projection weight |
| `decoder.mid_block.attentions.0.to_k.bias` | [512] | K projection bias |
| `decoder.mid_block.attentions.0.to_v.weight` | [512, 512] | V projection weight |
| `decoder.mid_block.attentions.0.to_v.bias` | [512] | V projection bias |
| `decoder.mid_block.attentions.0.to_out.0.weight` | [512, 512] | Output projection weight |
| `decoder.mid_block.attentions.0.to_out.0.bias` | [512] | Output projection bias |

10 tensors total for `decoder.mid_block.attentions.0` (no `proj_in` / `proj_out` —
diffusers KL VAE mid-block attention uses `to_q` / `to_k` / `to_v` / `to_out.0`,
not the transformer-style `proj_in` / `proj_out` implied by the earlier №212
shorthand; corrected in №233).

For R3 we implement **decoder only** (encoder is unused — latents come from sampler).

## 4. Scheduler (scheduler/scheduler_config.json)

URL: `https://huggingface.co/Tongyi-MAI/Z-Image-Turbo/raw/main/scheduler/scheduler_config.json`

```json
{
  "_class_name": "FlowMatchEulerDiscreteScheduler",
  "_diffusers_version": "0.36.0.dev0",
  "num_train_timesteps": 1000,
  "use_dynamic_shifting": false,
  "shift": 3.0
}
```

### 4.1 Sigma schedule (diffusers source)

`FlowMatchEulerDiscreteScheduler.set_train_timesteps` (with `use_dynamic_shifting=false` and `shift=3.0`):

```python
sigmas = linspace(1, 1/num_train, num_train)  # 1000 values: 1.0 → 0.001
sigmas = shift * sigmas / (1 + (shift-1) * sigmas)   # shift transform
timesteps = sigmas * num_train  # 1000 → 1.0
sigmas = concat([sigmas, 0.0])  # final zero
```

For inference with `num_inference_steps=9`: linspace over `[0, 999]` (9 indices), then index sigmas. **But the model README says 8 forward steps** — diffusers actually runs N-1 deltas for N sigma values, so 9 sigmas → 8 velocity evaluations. This matches the spec.

Turbo params: `num_inference_steps=9` (→ 8 DiT forwards), `guidance_scale=0.0` (no CFG branch — distilled).

### 4.2 Update rule (Euler step)

For flow matching: `x_{t+dt} = x_t + (sigma_next - sigma_t) * v(x_t, t)` where `v` is the DiT prediction. Final `sigma=0` → no update (terminal).

## 5. Text encoder (text_encoder/config.json)

URL: `https://huggingface.co/Tongyi-MAI/Z-Image-Turbo/raw/main/text_encoder/config.json`

Identical to `QWEN3_4B_CONFIG` already pinned in R2 (`src/vision/text_encoder.rs::QWEN3_4B_CONFIG`):

| field | value |
|-------|-------|
| `hidden_size` | 2560 |
| `num_hidden_layers` | 36 |
| `num_attention_heads` | 32 |
| `num_key_value_heads` | 8 (GQA) |
| `head_dim` | 128 |
| `intermediate_size` | 9728 |
| `vocab_size` | 151936 |
| `rms_norm_eps` | 1e-6 |
| `rope_theta` | 1e6 |
| `max_position_embeddings` | 40960 |
| `tie_word_embeddings` | true (no separate LM head) |
| `torch_dtype` | bfloat16 |

3 shards (~7.49 GB total: `metadata.total_size` = 8,044,936,192 bytes).

### 5.1 Text encoder tensor map

For each layer `N` (0..35): 11 weight tensors (no biases — Qwen3 attention_bias=false):

```
model.layers.N.input_layernorm.weight           # [hidden=2560]
model.layers.N.self_attn.q_proj.weight          # [q_dim=4096, hidden=2560]  (q_dim = q_heads × head_dim = 32 × 128)
model.layers.N.self_attn.k_proj.weight          # [kv_dim=1024, hidden=2560] (kv_dim = kv_heads × head_dim = 8 × 128)
model.layers.N.self_attn.v_proj.weight          # [kv_dim=1024, hidden=2560]
model.layers.N.self_attn.o_proj.weight          # [hidden=2560, q_dim=4096]
model.layers.N.self_attn.q_norm.weight          # [head_dim=128]            (QK-norm per head, RmsNorm)
model.layers.N.self_attn.k_norm.weight          # [head_dim=128]
model.layers.N.post_attention_layernorm.weight   # [hidden=2560]
model.layers.N.mlp.gate_proj.weight             # [intermediate=9728, hidden=2560]
model.layers.N.mlp.up_proj.weight               # [intermediate=9728, hidden=2560]
model.layers.N.mlp.down_proj.weight             # [hidden=2560, intermediate=9728]
```

Plus:
```
model.embed_tokens.weight  # [vocab_size=151936, hidden=2560]
model.norm.weight          # [hidden=2560]   final RmsNorm
```

(tie_word_embeddings=true ⇒ no `lm_head.weight`; if used as encoder only, the LM head is irrelevant anyway.)

## 6. Tokenizer (Qwen2Tokenizer)

Files (in `{weights_dir}/tokenizer/`):
- `tokenizer.json` (11 MB) — the canonical HF fast tokenizer format. Loaded directly by the `tokenizers` crate.
- `vocab.json` (2.7 MB) — fallback if needed.
- `merges.txt` (1.6 MB) — fallback if needed.

For Z-Image-Turbo T2I: **NO chat template applied** (diffusers `ZImagePipeline.encode_prompt` uses raw text, no chat-format wrapping — verified by inspecting diffusers `pipelines/z_image/pipeline_z_image.py`).

`encode(text)` returns token IDs `[seq]`. The TextEncoder (R2) takes `&[u32]` and produces `[seq, hidden]`. No padding/BOS-EOS injection needed for the encoder (R2 forward handles arbitrary token-id sequences).

## 7. Tokenizer rationale (for ADR-0124 update)

The HF `tokenizers` crate (already in `Cargo.lock` as transitive dep, v0.22.2) is the canonical HF implementation of fast BPE with GPT-2 pre-tokenizer + byte-mapping. Hand-rolling this is high-risk:

- Qwen2 uses byte-level BPE with a 151,643-token base vocab including 119 special tokens (added_tokens).
- Pre-tokenizer is a GPT-2-style regex split with a specific Unicode property class set; getting the regex wrong silently changes tokenization for non-ASCII inputs.
- The merges file is rank-ordered; reading it with the wrong order silently produces different IDs.

`tokenizers` is HF's verified reference implementation. The crate is deterministic (no RNG). We add it as an optional dep gated behind `vision` (NOT in `default`/`full` — §3.9).

## 8. Dtype policy

**Decision for R3:** F32 everywhere (no BF16).

Justification:
- Real model is BF16 (`torch_dtype=bfloat16` in text_encoder/config.json; transformer/vae dtype read from safetensors header at load time — no assumption).
- F32 weights: transformer 22.93 GB → ~46 GB RAM; VAE 167 MB → ~330 MB; Qwen3 7.49 GB → ~15 GB. Total: ~62 GB peak (with activations: 128×128 latent + 30×1024 seq tokens + DiT intermediate 4×3840 = 15360 → ~30 GB activations). **Does NOT fit in 32 GB RAM.**
- BF16 weights + F32 accumulation: transformer ~23 GB + ~1 GB activations + VAE ~170 MB + Qwen3 ~7.5 GB = ~32 GB peak. **Fits in 32 GB RAM barely**; fits in 64 GB comfortably.
- candle CPU supports both F32 and BF16 matmul. BF16 is slower on x86-64 (no native BF16 SIMD on most CPUs before AVX-512-BF16 / Sapphire Rapids); F32 is universally supported.

**R3 implementation strategy:** Load weights as F32 (cast from BF16 if needed). This is slower but simpler and matches the R2 text encoder contract (`TextEncoderConfig` uses `f64` for floats, tensors are `DType::F32`). Real-weights tests will be **slow** — expect minutes-to-tens-of-minutes per forward pass on CPU. The Go/No-Go report will document actual timings.

R4+ may add BF16-mixed-precision policy if needed (deferred — not in R3 scope).

## 9. Mechanics summary (Block 4 implementation guide)

### 9.1 Axial RoPE (3 axes: t/h/w, head_dim 128 = 32/48/48)

The transformer operates on a flattened sequence of tokens after patchify. Each token has 3D coordinates `(t, h, w)`:
- `t ∈ [0, num_latent_timesteps)` — typically 1 for image (single-step) — **verify in diffusers `ZImageTransformer2DModel.forward`**.
- `h ∈ [0, latent_h)` — for 1024×1024 image with patch_size=2 and 16 latent channels: latent is [1, 16, 128, 128], patchify 2×2 → 64×64 = 4096 spatial tokens.
- `w ∈ [0, latent_w)` — same as h.

`axes_lens = [1536, 512, 512]` is the **rope_scaling** for each axis (rescales positions to avoid extreme angles). `axes_dims = [32, 48, 48]` is the per-axis rotation subspace of head_dim.

Each token gets RoPE applied as: head_dim is split into 3 subspaces (32, 48, 48). For axis `i`, compute freqs `1 / theta^((2j)/axes_dims[i])` for j in 0..axes_dims[i]/2, then apply rotation using position * (1 / axes_lens[i]) as the time index.

### 9.2 T-embedding (t_scale 1000.0)

Timestep `t ∈ [0, 1000]` (sigmas scaled by t_scale). Diffusers pattern: sinusoidal embedding `sin/cos(2π × t / t_scale)` of dim `dim`, then 2-layer MLP with SiLU:

```
emb = sinusoidal(t * (1000 / num_train))  # dim
emb = mlp.1(silu(mlp.0(emb)))             # t_embedder.mlp.{0,2} weights
```

`t_embedder.mlp.0` is Linear(dim, dim); `t_embedder.mlp.2` is Linear(dim, dim). SiLU between.

### 9.3 Cap embedding (cap_feat_dim 2560 → dim 3840)

Qwen3 hidden states `[seq, 2560]` → `cap_embedder.0` Linear(2560 → 3840) → `cap_embedder.1` Linear(3840 → 3840) + bias.

```
cap = cap_embedder.1(silu(cap_embedder.0(qwen3_hidden)))
```

In diffusers `ZImageTransformer2DModel.forward` (fetched 2026-09-08): the REAL execution order is **noise_refiner (2 blocks, on x-tokens, WITH adaLN) → context_refiner (2 blocks, on cap-tokens, WITHOUT adaLN) → 30 main layers** — i.e. refiners run BEFORE main, not after. Sources: `transformer_z_image.py:985` (noise_refiner loop), `transformer_z_image.py:1001` (context_refiner loop), `transformer_z_image.py:1048` (main layers loop). The unified sequence is `[x, cap]` — x FIRST in basic mode (NOT `[cap, x]`). The old №212 claim that 'refiners run after main' and 'sequence is [cap_seq + noise_seq]' was WRONG — discrepancies #6 and #7 in §14. **For the tiny CI golden, we implement the architecture as wired here.**

### 9.4 adaLN modulation (single per-block)

```
# 6 outputs from adaLN_modulation.0: shift_msa, scale_msa, gate_msa, shift_mlp, scale_mlp, gate_mlp
mod = adaLN_modulation.0(silu(t_emb))   # [batch, 6*dim]
shift_msa, scale_msa, gate_msa, shift_mlp, scale_mlp, gate_mlp = mod.chunk(6, dim=-1)

# Self-attention with AdaLN
normed = attention_norm1(x) * (1 + scale_msa) + shift_msa
attn = attention(normed)
x = x + gate_msa * attn

# FFN with AdaLN
normed = ffn_norm1(x) * (1 + scale_mlp) + shift_mlp
ffn = feed_forward(normed)
x = x + gate_mlp * ffn
```

Note: adaLN_modulation.0 takes `silu(t_emb)` as input — the t-embedding is shared across all layers.

### 9.5 Refiner blocks (n_refiner_layers=2)

Two refiner streams:
- **context_refiner** (2 blocks, no adaLN — context = cap tokens): plain MHA + FFN with LayerNorm.
- **noise_refiner** (2 blocks, with adaLN — noise = image tokens): same as main layers.

Per diffusers source (fetched 2026-09-08): the refiners run BEFORE the 30 main blocks — `noise_refiner` (2 blocks, WITH adaLN) on x-tokens, `context_refiner` (2 blocks, no adaLN) on cap-tokens. They run in parallel (no cross-attention between them). Then the 30 main layers run on the unified `[x, cap]` sequence. The old №212 §9.5 claim that refiners run 'after the 30 main blocks' was WRONG — discrepancy #6 in §14.

### 9.6 Patchify / unpatchify

Patchify 2×2: `[1, in_channels=16, H=128, W=128]` → `[1, dim=3840, H/2=64, W/2=64]` via `all_x_embedder.2-1` Linear(in_channels*ph*pw=64 → dim=3840) applied per-patch.

Reshape: input `[1, 16, 128, 128]` → unfold 2×2 → `[1, 64, 64*64=4096]` patches → transpose → `[1, 4096, 64]` → Linear → `[1, 4096, 3840]`.

Unpatchify is the inverse: `all_final_layer.2-1.linear` Linear(3840 → 64) → reshape → `[1, 16, 128, 128]`.

## 10. R2 contract — INVARIANT

`TextEncoder::new(config, seed)` and its golden SHA-256 records (pinned in naryad №230) MUST NOT be modified. R3 adds `TextEncoder::from_weights(config, &HashMap<String, Tensor>)` as a parallel construction path — the seeded-init path remains untouched.

This invariant is asserted by running the R2 golden test as part of R3 verification (Block 7).

## 11. Precedents / sources consulted

- `candle-examples` flux example (DiT-style: double-stream, qk-norm, rope).
- `candle-examples` stable-diffusion (VAE decoder KL).
- `diffusers/models/transformers/transformer_zimage.py` (ZImageTransformer2DModel).
- `diffusers/models/autoencoders/vae.py` (AutoencoderKL.decode — flux-style shift/scaling).
- `diffusers/schedulers/scheduling_flow_match_euler_discrete.py` (sigma schedule).
- Existing Metalogos: `src/vision/text_encoder.rs` (R2 architecture patterns).

## 12. Contradictions / open questions

None observed — all `config.json` values match the naryad spec's §1 exactly. The `intermediate_size` for the transformer feed-forward is not in `config.json`; it must be read at weight-load time from `layers.0.feed_forward.w1.weight` shape (presumably `15360 = 4 × 3840`, but verified at runtime).

## 13. CI-visible vs env-gated split

| Block | CI (tiny golden) | env-gated (real weights) |
|-------|------------------|--------------------------|
| Block 1 (weights.rs, tokenizer.rs) | No (no weights) | Yes (loaded by from_weights) |
| Block 2 (TextEncoder::from_weights) | No (R2 golden covers architecture) | Yes (real Qwen3 forward) |
| Block 3 (VAE decoder) | Yes (VaeDecoder::new_tiny + tiny golden) | Yes (real VAE decode fixed-latent → PNG) |
| Block 4 (DiT + sampler) | Yes (tiny golden pinned №231) | Yes (called by e2e test) |
| Block 5 (e2e) | No (needs real weights) | Yes (clinical_e2e_first_image) |

Env-gated tests SKIP loudly when `MLOG_VISION_WEIGHTS_DIR` is unset — they are NOT `#[ignore]`. This is the §3 form of permitted unfinishedness.

## 14. Reference fidelity audit №232

**Audit date:** 2026-09-08.
**Auditor:** general-purpose sub-agent (Наряд №232, Block 0).
**Reference source:** `diffusers` main branch (`huggingface/diffusers`), files `src/diffusers/models/transformers/transformer_z_image.py` and `src/diffusers/pipelines/z_image/pipeline_z_image.py`, fetched 2026-09-08.
**Method:** line-by-line comparison of №212 fact-sheet claims against diffusers source + safetensors tensor headers (verified 2026-09-08 via direct read of `diffusion_pytorch_model-0000{1,2,3}-of-00003.safetensors` headers).

### 14.1 Discrepancy table (12 issues)

| # | What №212 claimed (wrong) | Correct reference (file:line, fetched 2026-09-08) | Fix applied in №232 |
|---|---------------------------|--------------------------------------------------|----------------------|
| 1 | t-embedder uses dim=3840 path: `Linear(dim→dim) → SiLU → Linear(dim→dim)`, sinusoidal input dim=3840 | `transformer_z_image.py:43-45`: `TimestepEmbedder(min(dim, 256), mid_size=1024)` → sinusoidal(256) → `Linear(256→1024)` → SiLU → `Linear(1024→256)` | `t_embedder.mlp.0` = `Linear(256→1024)`, `t_embedder.mlp.2` = `Linear(1024→256)`. Shapes match safetensors: `mlp.0.weight=[1024, 256]`, `mlp.2.weight=[256, 1024]` |
| 2 | Block adaLN: 6 chunks (shift_msa/scale_msa/gate_msa/shift_mlp/scale_mlp/gate_mlp) with shift (§9.4 L337-339) | `transformer_z_image.py` (`ZImageTransformerBlock.forward`): `Linear(256→4*dim)`, 4 chunks (`scale_msa`, `gate_msa`, `scale_mlp`, `gate_mlp`), `gate=tanh(gate)`, `scale=1+scale`, NO shift | adaLN now 4 chunks; `gate=tanh(gate)`, `scale=1+scale`, no shift. Shape `[15360, 256] = [4*dim, 256]` matches safetensors |
| 3 | Block norms: 2 RmsNorms on input only (`attention_norm1`, `ffn_norm1`); `attention_norm2`/`ffn_norm2` unused in §9.4 pseudocode | `transformer_z_image.py` (`ZImageTransformerBlock`): 4 RmsNorms — `attention_norm1` on `input×scale_msa`, `attention_norm2` on attn output (pre-residual), `ffn_norm1` on `input×scale_mlp`, `ffn_norm2` on FFN output (pre-residual) | Block now uses all 4 RmsNorms; `attention_norm2`/`ffn_norm2` applied to sublayer outputs before residual addition |
| 4 | Final layer: `RmsNorm(ones) + shift + scale + gate + residual` (AdaLayerNormContinuous-style) | `transformer_z_image.py` (`FinalLayer`): `LayerNorm(dim, affine=False, eps=1e-6)` → `×(1+scale)` → `Linear`. No gate, no shift, no residual | Final layer: `LayerNorm(affine=False, eps=1e-6)` — no learnable params in norm, only `scale` from adaLN, no gate/shift/residual. Shape `[3840, 256] = [dim, 256]` matches safetensors |
| 5 | `cap_embedder`: `Linear(2560→3840) → SiLU → Linear(3840→3840)` (2 Linears + SiLU) (§9.3 L326) | `transformer_z_image.py`: `Sequential(RMSNorm(cap_feat_dim, eps), Linear(cap_feat_dim→dim, bias=True))` — no SiLU, 1 Linear only | `cap_embedder.0` = `RMSNorm(2560)` (gain only, shape `[2560]`); `cap_embedder.1` = `Linear(2560→3840)` (shape `[3840, 2560]` + bias `[3840]`) |
| 6 | №212 §9.5 (L360) claimed 'after the 30 main blocks, the sequence is split — context_refiner runs on cap, noise_refiner runs on noise' (refiners AFTER main) | `transformer_z_image.py:985` (noise_refiner loop, BEFORE main), `:1001` (context_refiner loop, BEFORE main), `:1048` (main layers loop, LAST) | Refiners now run BEFORE main: `noise_refiner` (2 blocks, WITH adaLN) on x-tokens, `context_refiner` (2 blocks, no adaLN) on cap-tokens, THEN 30 main layers |
| 7 | №212 §9.3 (L332) claimed unified sequence is `[cap_seq + noise_seq]` (cap first) | `transformer_z_image.py` (`ZImageTransformer2DModel.forward`): unified sequence is `[x, cap]` — x FIRST (basic mode) | Sequence concatenation flipped to `[x, cap]` |
| 8 | Rust code (informed by №212 §9.1) used 1D R2 RoPE pattern (single axis, real-real rotation pairs) | `transformer_z_image.py:107-122`: axial 3-axes (`axes_dims=[32, 48, 48]`, `axes_lens=[1536, 512, 512]`) with complex rotation (real+imag interleaved) | applied in №233 (PR #231) — rope.apply(q,k) wired into attention_forward after qk-norm; clamp replaced with loud Err |
| 9 | FeedForward `hidden_dim = 4*dim = 15360` (§2.1 Note on intermediate_size, §12 L387) | `transformer_z_image.py:213`: `hidden_dim = int(dim/3*8) = int(3840/3*8) = 10240` | FFN `hidden_dim` corrected to 10240; tensor shapes `w1=[10240, 3840]`, `w2=[3840, 10240]`, `w3=[10240, 3840]` match safetensors |
| 10 | №212 §3.1 (L158) claimed `latents = (latents - shift_factor) / scaling_factor` (subtract first, then divide) | `pipeline_z_image.py:589`: `latents = latents / scaling_factor + shift_factor` (divide first, then add — reversed order) | VAE decode now: `z = latent / scaling + shift` (NOT `(latent - shift) / scaling`) |
| 11 | Rust VAE decoder had `mid_attn=None` (skipped mid-block attention) | `vae/config.json`: `mid_block_add_attention: true`; diffusers `AutoencoderKL` decoder: mid-block attention weights present (`group_norm`, `proj_in`, `q/k/v/out_proj`, `proj_out`) | applied in №233 (PR #231) — VaeAttention struct + from_weights loading + compute in decode |
| 12 | Rust attention applied 1D RoPE pattern BEFORE qk-norm | `transformer_z_image.py:107-122`: MHA 30/30 head_dim 128, `qk_norm=RMSNorm(eps=1e-5)`, `freqs_cis` applied AFTER qk-norm via `apply_rotary_emb` (complex rotation) | applied in №233 (PR #231) — qk-norm THEN rope.apply, order verified per L107-122 |

### 14.2 Pinned source facts

All references below are from `diffusers` main branch (`huggingface/diffusers`), fetched 2026-09-08.

**Source URLs:**
- Transformer: `https://github.com/huggingface/diffusers/blob/main/src/diffusers/models/transformers/transformer_z_image.py`
- Pipeline: `https://github.com/huggingface/diffusers/blob/main/src/diffusers/pipelines/z_image/pipeline_z_image.py`

**Pinned facts:**

| fact | value | source (file:line, fetched 2026-09-08) |
|------|-------|----------------------------------------|
| cap pos_ids `start` | `(1, 0, 0)` | `transformer_z_image.py:599` |
| cap pos_ids `grid_size` | `(padded_cap_len, 1, 1)` | `transformer_z_image.py:599` |
| cap pos_ids per-token | `(1+i, 0, 0)` (i = token index, 0..padded_cap_len) | `transformer_z_image.py:598-600` (`create_coordinate_grid` with `start=(1,0,0)`, `grid_size=(padded_cap_len,1,1)`) + L535-539 (`create_coordinate_grid` implementation: `arange(x0, x0+span)`) |
| x pos_ids `start` | `(cap_len+1, 0, 0)` | `transformer_z_image.py:608` |
| x pos_ids `grid_size` | `(F_t, H_t, W_t)` | `transformer_z_image.py:608` |
| t scaling before t_embedder | `t * self.t_scale` (t_scale=1000.0) | `transformer_z_image.py:945` |
| SiLU between t_embedder.mlp.0 and mlp.2 | yes — `F.silu(mlp.0(emb))` then `mlp.2(...)` | `transformer_z_image.py:43-45` |
| RoPE application order | `qk_norm` FIRST, then `apply_rotary_emb(q, freqs_cis)` | `transformer_z_image.py:107-122` |
| FeedForward `hidden_dim` | `int(dim/3*8) = 10240` | `transformer_z_image.py:213` |
| VAE latent ritual | `latents = latents / scaling_factor + shift_factor` | `pipeline_z_image.py:589` |
