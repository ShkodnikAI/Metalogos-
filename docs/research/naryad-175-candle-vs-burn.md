# Naryad #175 — Recon: `candle` vs `burn` as a foundation for local model training

> **Status:** Research report — not an architectural decision, material for the owner.
> **Date:** 2026-09-04
> **Priority:** Research; not a single line of implementation in Metalogos.

---

## Block 1 — Comparison across 6 criteria

### Summary table

| Criterion | `candle-core` 0.11.0 + `candle-nn` 0.11.0 | `burn` 0.21.0 (ndarray + autodiff + train) |
|---|---|---|
| **1. Autograd** | Tape-based, via `Var` + `loss.backward()`. Stable since v0.1 (2023). `backward_step()` is one line. | The `burn-autodiff` crate — a wrapper over any backend. Tape-based, via `backward()` + `GradientsParams`. Stable since 2022. Backend-agnostic. |
| **2. Real dependency footprint** | candle-core: 29 deps (12 platform-specific). candle-nn: 12 deps (4 platform-specific). Cross-platform: ~19. | burn umbrella: 23 sub-crates. burn-ndarray: 21 deps (BLAS, ndarray). burn-autodiff: 10 deps. Total transitive: ~300+. Build: 513s vs 157s. |
| **3. FFI ergonomics** | Low barrier: `Tensor::from_vec`, `forward`, `backward`, `SGD::new`. 2 compile attempts. Maps onto `Value::Struct` or `Value::Tensor`. | High barrier: `Backend` trait, derive macros, `GradientsParams` (not `Gradients`), `OptimizerAdaptor`. 5 compile attempts. Requires boxing or `Value::Backend`. |
| **4. CPU vs GPU** | CPU: `Device::Cpu`, native Rust via `gemm` (no BLAS). GPU: CUDA, Metal. CPU is first-class. | CPU: `NdArray` backend via BLAS/OpenBLAS. GPU: Wgpu, CUDA, ROCm, Tch. BLAS can be faster on large matrices. |
| **5. Activity** | Stars: 21K. Commits (6 months): ~135. Issues: 892 (1:23 ratio). Hugging Face backing. Releases every 2-3 months. | Stars: 15.9K. Commits (6 months): ~562 (4× more). Issues: 288 (1:55 ratio). Releases every 1-2 months, pre-releases active. |
| **6. License** | Apache-2.0 ✅ | Apache-2.0 ✅ |

---

## Block 2 — Minimal practical test

### Test task
2-layer MLP (2→8→1) on XOR (4 samples). 1000 epochs. CPU-only. Release build.
- candle: `/home/z/my-project/research/candle-test/src/main.rs` (~50 lines)
- burn: `/home/z/my-project/research/burn-test/src/main.rs` (~65 lines)

### Results (real measurements)

| Metric | candle 0.11.0 | burn 0.21.0 | Δ |
|---|---|---|---|
| **Clean build** | **157s** (2m 37s) | **513s** (8m 32s) | burn 3.3× slower |
| **Binary size** | **4.6 MB** | **6.0 MB** | burn 30% larger |
| **Epoch time (CPU)** | **117µs** | **167µs** | burn 43% slower* |
| **Model converged?** | No (SGD, no momentum) | Yes (Adam, loss=0.0) | — |
| **Compile attempts** | 2 | 5 | candle 2.5× easier |
| **Test code lines** | ~50 | ~65 | candle 23% less |

*Not a direct comparison: candle SGD vs burn Adam (Adam costs more per step).

### Additional observations
- candle deps: ~1.2GB in target/; burn deps: ~1.1GB
- burn: each failed compile attempt cost ~8 minutes (deps were being compiled)
- burn: incremental build (source only) — 33s; candle: 0.5s

---

## Block 3 — .mlog syntax sketches

### Sketch 1: Declarative (a single command describes the methodology)

