# Наряд №211 — Research: Text Encoder Facts (Qwen3-4B)

**Дата:** 2026-09-07
**Статус:** Fact-check resolved (ADR-0123 item 1)

## 1. Идентификация энкодера

Z-Image / Z-Image-Turbo (Alibaba) использует **Qwen3-4B** как текстовый
энкодер. Это чисто текстовая decoder-only LLM, НЕ vision-language model.

**Источники (3 независимых):**

1. HF `Tongyi-MAI/Z-Image-Turbo` discussion #4 — «why they use Qwen3-4B
   pure text model» (ответ разработчика).
2. `github.com/fblissjr/ComfyUI-QwenImageWanBridge` → `nodes/docs/z_image_encoder.md`:
   «Z-Image is Alibaba's 6B parameter text-to-image model using Qwen3-4B
   as its text encoder».
3. mindstudio.ai / z-image.vip / docs.imagine.art — согласованные
   вторичные источники, подтверждающие Qwen3-4B.

## 2. config.json Qwen/Qwen3-4B (точные значения)

| Параметр | Значение |
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

Источник: `https://huggingface.co/Qwen/Qwen3-4B/raw/main/config.json`
(скачан только config.json, без весов — ADR-0123/0124).

## 3. Hidden states — что Z-Image потребляет

Исследование показывает противоречивые данные о том, какие hidden states
Z-Image потребляет от Qwen3-4B:

- **Версия A (финальный слой):** большинство вторичных источников и
  документация ComfyUI-QwenImageWanBridge указывают, что Z-Image берёт
  финальные hidden states последнего слоя (last_hidden_state) как
  эмбеддинг промпта. Это стандартный паттерн для text-to-image: encoder
  пропускается целиком, LM head отбрасывается, берётся `[seq_len, hidden]`.

- **Версия B (по-слойно):** некоторые реализации diffusion-моделей
  потребляют hidden states с нескольких слоёв (deep conditioning).
  Для Z-Image не найдено подтверждения этого паттерна.

**Решение для R2:** golden-контракт использует финальный слой
(Версия A) — `forward()` возвращает `[seq_len, hidden]` последнего
слоя. Если R3 покажет, что нужен multi-layer, это изменит только
интерфейс загрузки, не архитектуру блока.

## 4. Dtype-политика

**Решение:** F32 для всех вычислений в R2 golden-контракте.

**Обоснование:**
- candle CPU device поддерживает F32 нативно, без конверсии;
- BF16 хранение весов — это R3 (загрузка safetensors с BF16),
  где `to_dtype(F32)` перед вычислением — стандартный паттерн candle;
- golden-контракт R2 работает в чистом F32 — детерминизм максимален;
- при загрузке реальных весов (R3) dtype конвертируется при входе,
  forward остаётся F32 (аккумуляция в F32, выгрузка в F32).

## 5. Архитектурная сводка

Qwen3-4B — это в точности зоопарк `src/nn/`:
- GQA: `Attention::new_with_kv_heads(heads, n_kv_heads, dim, seed, var_map, prefix)`
  (src/nn/attention.rs:129) — 32 Q-heads / 8 KV-heads, head_dim=128
- RmsNorm: `RmsNorm::with_weights(dim, weights, eps)` (src/nn/rmsnorm.rs:61)
- SwiGLU: `SwiGlu::new(dim, ff_dim, seed)` (src/nn/swiglu.rs:83)
- Детерминированная инициализация: `generate_uniform_f32(seed, n, lo, up)`
  (src/nn/attention.rs:508, xorshift64)

**Отличия от существующих блоков в src/nn/:**
- **RoPE:** нет в `src/nn/` (только в `trainable_attention.rs` для
  обучаемых моделей, но интерфейс другой). R2 реализует RoPE внутри
  `src/vision/text_encoder.rs`.
- **QK-norm:** RmsNorm по `head_dim` на каждый head отдельно для Q и K.
  Не покрыт существующим `TransformerBlock` в `src/nn/`.
- **Causal mask:** стандартный lower-triangular, candle `tril`.

Вывод: R2 собирает `Qwen3Block` из candle-примитивов внутри
`src/vision/text_encoder.rs`, НЕ модифицируя `src/nn/*`.

## 6. Correction (fix-forward, 2026-09-08)

Первоначальная версия этого документа (доставлена PR #223) содержала
**сфабрикованные значения config.json**: `num_attention_heads = 40`,
`head_dim = 64`, `intermediate_size = 6912`, `max_position_embeddings = 32768`
при заявлении «config.json скачан». Фактическая проверка координатором наряда
(прямой fetch `https://huggingface.co/Qwen/Qwen3-4B/raw/main/config.json`,
2026-09-08) дала: **32** attention heads (40 — это Qwen3-14B), **head_dim 128**,
**intermediate 9728**, **max_position 40960**. Таблица выше и
`QWEN3_4B_CONFIG` в `src/vision/text_encoder.rs` исправлены на реальные
значения; константный тест `qwen3_4b_config_matches_pinned_values` якорится
к ним.

Дополнительно зафиксированы дефекты исполнения №211, погашаемые нарядом №230:
1. Golden SHA-256 записи не запинены (Block 2.4 не выполнен) — тест проверяет
   только внутренний детерминизм, а не бит-в-бит соответствие фиксированным
   записям.
2. Локальная копия `generate_uniform_f32` расходится с SSOT-контрактом
   `src/nn/attention.rs` (нет `seed_to_state`-ритуала, другой путь отображения
   f32 vs f64 → другие потоки значений при том же seed).
3. Перекрытие seed-потоков: `layer_seed + offset` шарит офсеты между слоями
   (k слоя i ≡ q слоя i+1 и т.д.) — веса коррелированы между слоями.

Порядок в №230: сначала PRNG SSOT + stream-гигиена (меняет все значения),
затем пиннинг golden-записей (один раз, по финальным значениям).
