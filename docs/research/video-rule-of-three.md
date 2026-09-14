# Rule of Three — General Denoiser Assessment (Наряд №308, issue #388)

> **Дата:** 2026-09-14. **База:** main `8768de2`. **Статус:** факты зафиксированы, вердикт — отложен до Voice-A2.

## 1. Контекст

`euler_step` — общий ODE-примитив в `src/vision/sampler.rs`. Полный контур сэмплирования (`flow_match_euler_sample`) привязан к `ZImageTransformer` (Vision). Video — третий конкретный потребитель ODE-примитива. Правило трёх: извлечение общего `Denoiser` интерфейса рассматривается после того, как три конкретных контура доказаны.

## 2. Три контура (текущее состояние)

| Столп | ODE-примитив | Денойзер | Сэмплирование | Статус |
|---|---|---|---|---|
| Vision | `euler_step` (shared) | `ZImageTransformer` (vision-specific) | `flow_match_euler_sample` (vision-specific) | ✅ Real (tiny golden, CI green) |
| Voice | `euler_step` (shared, planned) | TBD (Voice-A2 не опубликован) | TBD | ❌ Не начат |
| Video | `euler_step` (shared, planned) | `VideoDit` (stub, №308) | `flow_match_euler_sample_video` (stub, №308) | ⏳ Skeleton |

## 3. Доля переиспользуемого кода

| Компонент | Vision | Video (stub) | Переиспользование |
|---|---|---|---|
| `euler_step` | ✅ | ✅ (planned) | **100%** — идентичный ODE-шаг |
| Flow matching loop | Vision-specific (`ZImageTransformer`) | Video-specific (`VideoDit`) | **~30%** — структура цикла та же, модель разная |
| VAE | `VisionVae` (2D conv) | `VideoVae` (3D conv, stub) | **~10%** — 2D vs 3D, разная архитектура |
| Attention | 2D spatial | 3D spatiotemporal + causal | **~20%** — общий паттерн, разная реализация |
| Positional encoding | 2D RoPE | 3D RoPE (temporal + spatial) | **~40%** — общий принцип, разная размерность |

**Общий знаменатель**: только `euler_step` (ODE-примитив) — 100% переиспользование. Остальное — 10-40% — слишком мало для извлечения общего интерфейса без утечки специфики.

## 4. Утечка видео-специфики

Извлечение общего `Denoiser` интерфейса потребует:
- temporal-оси (Video: 3D, Vision: 2D) → общий интерфейс должен поддерживать N-мерные входы
- causal attention (Video: frame i attends to 0..=i; Vision: нет) → флаг causal в интерфейсе
- 3D positional encoding → общий интерфейс должен поддерживать 2D и 3D

**Оценка**: общий интерфейс будет либо слишком общим (dyn Any, потеря типизации), либо загрязнённым видео-спецификой (temporal/causal флаги в Vision-коде). Ни один вариант не приемлем.

## 5. Выразимость MoE-денойзеров

Wan 2.2 A14B — MoE-архитектура (mixture of experts). Общий `Denoiser` интерфейс должен:
- Поддерживать dense и MoE модели
- Не ломаться при добавлении expert-routing
- Не требовать от Vision/Voice знания о MoE

**Оценка**: MoE можно выразить через trait object (`dyn Denoiser`), но это потеря типизации + overhead. До реальной MoE-модели в Voice/Video — преждевременно.

## 6. Вердикт

**Отложен до Voice-A2.**

Факты:
- `euler_step` — 100% переиспользование (уже shared).
- Полный контур — 10-40% переиспользование — ниже порога для извлечения.
- Утечка видео-специфики (temporal, causal, 3D) — неприемлема.
- MoE-выразимость — преждевременна.

Протокол завершения:
1. Voice-A2 публикует свой денойзер → три контура доказаны.
2. Если Voice-A2 покажет ≥50% переиспользование с Vision/Video → ADR на `Denoiser` trait.
3. Если <50% → статус-кво: `euler_step` shared, контуры отдельные.

## 7. Инженерный Go/No-Go

**Инженерный Go** — контур на tiny-весах, контракты, CI (этот наряд, №308):
- VAE contract: 3D conv encode/decode, latent shape [B, 16, T/4, H/8, W/8].
- DiT contract: noisy latent + timestep + text → clean latent.
- Sampler: euler_step (shared) + VideoDit (stub).
- CI: skeleton tests green, no real weights needed.

**Качественный Go** — RTF/качество на реальных весах — отдельный наряд V4, по preflight runbook-паттерну №237.

Текущий статус: **инженерный Go** (skeleton + contracts). **Качественный No-Go** (нет железа, №294).