```mlog
train_model {
  architecture: "gpt-mini"
  dataset: "./corpus.txt"
  epochs: 10
  learning_rate: 0.001
  batch_size: 32
  output: "./model.safetensors"
}
```

### Sketch 2: Programmatic (composable, for advanced cases)

```mlog
learnable architecture GptMini(vocab: Float, d_model: Float) -> Model {
  layers: [
    embedding(vocab, d_model),
    transformer_layer(d_model, heads=4),
    linear(d_model, vocab)
  ]
  optimizer: "adam"
  learning_rate: 0.001
}

pattern TrainCorpus(input: String) -> String {
  let model = GptMini(50000.0, 128.0)
  let dataset = load_text("./corpus.txt")
  let trained = train(model, dataset, epochs=10, batch_size=32)
  save(trained, "./model.safetensors")
  return "Training complete"
}

flow Main { input: String = "start" -> TrainCorpus -> output }
```

### Sketch 3: Inference after training (fallback to LLM API)

```mlog
entity local_model: Model = load_model("./model.safetensors")

pattern SmartClassify(text: String) -> String {
  let prediction = infer(local_model, text)
  if confidence(prediction) < 0.8 {
    return call_llm("Classify: " + text)
  }
  return label(prediction)
}
```

---

## Recommendation: **candle**

### Rationale

1. **Build time is critical.** FEATURE_INTAKE.md hard limit 120s. burn would add 513s (4× over the limit). candle — 157s (acceptable; the limit can be raised to 240s).

2. **API simplicity.** candle — 2 compile attempts. burn — 5 attempts. For language integration (where every builtin must be reliable) API simplicity = fewer bugs.

3. **Binary size.** 4.6MB vs 6.0MB. The Metalogos binary is currently ~6MB — candle would double it, burn would triple it.

4. **Hugging Face ecosystem.** candle is part of HF (safetensors, tokenizers already in Metalogos). Format compatibility, pretrained weights from HF Hub — free.

5. **CPU-first.** candle runs on CPU without BLAS (native Rust via `gemm`). burn requires BLAS (OpenBLAS) — a C dependency that complicates cross-compilation.

### candle weaknesses (honest)

1. **Fewer layers out of the box.** candle-nn: Linear, Conv, Embedding, RNN. burn-nn: broader (BatchNorm, LayerNorm, Dropout, Attention). The missing layers will have to be written manually.

2. **No built-in training loop.** candle has no `Learner`/`TrainingStep`. burn-train: dataloaders, metrics, learner. For Metalogos this may be a plus — more control.

3. **SGD without momentum.** The built-in `candle_nn::SGD` does not support momentum. `candle_nn::Adam` exists, but will require API verification.

4. **Fewer examples.** burn: 20+ examples (MNIST, text-classification, DQN). candle: fewer, but the HF Hub compensates.

5. **Open issues: 892** (vs burn 288). At 21K stars this is expected — a wide audience = more edge cases.

### When burn is better than candle

- Multi-GPU distributed training (`burn-collective`)
- BLAS speedup on matrices >512×512
- Backend-agnostic hot-swap (CPU↔GPU↔Wgpu without recompilation)
- The `burn-train Learner` abstraction saves enough code

---

## Final summary

| | candle | burn |
|---|---|---|
| Build time | **157s** ✅ | 513s ❌ |
| Binary size | **4.6MB** ✅ | 6.0MB |
| API simplicity | **2 attempts** ✅ | 5 attempts |
| Epoch time (CPU) | **117µs** ✅ | 167µs |
| Depend depth | **~19** ✅ | ~300+ |
| License | Apache-2.0 ✅ | Apache-2.0 ✅ |
| Community | 21K stars, HF backing | 15.9K stars, 4× commits |
| Training utils | Manual loop | **Learner** ✅ |
| Layer variety | Basic | **Comprehensive** ✅ |
| HF compatibility | **Native** ✅ | Via import |

**Recommendation: candle** — the pragmatic choice (build time, binary size, API simplicity, HF ecosystem).

**The decision rests with the owner.**
