---
name: metalogos-language-semantics
description: >
  Decide the precise, operational semantics of Metalogos' hard features instead of hand-waving.
  USE THIS SKILL whenever you design or implement Fluid Types, confidence propagation, the rule
  engine (priority/conflict resolution), semantic memory (decay/recall), learnable patterns, or
  the adapt/self-modification system — or whenever you are tempted to invent behavior that prior
  art has already solved. Trigger it before writing an ADR, before choosing how a feature behaves,
  and any time the README's description of a feature is too vague to compile against. It maps each
  feature to the field that already solved it and gives a recommended starting semantics.
---

# Metalogos — решения по семантике (стой на плечах гигантов)

**Правило:** для каждой подсистемы — прочитай prior art, выбери **простейшую защитимую** семантику,
запиши ADR (`docs/adr/NNNN-*.md`: контекст / варианты / выбор / на чём основан), сделай её тестируемой.
Никогда не закладывай поведение «как-нибудь» — у каждой фичи ниже есть область, которая её уже решила.

---

## Fluid Types — когда коллапсирует суперпозиция?

`Fluid<Number[0.8], Text[0.2]>` бессмысленен, пока не определено, **когда значение становится конкретным**.
- **Prior art:** gradual typing (Siek–Taha), refinement/liquid types, размеченные объединения (sum types),
  probabilistic types.
- **Старт:** `Fluid` = размеченное объединение вариантов + вектор уверенностей. Коллапс **ленивый, в точке
  использования**: операция/правило/ветвь, требующая конкретного типа, форсирует выбор варианта с максимальной
  уверенностью (или ошибку soft-failure, если ниже порога). Документируй порог.
- **Не делай:** «настоящую» вероятностную типизацию со сложным выводом на старте — это research.

## Распространение confidence через паттерны

Если у входа уверенность 0.7, какова уверенность выхода `pattern`?
- **Prior art:** Markov Logic Networks (Richardson–Domingos), ProbLog, Dempster–Shafer, fuzzy logic.
- **Старт:** простое и честное правило — выход несёт `min` (или произведение) уверенностей входов; для
  `learnable` — уверенность из ответа модели (logprob/самооценка). **Документируй, что это эвристика, а не
  вероятностно-корректный вывод.** Не заявляй математическую строгость, которой нет.

## Движок правил — приоритет и разрешение конфликтов

`rule If(...) then ... with priority=N` — что если сработали несколько противоречивых правил?
- **Prior art:** продукционные системы и алгоритм Rete, Datalog (фиксточка), MLN (взвешенные правила).
- **Старт:** правила сортируются по `priority` (выше = раньше); конфликты — выигрывает высший приоритет,
  при равенстве — порядок объявления; вероятностные правила (`then x = 0.85`) пишут значение-уверенность,
  а не булеву истину. Зафиксируй стратегию как «priority-ordered, first-wins» в ADR.
- **Рост:** forward-chaining до фикс-точки, затем (фаза 2) взвешенный вывод в духе MLN.

## Семантическая память — затухание и recall

- **Prior art:** vector DB (qdrant/milvus) + decay-функции; активация памяти в ACT-R (база-уровень + спад);
  temporal/knowledge graphs.
- **Старт:** in-memory store записей `{ключ, значение, ts, priority, confidence}`; затухание —
  `activation = priority * exp(-rate * age)`; `recall` — по строковому/эмбеддинг-сходству выше
  `min_confidence`; `forget after T` и `decay rate=r` уменьшают активацию/удаляют. Персист — `serde`.
- **Рост:** эмбеддинги + векторный индекс (M4→фаза 2), граф знаний для `semantic`-памяти (neo4j/встроенный).

## Learnable patterns — стадии реализации

- **Prior art:** in-context learning; DSPy (программирование LLM как модулей); LMQL/Guidance/Outlines
  (структурированный/ограниченный вывод).
- **Стадия 1 (M3):** паттерн = промпт к LLM; результат парсится в объявленный тип; ошибка парсинга →
  soft-failure с низкой уверенностью.
- **Стадия 2:** накопление few-shot примеров (`adapt ... with new_example`) и подстановка их в промпт.
- **Стадия 3 (фаза 2):** дообучение/локальная модель через PyO3+PyTorch, экспорт в ONNX для рантайма.

## Adapt / самомодификация — безопасная форма

- **Prior art:** program synthesis и genetic programming — **это research-ямы, помечай и откладывай**.
- **Старт (M5):** `adapt`/`mutate` меняют **только** few-shot набор `learnable`-паттерна (in-context),
  не переписывают произвольный код. Каждая мутация: (1) применяется в **sandbox** (allow/forbid/timeout);
  (2) прогоняет тест-сьют; (3) **откатывается**, если метрика упала ниже порога (`rollback_if`).
- **Инвариант безопасности:** правила, помеченные как safety-critical, мутации трогать не могут. Это
  проверяется до применения мутации, а не после.

---

## Soft-failure вместо исключений

«Градиентный откат» из README = вычисление не падает с краш-исключением, а возвращает результат с **низкой
уверенностью** и помечает деградацию, чтобы `flow`/`rule` могли среагировать (ветка low-confidence, эскалация
человеку). Реализуй как тип-результат с полем уверенности и причиной, а не как панику рантайма.

## Эвристика выбора

Перед любым решением спроси себя: «это уже решено в области из списка?» Если да — возьми простейший вариант
оттуда и сошлись на него в ADR. Если нет (например, полноценный вероятностный вывод типов или genetic adapt) —
пометь как research, сделай заглушку-MVP, не блокируй лестницу постройки.
