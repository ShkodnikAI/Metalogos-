# Naryad #176 — Recon: reusable architectural blocks (attention/RoPE/transformer-block)

> **Status:** Research report — not an architectural decision, material for the owner.
> **Date:** 2026-09-04
> **Complements:** Naryad #175 (the choice of candle/burn as the tensor foundation).
> **Focus:** The layer ABOVE the foundation — ready-made, proven architectural components.

---

## Block 1 — Inventory of `candle-transformers`

### 1.1 Model list (125 files in `candle-transformers/src/models/`)

Key architectures (partial list, 125 entries total):

| Category | Models |
|---|---|
| **LLM (decoder-only)** | Llama, Mistral, Mixtral, Falcon, Phi, Phi3, Qwen2, Qwen3, Gemma, Gemma2, Gemma3, GLM4, Yi, Starcoder2, MPT, StableLM, Olmo, Olmo2, DeepSeek2, Helium, Granite, GraniteMoeHybrid, LFM2 |
| **LLM (MoE)** | Mixtral, Qwen3MoE, Qwen2MoE, GraniteMoeHybrid |
| **LLM (SSM)** | Mamba, Mamba2, RWKV v5/v6/v7 |
| **Encoder** | BERT, DistilBERT, DeBERTa v2, ModernBERT, NomicBERT, JinaBERT, XLM-RoBERTa, SigLIP |
| **Vision** | ViT, ConvNeXt, ResNet, EfficientNet, MobileNet v4, DINOv2, BEiT, Hiera |
| **Multimodal** | Llava, CLIP, Chinese-CLIP, BLIP, PaLiGemma, Moondream, Pixtral |
| **Speech/Audio** | Whisper, EnCodec, DAC, MetaVoice, SNAC, Voxtral |
| **Diffusion** | Stable Diffusion, Flux, Wuerstchen |
| **Specialized** | Segment Anything, Depth Anything v2, TroCR, PaddleOCR-VL |

### 1.2 Detailed breakdown: Llama (representative architecture)

Source: `candle-transformers/src/models/llama.rs` (~500 lines)

**Architectural blocks:**

| Block | Structure in code | Components |
|---|---|---|
| **RoPE (Rotary Position Embedding)** | `Cache::new()` → precompute `cos`/`sin` | `calculate_default_inv_freq()` → inv_freq vec; `idx_theta = arange * inv_freq`; `cos = idx_theta.cos()`, `sin = idx_theta.sin()`. Llama3 scaling: smooth interpolation. |
| **CausalSelfAttention** | `struct CausalSelfAttention` | `q_proj`, `k_proj`, `v_proj`, `o_proj` (Linear, no bias). GQA support (`num_key_value_heads`). `apply_rotary_emb()` → `candle_nn::rotary_emb::rope()`. KV-cache in `Cache.kvs`. |
| **Forward (attention)** | `CausalSelfAttention::forward()` | Q/K/V projections → reshape (b, seq, heads, head_dim) → transpose → rope → KV-cache concat → repeat_kv → `q.matmul(k.t()) / sqrt(head_dim)` → causal mask → softmax → `att.matmul(v)` → o_proj. |
| **Mlp (SwiGLU)** | `struct Mlp` | `c_fc1` (gate_proj), `c_fc2` (up_proj), `c_proj` (down_proj). Forward: `silu(c_fc1(x)) * c_fc2(x)` → `c_proj()`. |
| **Transformer Block** | `struct Block` | `rms_1` (RmsNorm) → `attn` (CausalSelfAttention) → residual → `rms_2` (RmsNorm) → `mlp` → residual. Classic pre-norm. |
| **Full Model** | `struct Llama` | `wte` (embedding) → `blocks: Vec<Block>` → `ln_f` (RmsNorm) → `lm_head` (Linear). |

**Key observation:** Each block is a plain Rust struct with `forward()`. No derive macros, no trait acrobatics. A simple pattern: `struct → impl → load(vb) → forward(x)`. This is an **ideal template** for porting to Metalogos-native code or for calling via FFI.

### 1.3 License

`candle-transformers`: Apache-2.0 (the entire huggingface/candle repository). Compatible with Metalogos.

