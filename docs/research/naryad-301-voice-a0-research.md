# Voice Pillar — Research A0 (Наряд №301, issue #369)

> **Дата:** 2026-09-14. **База:** main `b0dcd9a` (post-№300 merge). **Статус:** выполнено и проверено владельцем.

## 1. Цель исследования

Определить scope, wedge-модель, value-registry pattern, и security-gates для Voice-пиллара Metalogos до любой реализации кода. Дисциплина: ADR до кода (как Reflex/Vision).

## 2. Существующая инфраструктура (переиспользование)

| Компонент | Источник | Переиспользование для Voice |
|---|---|---|
| `euler_step` (ODE-примитив) | `src/vision/sampler.rs` | Обобщён, переиспользуем для flow-matching TTS |
| `flow_match_euler_sample` | `src/vision/sampler.rs` | Жёстко привязан к `ZImageTransformer` — извлечение общего денойзера НЕ предполагать заранее (rule of three; проверка — фаза A2) |
| `encode_png` / `decode_png` | `src/vision/vae.rs` | Аналог: `encode_wav` / `decode_wav` (новый, не дублировать) |
| `VisionRegistry` / `VisionId` | `src/vision/mod.rs` (ADR-0124) | Лекало: `VoiceRegistry` / `AudioId` (ADR-0144) |
| `MODEL_WEIGHTS_UNSAFE` gate | `src/audit.rs` (№241/№300) | Обобщён в №300 — `voice_fetch_weights` покрыт suffix convention |
| `vision_fetch_weights` runtime | `src/builtins/vision.rs` (SSRF guard, allowlist, SHA pinning) | Лекало для `voice_fetch_weights` runtime layers |
| `whisper_transcribe` | `src/builtins/llm.rs` (№279) | Переиспользуется для ASR-сверки в consent-ритуале |
| `secret()` / AES-256-GCM | Наряд №172 | Переиспользуется для шифрования voiceprint at rest |
| `canary_insert` / `canary_check` | Наряд №284 | Лекало для path-sensitive consent-проверки |

## 3. Таблица клинов (срез сентябрь 2026)

| Модель | Код / Вес | Размер | Клонирование | Watermark | Статус |
|---|---|---|---|---|---|
| **Chatterbox Multilingual V3** | MIT / MIT | 500M | ✅ | ✅ default | **Рекомендован** (ADR-0146) |
| **Kokoro-82M** | Apache-2.0 / Apache-2.0 | 82M | ❌ | ? | **Разогрев A2** (ADR-0146) |
| CosyVoice 3 | Apache-2.0 / Apache-2.0 | 0.5B | ✅ | ? | Future — не проверены ru/be, языки |
| NeuTTS Air | Apache-2.0 / Apache-2.0 | 748M | ✅ | ? | Future |
| GPT-SoVITS | MIT / MIT | ? | ✅ | ? | Future |
| F5-TTS | MIT / CC-BY-NC | ? | ? | ? | Research-profile only |
| Seed-VC | GPL-3.0 | ? | VC (не клон) | ? | **Исключён** (GPL contamination) |

## 4. Линейные/affine-типы и ConsentToken

В языке отсутствуют линейные/affine-типы (Option/Result отвергнуты ADR-0106). ConsentToken — **не тип**, а статическая проверка использования-один-раз через расширение `src/audit.rs` (тот же класс механизма, что `SECRET_LEAK`/`SQL_DYNAMIC`). Path-sensitive: `if consent.ok { voice_enroll(...) }` — enrollment только в then-ветке. Подробнее: ADR-0145 D1.

## 5. Словарь границ

Слово «tripwire» в ADR-0125 отсутствует — границу anti-spoof формулировать в реальном словаре проекта: «MVP-детектор, не адверсариальная гарантия» (ADR-0145 D3).

## 6. Лицензионный паттерн ниши

Permissive-код + non-commercial-веса — пример k2-fsa/OmniVoice «VoiceStudio»: Apache-2.0 код, CC-BY-NC веса. Лицензию весов проверять отдельно от лицензии кода для каждого кандидата. Chatterbox и Kokoro — оба fully permissive (MIT/MIT и Apache-2.0/Apache-2.0 соответственно).

## 7. Cross-pillar сводка

| Элемент | Vision (ADR-0122/0124/0125) | Voice (ADR-0143/0144/0145/0146) | Cross-pillar решение |
|---|---|---|---|
| `MODEL_WEIGHTS_UNSAFE` | №241 (vision_fetch_weights) | №300 (suffix `_fetch_weights`) | **Shared** — №300 generalized |
| `generative-declaration` grammar | `vision { }` (STRING name) | `voice { }` (STRING name) | Shared pattern, separate declarations |
| Registry license fields | Per-entry | Per-entry | Same structure (ADR-0144 D3) |
| Opaque handle | `Value::Vision(VisionId)` | `Value::Audio(AudioId)` | Separate variants, same pattern (ADR-0114) |
| Provenance gates | ADR-0125 (5 gates) | ADR-0145 (5 gates, 1 shared) | Separate gates, 1 shared (`MODEL_WEIGHTS_UNSAFE`) |
| Watermark | LSB (MLGV + 32-bit model hash) | TBD (AudioSeal-class — research backlog) | Different watermark class for audio |

Cross-pillar ADR не создаётся — каждый столп имеет свой scope/wedge/registry/gates ADR. Общий элемент (`MODEL_WEIGHTS_UNSAFE`) уже зафиксирован в №300. Если Vision-план ещё не принят — зафиксировано обязательство свести при его принятии.

## 8. Резюме ADRs

| ADR | Название | Status |
|---|---|---|
| 0143 | Voice scope — TTS, zero-shot cloning, voice-design | Accepted |
| 0144 | Voice value-registry — Value::Audio(AudioId) + VoiceRegistry | Accepted |
| 0145 | Voice security gates — consent, provenance, privacy, taint | Accepted |
| 0146 | Voice wedge — Chatterbox Multilingual V3 + Kokoro-82M | Accepted |
