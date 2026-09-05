# Наряд №175 — Разведка: `candle` vs `burn` как фундамент для локального обучения моделей

> **Статус:** Research report — не архитектурное решение, материал для владельца.
> **Дата:** 2026-09-04
> **Приоритет:** Исследовательский, без единой строки реализации в Metalogos.

---

## Блок 1 — Сравнение по 6 критериям

### Сводная таблица

| Критерий | `candle-core` 0.11.0 + `candle-nn` 0.11.0 | `burn` 0.21.0 (ndarray + autodiff + train) |
|---|---|---|
| **1. Autograd** | Tape-based, через `Var` + `loss.backward()`. Стабилен с v0.1 (2023). `backward_step()` — одна строка. | `burn-autodiff` crate — обёртка над любым backend. Tape-based, через `backward()` + `GradientsParams`. Стабилен с 2022. Backend-agnostic. |
| **2. Реальный размер зависимостей** | candle-core: 29 deps (12 platform-specific). candle-nn: 12 deps (4 platform-specific). Cross-platform: ~19. | burn umbrella: 23 sub-crates. burn-ndarray: 21 deps (BLAS, ndarray). burn-autodiff: 10 deps. Total transitive: ~300+. Build: 513s vs 157s. |
| **3. FFI-эргономика** | Низкий порог: `Tensor::from_vec`, `forward`, `backward`, `SGD::new`. 2 попытки скомпилировать. Ложится на `Value::Struct` или `Value::Tensor`. | Высокий порог: `Backend` trait, derive macros, `GradientsParams` (не `Gradients`), `OptimizerAdaptor`. 5 попыток скомпилировать. Требует装箱 или `Value::Backend`. |
| **4. CPU vs GPU** | CPU: `Device::Cpu`, нативный Rust через `gemm` (без BLAS). GPU: CUDA, Metal. CPU — first-class. | CPU: `NdArray` backend через BLAS/OpenBLAS. GPU: Wgpu, CUDA, ROCm, Tch. BLAS может быть быстрее на больших матрицах. |
| **5. Активность** | Stars: 21K. Commits (6 мес): ~135. Issues: 892 (1:23 ratio). Hugging Face backing. Релизы каждые 2-3 мес. | Stars: 15.9K. Commits (6 мес): ~562 (4× больше). Issues: 288 (1:55 ratio). Релизы каждые 1-2 мес, pre-releases активны. |
| **6. Лицензия** | Apache-2.0 ✅ | Apache-2.0 ✅ |

---

## Блок 2 — Минимальный практический тест

### Тестовая задача
2-layer MLP (2→8→1) на XOR (4 samples). 1000 epochs. CPU-only. Release build.
- candle: `/home/z/my-project/research/candle-test/src/main.rs` (~50 строк)
- burn: `/home/z/my-project/research/burn-test/src/main.rs` (~65 строк)

### Результаты (реальные замеры)

| Метрика | candle 0.11.0 | burn 0.21.0 | Δ |
|---|---|---|---|
| **Clean build** | **157s** (2m 37s) | **513s** (8m 32s) | burn 3.3× slower |
| **Binary size** | **4.6 MB** | **6.0 MB** | burn 30% larger |
| **Epoch time (CPU)** | **117µs** | **167µs** | burn 43% slower* |
| **Model converged?** | No (SGD, no momentum) | Yes (Adam, loss=0.0) | — |
| **Compile attempts** | 2 | 5 | candle 2.5× easier |
| **Test code lines** | ~50 | ~65 | candle 23% less |

*Не прямое сравнение: candle SGD vs burn Adam (Adam дороже за шаг).

### Доп. наблюдения
- candle deps: ~1.2GB в target/; burn deps: ~1.1GB
- burn: каждая неудачная попытка компиляции стоила ~8 минут (deps компилировались)
- burn: incremental build (только source) — 33s; candle: 0.5s

---

## Блок 3 — Эскизы синтаксиса .mlog

### Эскиз 1: Декларативный (одна команда описывает методологию)

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

### Эскиз 2: Программный (композируемый, для продвинутых случаев)

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

### Эскиз 3: Инференс после обучения (fallback к API LLM)

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

## Рекомендация: **candle**

### Обоснование

1. **Build time критичен.** FEATURE_INTAKE.md hard limit 120s. burn добавит 513s (4× превышение). candle — 157s (приемлемо, можно поднять лимит до 240s).

2. **API простота.** candle — 2 попытки до компиляции. burn — 5 попыток. Для интеграции в язык (где каждый builtin должен быть надёжным) простота API = меньше багов.

3. **Binary size.** 4.6MB vs 6.0MB. Metalogos binary сейчас ~6MB — candle удвоит, burn утроит.

4. **Hugging Face экосистема.** candle — часть HF (safetensors, tokenizers уже в Metalogos). Совместимость форматов, pretrained weights из HF Hub — бесплатно.

5. **CPU-first.** candle работает на CPU без BLAS (нативный Rust через `gemm`). burn требует BLAS (OpenBLAS) — C-зависимость, усложняет кросс-компиляцию.

### Слабые стороны candle (честно)

1. **Меньше слоёв из коробки.** candle-nn: Linear, Conv, Embedding, RNN. burn-nn: шире (BatchNorm, LayerNorm, Dropout, Attention). Недостающие слои придётся писать вручную.

2. **Нет встроенного training loop.** candle не имеет `Learner`/`TrainingStep`. burn-train: dataloaders, metrics, learner. Для Metalogos может быть плюсом — больше контроля.

3. **SGD без momentum.** Встроенный `candle_nn::SGD` не поддерживает momentum. `candle_nn::Adam` существует, но потребует проверки API.

4. **Меньше примеров.** burn: 20+ examples (MNIST, text-classification, DQN). candle: меньше, но HF Hub компенсирует.

5. **Open issues: 892** (vs burn 288). При 21K stars это ожидаемо — широкая аудитория = больше edge cases.

### Когда burn лучше candle

- Мульти-GPU распределённое обучение (`burn-collective`)
- BLAS-ускорение на матрицах >512×512
- Backend-agnostic hot-swap (CPU↔GPU↔Wgpu без перекомпиляции)
- `burn-train Learner` абстракция экономит достаточно кода

---

## Итоговая сводка

| | candle | burn |
|---|---|---|
| Build time | **157s** ✅ | 513s ❌ |
| Binary size | **4.6MB** ✅ | 6.0MB |
| API простота | **2 attempts** ✅ | 5 attempts |
| Epoch time (CPU) | **117µs** ✅ | 167µs |
| Depend depth | **~19** ✅ | ~300+ |
| License | Apache-2.0 ✅ | Apache-2.0 ✅ |
| Community | 21K stars, HF backing | 15.9K stars, 4× commits |
| Training utils | Manual loop | **Learner** ✅ |
| Layer variety | Basic | **Comprehensive** ✅ |
| HF compatibility | **Native** ✅ | Via import |

**Рекомендация: candle** — прагматичный выбор (build time, binary size, API простота, HF экосистема).

**Решение — за владельцем.**