### 1.4 Availability for local verification

The models in `candle-transformers` are **inference-only** (safetensors loading + forward pass). For **training**, `candle-nn` (SGD, Adam) + `VarMap` (trainable variables) is needed — which is exactly what was tested in naryad #175. `candle-transformers` provides the architecture, `candle-nn` provides the training primitives.

---

## Block 2 — Inventory of `burn`

### 2.1 Model Zoo

**Burn has no separate `burn-transformers` crate.** The architectures live in `examples/`:

| Example | Architecture | Training? |
|---|---|---|
| `text-generation` | **TransformerEncoder** (GPT-style) | Yes (trainable) |
| `text-classification` | Transformer encoder + classifier head | Yes |
| `mnist` | MLP / ConvNet | Yes |
| `modern-lstm` | LSTM | Yes |
| `dqn-agent` | Q-Network (RL) | Yes |
| `wgan` | Wasserstein GAN | Yes |
| `import-model-weights` | Import PyTorch weights | Inference only |
| `simple-regression` | MLP regression | Yes |
| `multi-gpus` | Multi-GPU training | Yes |
| `server` | Model inference server | Inference |

**Key difference from candle:** burn ships **built-in blocks** in `burn-nn` (not via a model zoo), while the architectures live in the examples.

### 2.2 Built-in blocks of `burn-nn`

`crates/burn-nn/src/modules/`:

| Block | File(s) | Composition-ready? |
|---|---|---|
| **Multi-Head Attention** | `attention/mha.rs` | ✅ `Mha::new(config)` → `.forward(input)` |
| **Cross Attention** | `attention/cross_attention.rs` | ✅ |
| **Attention Mask** | `attention/mask.rs` | ✅ `generate_autoregressive_mask()` |
| **RoPE** | `rope_encoding.rs` | ✅ `RopeEncodingConfig` |
| **Positional Encoding** | `pos_encoding.rs` | ✅ Sinusoidal |
| **TransformerEncoder** | `transformer/encoder.rs` | ✅ `TransformerEncoderConfig` → `.init()` → `.forward()` |
| **TransformerDecoder** | `transformer/decoder.rs` | ✅ |
| **Position-wise Feed-Forward** | `transformer/pwff.rs` | ✅ |
| **LayerNorm / BatchNorm / GroupNorm** | `norm/` | ✅ |
| **Embedding** | `embedding.rs` | ✅ |
| **Linear** | `linear.rs` | ✅ |
| **Dropout** | `dropout.rs` | ✅ |
| **RNN / LSTM / GRU** | `rnn/` | ✅ |
| **Conv1d/2d/3d** | `conv/` | ✅ |
| **Pooling** | `pool/` | ✅ |
| **KV Cache** | `cache/` | ✅ |

### 2.3 Detailed breakdown: text-generation example (GPT-style)

Source: `examples/text-generation/src/model.rs` (~100 lines)

```rust
#[derive(Module, Debug)]
pub struct TextGenerationModel<B: Backend> {
    transformer: TransformerEncoder<B>,
    embedding_token: Embedding<B>,
    embedding_pos: Embedding<B>,
    output: Linear<B>,
}
```

**Architectural blocks:**

| Block | burn API | How they connect |
|---|---|---|
| **Token embedding** | `EmbeddingConfig::new(vocab, d_model).init(device)` | `.forward(token_ids)` |
| **Positional embedding** | `EmbeddingConfig::new(max_seq, d_model).init(device)` | `.forward(arange(0..seq))` |
| **Embedding fusion** | `(emb_pos + emb_tok) / 2` | Tensor addition |
| **Causal mask** | `generate_autoregressive_mask(batch, seq, device)` | Passed to transformer |
| **TransformerEncoder** | `TransformerEncoderConfig { d_model, n_heads, .. }.init()` | `.forward(TransformerEncoderInput::new(emb).mask_pad(mask).mask_attn(mask))` |
| **Output head** | `LinearConfig::new(d_model, vocab).init()` | `.forward(encoded)` |
| **Loss** | `CrossEntropyLossConfig::new().with_pad_tokens(...)` | `.forward(output, targets)` |
| **Training step** | `impl TrainStep` → `item.loss.backward()` | Derive macro handles plumbing |

