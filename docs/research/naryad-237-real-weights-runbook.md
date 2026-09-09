# Наряд №237 — Real-Weights Runbook (Vision R3.7)

**Purpose:** пошаговый протокол real-weights прогона (env-gated тесты №212) на
машине владельца. Код GO-ready (№236, PR #234, CI 15/15); этот документ делает
прогон = одна сессия команд, без импровизации.
**Status:** прогон **PARKED** (решение владельца 2026-09-09 — железа нет в
доставочном окружении: 9.2 ГБ диска из требуемых ≥ 40 ГБ). Допущение владельца
«все нормально» = условный GO по коду; допущение ≠ верификация — верификация
делается ТОЛЬКО этим прогоном.
**Rule §3.8:** каждое число ниже заполняется выводом реальной команды. Слот
«REQUIRES REAL RUN» заполняется только реальным прогоном; прогон не состоялся —
слот остаётся пустым. Фабрикация чисел = провал наряда.

---

## 0. Что решает прогон (Go/No-Go)

Критерии — дословно из `docs/research/naryad-212-go-no-go.md` (Block 5.2
нарядной спецификации №212):

- Latency on CPU;
- Necessity of GPU;
- Quality acceptability (subjective — requires actual generated image).

Если env-gated прогон даёт узнаваемое изображение при приемлемой латентности —
**Go в R4** (vision grammar + dispatch, №213). Если изображение повреждено или
латентность неприемлема — **No-Go** + адресный fix-forward PR по списку R3
упрощений из go-no-go («R3 architecture status»). Решение принимает
координатор (заголовок go-no-go), не исполнитель.

## 1. Предпрогон (все пункты обязательны)

| # | Проверка | Команда / критерий |
|---|----------|--------------------|
| 1.1 | Свободный диск ≥ 40 ГБ на целевой ФС | `df -h $MLOG_VISION_WEIGHTS_DIR` — веса реально 32 848 304 654 B ≈ 32.85 GB (16 файлов, верифицировано по HF API 2026-09-09; старая оценка go-no-go «~24.6 GB» — занижение того же источника) + PNG + headroom |
| 1.2 | RAM ≥ 64 ГБ (F32-политика) | dtype-политика: `naryad-212-wedge-e2e-facts.md` §8 — F32: transformer 22.93 GB → ~46 GB RAM + Qwen3 7.49 GB → ~15 GB + VAE ~0.33 GB + активации ~30 GB → **~62 GB peak**. BF16-путь (~32 GB peak) — **R4+ территория, в этом прогоне НЕ импровизировать** |
| 1.3 | Репозиторий на зелёном коммите ≥ базы №237 | `git fetch origin main --force && git reset --hard origin/main`; CI 15/15 на этом sha |
| 1.4 | Каталог весов задан абсолютным путём | `export MLOG_VISION_WEIGHTS_DIR=/abs/path` (большая ФС с шага 1.1) |
| 1.5 | Сборка целостна | `cargo build --workspace --features vision` — success |

Ожидание по времени: CPU-прогон медленный — минуты-десятки минут на forward
pass (facts §8); закладывать часы на полную сессию (загрузка + 3 теста +
детерминизм-прогон).

## 2. Загрузка весов (fetch по checksum-дисциплине)

```bash
export MLOG_VISION_WEIGHTS_DIR=/abs/path/to/weights

tools/fetch_vision_weights.sh --dry-run   # план: URL → путь, SKIP/качать; офлайн
tools/fetch_vision_weights.sh             # загрузка ~32.85 GB; resume (curl -L -C -)
tools/fetch_vision_weights.sh             # идемпотентность: КАЖДЫЙ файл → SKIP (sha-verified)
```

Дисциплина скрипта (реализует Block 2.1 манифеста): reference = манифест-SHA
(если заполнен) → иначе HF LFS oid; расхождение → громкий отказ, файл не
потребляется; сетевой сбой → громкий ненулевой выход, «пропустили и пошли
дальше» не существует.

После загрузки:

1. Вставить напечатанные строки `| file | sha256 | bytes |` в таблицы
   `docs/research/naryad-212-weights-manifest.md` (замена оставшихся `_TODO_`).
2. Закоммитить ТОЛЬКО манифест: веса в git запрещены (§3.2). Проверка:
   `git status` не показывает ни одного `.safetensors`;
   `git ls-tree -r HEAD --name-only | grep -ciE 'safetensors|\.ckpt$|\.pth$|\.gguf$'` = 0.

## 3. Прогон env-gated тестов (точные команды №212)

```bash
export MLOG_VISION_OUT=/abs/path/to/out    # опционально; default target/
cargo test --features vision --test naryad_212_wedge_e2e -- --nocapture 2>&1 | tee n237_run1.log
```

Без `MLOG_VISION_WEIGHTS_DIR` эти тесты громко SKIP-ят; с заданным каталогом —
должны выполнить все три:

- `text_encoder_real_weights_forward` — реальный Qwen3-4B forward (3 шарда, ~7.5 GB) → `[seq, 2560]`;
- `vae_real_weights_decode_fixed_latent` — реальный VAE decode (167 MB) → PNG 1024×1024 (`n212_vae_fixed_latent.png`);
- `clinical_e2e_first_image` — полный клин: prompt → tokens → Qwen3 → DiT 8 forward → VAE → PNG 1024×1024 (`first_image.png`, seed 21200, prompt «a red apple on a wooden table, studio light»).

CI-видимые tiny goldens (VAE `85ef6a87…`, DiT `e686167b…`) в этом прогоне
неизменны — их зелень уже в CI; если они вдруг красные — СТОП, фиксация среды,
никаких пинов не трогать (§3.2).

## 4. Фиксация результата (заполнение слотов «REQUIRES REAL RUN»)

Слоты — в `docs/research/naryad-212-go-no-go.md`, секция «Verbatim DoD entries».
Заполнять дословно из вывода и файловой системы:

| Слот | Источник |
|------|----------|
| PNG path | stdout теста (`clinical_e2e_first_image: PNG path=…`) |
| PNG SHA-256 | `sha256sum $MLOG_VISION_OUT/first_image.png` |
| PNG size | `wc -c $MLOG_VISION_OUT/first_image.png` |
| Timings (tokenize / encode / sampler / decode) | stderr теста (`clinical_e2e: tokenize: … / encode … / sampler … / decode …`) — копировать дословно |
| Determinism (2 runs bit-exact) | второй прогон того же теста + сравнение PNG SHA |
| Hardware | `nproc`; `free -g`; `uname -r`; наличие GPU (CPU-путь — none) |

Детерминизм-прогон:

```bash
cargo test --features vision --test naryad_212_wedge_e2e clinical_e2e_first_image -- --nocapture 2>&1 | tee n237_run2.log
sha256sum $MLOG_VISION_OUT/first_image.png   # должен совпасть с run1 бит-в-бит
```

Если прогон не дошёл до какого-то слота — слот остаётся пустым (`<REQUIRES
REAL RUN>`), с loud-примечанием, на каком шаге остановились и почему.

## 5. Субъективный качественный гейт

Открыть `first_image.png`. Честно ответить: изображение узнаваемо (красное
яблоко на деревянном столе)? Классификация: шум / структура без объекта /
узнаваемый объект. Это вход в критерий «Quality acceptability» — решает
координатор, исполнитель фиксирует наблюдение.

## 6. Инварианты во время прогона

- `src/**` и `tests/**` не трогаются (код GO-ready по №236). Обнаруженный
  дефект → отдельный fix-forward наряд, не ad-hoc правка посреди прогона.
- Формула игноров (truth-up №237 Block 2.3 — «96/0» невоспроизводимо):
  `git grep -c '#\[ignore' HEAD -- src tests` = **129** на базе `1f26f41`/`396b1df`
  (125 tests + 4 src); инвариант = «N вписать фактическим числом, дельта к базе = 0».
- Веса не в git: `git ls-tree -r HEAD --name-only | grep -ciE 'safetensors|\.ckpt$|\.pth$|\.gguf$'` = 0.
- Golden'ы не пере-пинятся: VAE `85ef6a879d58…`, DiT `e686167b2e82…` — константы тестов.

## 7. Отчёт

Прогон оформляется ОТДЕЛЬНЫМ отчётом владельца машины (по этому runbook), не
коммитами наряда №237 (§3.5): заполненные слоты go-no-go + PNG-метаданные +
тайминги + вывод Go/No-Go координатора. Наряд №237 поставляет инструмент и
протокол; прогон — следующая единица работы.