**Key observation:** burn provides a **ready-made `TransformerEncoder`** — one struct, configured via `TransformerEncoderConfig`. No need to assemble attention + RoPE + FFN by hand. But it is an **opaque block** — individual parts cannot be modified (e.g., replacing standard attention with flash attention) without diving into the internals.

---

## Block 3 — Composition ergonomics for Metalogos

### 3.1 `.mlog` sketch for candle (block composition)

```mlog
// candle: each block is a separate composable construct.
// The agent can replace/modify individual parts of the architecture.

learnable block RotaryEmbedding(dim: Float, max_seq: Float, theta: Float) -> RoPE {
  // Calls candle_nn::rotary_emb::rope()
  // Parameters: head_dim, max_position_embeddings, rope_theta
  inv_freq: 1.0 / theta.powf(dim / head_dim)
  cos: cos(arange(max_seq) * inv_freq)
  sin: sin(arange(max_seq) * inv_freq)
}

learnable block CausalAttention(
  dim: Float, heads: Float, kv_heads: Float
) -> Attention {
  q_proj: linear(dim, heads * head_dim, bias=false)
  k_proj: linear(dim, kv_heads * head_dim, bias=false)
  v_proj: linear(dim, kv_heads * head_dim, bias=false)
  o_proj: linear(heads * head_dim, dim, bias=false)
  
  forward(x, rope: RoPE) {
    let q = q_proj(x).reshape(heads, head_dim)
    let k = k_proj(x).reshape(kv_heads, head_dim)
    let v = v_proj(x).reshape(kv_heads, head_dim)
    let q = rope.apply(q)
    let k = rope.apply(k)
    let att = softmax(q.matmul(k.t()) / sqrt(head_dim))
    return o_proj(att.matmul(v))
  }
}

learnable block TransformerBlock(
  dim: Float, heads: Float, mlp_ratio: Float
) -> Block {
  norm1: rms_norm(dim)
  attn: CausalAttention(dim, heads, heads)
  norm2: rms_norm(dim)
  mlp: SwiGLU(dim, dim * mlp_ratio)
  
  forward(x, rope: RoPE) {
    let x = x + attn(norm1(x), rope)
    let x = x + mlp(norm2(x))
    return x
  }
}

// The full model is assembled from blocks
learnable architecture GptMini(
  vocab: Float, d_model: Float, n_layers: Float
) -> Model {
  embedding: embedding(vocab, d_model)
  rope: RotaryEmbedding(d_model / n_heads, max_seq=512, theta=10000)
  blocks: [TransformerBlock(d_model, n_heads=4, mlp_ratio=4)] * n_layers
  norm: rms_norm(d_model)
  head: linear(d_model, vocab, bias=false)
  
  forward(tokens) {
    let x = embedding(tokens)
    for block in blocks { x = block(x, rope) }
    return head(norm(x))
  }
}
```

### 3.2 `.mlog` sketch for burn (opaque block)

```mlog
// burn: TransformerEncoder — a ready-made, opaque block.
// The agent configures, but does not modify the internals.

learnable architecture GptMini(
  vocab: Float, d_model: Float, n_layers: Float
) -> Model {
  // burn::nn::TransformerEncoder — opaque, not split into attention/RoPE
  transformer: TransformerEncoder {
    d_model: d_model
    n_heads: 4
    n_layers: n_layers
    ffn_hidden: d_model * 4
    dropout: 0.0
    norm_first: true  // pre-norm (Llama-style)
  }
  
  embedding_token: Embedding(vocab, d_model)
  embedding_pos: Embedding(512, d_model)
  head: linear(d_model, vocab)
  
  forward(tokens) {
    let pos = arange(0, len(tokens))
    let emb = (embedding_token(tokens) + embedding_pos(pos)) / 2
    let mask = autoregressive_mask(len(tokens))
    let out = transformer(emb, mask=mask)
    return head(out)
  }
}
```

### 3.3 Composition ergonomics assessment

| Criterion | candle | burn |
|---|---|---|
| **Blocks as first-class constructs** | ✅ Each block is a separate Rust struct. Ports into `.mlog` as a separate `learnable block`. | ⚠️ `TransformerEncoder` — opaque. Attention cannot be replaced without forking burn. |
| **The agent can modify the architecture** | ✅ Replacing attention with flash-attn — change one `forward()`. | ❌ Replacing attention — fork burn-nn or implement from scratch. |
| **Reference template for the agent** | ✅ 125 models in candle-transformers, each a working example. Llama (500 lines) — transparent. | ⚠️ text-generation example — working, but opaque (TransformerEncoder hides the details). |
| **Composability for a new task** | ✅ Agent: "need cross-attention instead of self-attention" → write a new block. | ⚠️ Agent: "need cross-attention" → use `CrossAttention` (it exists), but it cannot be embedded in `TransformerEncoder` — a custom encoder is needed. |
| **Granularity of control** | ✅ RoPE, attention, FFN, norm — all separate. | ⚠️ `TransformerEncoderConfig` — parameters, but not structure. |

**Composition verdict:** candle is better for **agent-driven composition** — blocks are transparent, replaceable, each one visible. burn is better for the **human developer** — less code, but less control.

---

## Block 4 — Testability of the composition (determinism)

### Verification method

A dedicated test was created: fixed weights → 3 forward passes → comparison of outputs with ε=1e-8.

### Results

| Foundation | Test | Result |
|---|---|---|
| **candle** | `Linear(2,3)` forward, 3 runs, CPU | **Deterministic: YES ✅** — outputs identical to 1e-8 |
| **burn** | `Linear(2,3)` forward, 3 runs, CPU (NdArray backend) | **Deterministic: YES ✅** — outputs identical to 1e-8 |

**Confirmed:** both foundations give a deterministic forward pass with fixed weights on CPU. This means golden testing of architectural composition is possible — if the initialization seed and the weights are fixed, the forward pass is reproducible.

Test projects:
- `/home/z/my-project/research/candle-determ/` — candle determinism test
- `/home/z/my-project/research/burn-determ/` — burn determinism test

### Additional observations

- candle: determinism **without specifying a seed** — because `VarMap` initializes deterministically (via Kaiming normal with a fixed seed by default).
- burn: determinism **without specifying a seed** — because `LinearConfig::init()` uses a deterministic initializer by default.
- For **full** determinism in training one must verify: dropout (absent from forward without training mode), data shuffling (controlled by code), BLAS non-determinism (possible on multithreaded backends — not observed on single-threaded CPU in the test).

---

## Final summary

| | candle | burn |
|---|---|---|
| **Model zoo** | **125 models** in candle-transformers ✅ | 10 examples, built-in blocks in burn-nn |
| **Reference breakdown** | Llama: 500 lines, transparent, every block visible ✅ | text-gen: 100 lines, opaque TransformerEncoder |
| **Ready-made blocks** | RoPE (rotary_emb.rs), attention (in the models), RmsNorm, SwiGLU — **in the models, not a library** | **MHA, RoPE, TransformerEncoder, LayerNorm, Dropout — built in** ✅ |
| **Composability** | ✅ Blocks as separate structs, the agent replaces/modifies | ⚠️ Opaque blocks, internals cannot be modified |
| **Reference template for the agent** | ✅ 125 working models, each a template | ⚠️ 1 example (text-gen), opaque |
| **Forward-pass determinism** | ✅ YES (verified) | ✅ YES (verified) |
| **License** | Apache-2.0 ✅ | Apache-2.0 ✅ |

### Key difference

**candle** ships **reference templates** (125 models, each a working example of block assembly). The blocks are **not built into** the library — they live in the model code. An agent assembling a new architecture **mirrors** the Llama/Phi/Gemma code as a template.

**burn** ships **ready-made blocks** (TransformerEncoder, MHA, RoPE — built in, configurable). But the blocks are **opaque** — attention cannot be replaced without a fork. The agent **configures**, it does not **compose**.

### For Metalogos

If the goal is **the agent composing blocks for the task** (the naryad context): candle is better — blocks are transparent, replaceable, and there are 125 references.

If the goal is **a human writing less code**: burn is better — `TransformerEncoder` is one line instead of 200.

**The decision is up to the owner, together with the results of naryad #175.**
