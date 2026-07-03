# НАРЯД — Metalogos: 4 языковые примитивы методом обратной итерации

**Дата:** 2026-06-22
**Адресат:** агент-строитель Metalogos
**Метод:** обратная итерация — задача существует (Fosved Office V2), язык достраивается под неё как примитив, не как обходной путь
**Устав:** METALOGOS_INSTRUCTIONS.md v3 — контракт-первым, ADR на решение, sandbox для adapt/LLM, все три бэкенда, маленькие коммиты, не ломай зелёное

---

## ПОЧЕМУ ЭТИ ЧЕТЫРЕ

Пять противоречий были найдены в реальной архитектуре Fosved V2. Одно (FORMAT_INSTRUCTION конфликт) — не языковая проблема, а конфигурационная: она решится сама, как только Проблема A закрыта (см. §5). Остальные четыре — реальные дыры в языке: каждая уже сейчас требует костыля в Python или в mlog-обвязке, и каждая будет требовать нового костыля при следующем отделе с похожей потребностью. Строим примитив один раз — используем везде.

| # | Проблема сейчас | Костыль | Что после языковой примитивы |
|---|---|---|---|
| A | `SelectSkillsByKeywords` — substring-матч, бюджет 2000 симв., режет скилл на 500 симв. | Ручной budget-hack в mlog | Настоящая tier-структура как языковая конструкция |
| B | Формула P = A×R×S×L×E×E_mod для списка акторов | Либо строковая генерация промпта (LLM считает сам, ненадёжно), либо вынос в Python | Нативная агрегация над списком структур в mlog |
| C | Нет таблиц research_sources/analysis/yana_events — упомянуты в коде, не в init_db | Python init_db как единственный источник схемы | Schema-as-code прямо в .mlog отдела |
| D | web_app_data / Mini App action bar заблокированы без пересборки Rust | Функциональность просто недоступна | `Hook`-декларация (уже заявлена в 24 видах объявлений, но не подтверждена рабочей) как источник маршрутизации callback'ов |

---

## PROBLEM A — Tiered Skill Index как языковая примитива

### Диагноз текущего костыля

```
SelectSkillsByKeywords(dept, query):
  index = read_file("skills_index/<dept>.txt")  // плоский список имён
  для каждого skill в index:
    если len(result) < 2000:                     // жёсткий бюджет
      content = read_file("skills/<skill>/SKILL.md")
      если content содержит lower(query[0:50]):   // ненадёжный матч
        result += skill content[0:500]            // ОБРЕЗАННЫЙ скилл
```

Три отдельных дефекта в одном месте: (1) нет понятия tier — Tier 1 (must always load) неотличим от Tier 3 (special), (2) матч по первым 50 символам запроса и первым 500 символам скилла — оба обрезания случайны относительно смысла, (3) бюджет считается в символах текста, не в токенах и не в целых единицах — скилл режется посередине предложения.

### Prior art

DSPy (module composition with explicit signatures), Rete (production rule matching — конкретно для условия загрузки по паттерну, не substring), LMQL/Guidance (constrained selection). Смотреть скилл `metalogos-language-semantics` — там уже есть карта на ProbLog/Rete для движка правил, применимо напрямую к «когда грузить скилл».

### Контракт

**`examples/skill_index_tiered.mlog`:**

```mlog
// Декларация индекса скиллов отдела — заменяет skills_index/<dept>.txt
skill_index osp {
  tier 1 always [
    "deconstruct", "awareness-frame", "root-cause-mapping",
    "forces-and-potentials", "strategic-maneuvers", "two-paths-synthesis",
    "decision-tree-forecast", "reflexive-impact", "calibration",
    "system-health-diagnostics"
  ]
  tier 2 when_matches [
    { skill: "cross-asset-divergence", triggers: ["рынок", "актив", "валют", "акци"] },
    { skill: "deep-profiling",         triggers: ["персона", "лидер", "исчез"] },
    { skill: "narrative-vs-flows",     triggers: ["нарратив", "пропаганд", "сми"] },
    { skill: "source-triangulation",   triggers: ["источник", "проверь", "правда ли"] }
  ]
  tier 3 when_matches [
    { skill: "red-team",       triggers: ["контр-анализ", "redteam", "слеп"] },
    { skill: "order-of-battle", triggers: ["война", "армия", "фронт"] }
  ]
  budget: 25000 tokens
  truncation: whole_skill_only  // никогда не режет скилл посередине
}

pattern LoadSkills(dept: String, query: String) -> List {
  let idx = resolve_skill_index(dept)
  let selected = []
  each s in idx.tier1 {
    let selected = list_append(selected, s)
  }
  each rule in idx.tier2 {
    if matches_any(query, rule.triggers) {
      let selected = list_append(selected, rule.skill)
    }
  }
  each rule in idx.tier3 {
    if matches_any(query, rule.triggers) {
      let selected = list_append(selected, rule.skill)
    }
  }
  return fit_to_budget(selected, idx.budget, idx.truncation)
}

flow Main {
  input: String = "проанализируй рынок нефти на фоне исчезновения главы ЦБ"
  let skills = LoadSkills("osp", input)
  return skills
}
```

**`examples/skill_index_tiered.expected`:**

```
["deconstruct","awareness-frame","root-cause-mapping","forces-and-potentials","strategic-maneuvers","two-paths-synthesis","decision-tree-forecast","reflexive-impact","calibration","system-health-diagnostics","cross-asset-divergence","deep-profiling"]
```

### Что реализовать

1. Новый вид объявления `skill_index <dept> { ... }` — парсер + AST-нода (расширяет список из 24 видов объявлений §4 устава, требует ADR на добавление 25-го)
2. `tier N always [...]` — список, грузится безусловно
3. `tier N when_matches [...]` — список объектов `{skill, triggers}`, `matches_any(text, triggers)` — новый builtin, case-insensitive substring по списку, но с явной семантикой (не скрытый как сейчас)
4. `budget: N tokens` — понятие токенов, не символов. Нужен builtin `estimate_tokens(text)` (эвристика: `len(text) / 4` для латиницы/кириллицы допустима как MVP, ADR фиксирует эвристику явно как временную)
5. `truncation: whole_skill_only` — при превышении бюджета отбрасываются целые скиллы с конца списка (по приоритету tier, затем по порядку объявления), никогда не режется середина файла
6. `resolve_skill_index(dept)` — builtin, читает декларацию `skill_index` для отдела (заменяет `read_file("skills_index/<dept>.txt")`)
7. `fit_to_budget(list, budget, mode)` — builtin, применяет бюджет к списку выбранных скиллов, реально читая файлы и суммируя токены

### Definition of Done

- Golden-тест `skill_index_tiered.mlog` проходит на всех трёх бэкендах (tree-walking, bytecode, JIT)
- ADR `docs/adr/00XX-tiered-skill-index.md`: обоснование замены substring-only на structured tiers, альтернативы рассмотрены (embedding-based similarity — отклонено как overkill для MVP, revisit после Phase 8)
- Обратная совместимость: если `skill_index` для отдела не объявлен — fallback на старое поведение с warning в лог (не ломает существующие 11 отделов без миграции)

---

## PROBLEM B — Агрегация над списком структур (формула потенциала)

### Диагноз

Формула `P = A×R×S×L×E×E_mod` должна считаться для списка акторов (обычно 2-5). Сейчас нет надёжного способа: (1) хранить список структур с именованными float-полями, (2) применить арифметику к каждому элементу, (3) собрать результат обратно в структурированный JSON для вставки в промпт LLM.

### Prior art

Это стандартная map/reduce операция — нужен строгий, а не вероятностный примитив (в отличие от `learnable pattern`, здесь детерминированная арифметика, LLM не участвует). Смотреть на списочные comprehensions Python/Rust iterator chains как референс синтаксиса, но не тянуть за собой их полную мощность — Metalogos уже имеет `each` и `EachWithIndex`, нужно расширение, не новая парадигма.

### Контракт

**`examples/actor_potential.mlog`:**

```mlog
struct Actor {
  name: String
  A: Float  // active capacity
  R: Float  // resources
  S: Float  // situation favorability
  L: Float  // latent reserves
  E: Float  // environmental fit
  E_mod: Float  // environmental modifier
}

entity actors: List<Actor> = [
  { name: "РФ",      A: 0.6, R: 0.7, S: 0.5, L: 0.4, E: 0.5, E_mod: 0.9 },
  { name: "Украина", A: 0.7, R: 0.4, S: 0.6, L: 0.6, E: 0.7, E_mod: 1.2 },
]

pattern ComputePotential(a: Actor) -> Float {
  return a.A * a.R * a.S * a.L * a.E * a.E_mod
}

pattern RankActors(list: List<Actor>) -> List {
  let scored = map(list, ComputePotential)
  let paired = zip(list, scored)
  return sort_by(paired, descending=true, key=1)
}

flow Main {
  let ranked = RankActors(actors)
  let top = ranked[0]
  let differential = ranked[0][1] - ranked[1][1]
  return "Топ актор: " + top[0].name + ", P=" + to_string(top[1]) + ", D=" + to_string(differential)
}
```

**`examples/actor_potential.expected`:**

```
Топ актор: Украина, P=0.14108, D=0.020808
```

(проверить расчёт вручную при реализации — контракт фиксирует формат вывода, конкретные числа получить прогоном)

### Что реализовать

1. `struct <Name> { field: Type ... }` — если ещё не полноценно поддержан для пользовательских типов (в списке 14 expressions есть `StructLit`, но декларация именованного struct-типа отдельно от entity — уточнить в ADR, возможно уже есть через `EntityType`)
2. `map(list, pattern)` — builtin, применяет чистый pattern к каждому элементу
3. `zip(list_a, list_b)` — builtin, попарное объединение двух списков в список пар/кортежей
4. `sort_by(list, descending, key)` — builtin, сортировка списка структур/кортежей по индексу или полю
5. Индексация кортежей через `[0]`, `[1]` — расширение `IndexAccess` на не-List типы (кортеж как продукт двух типов)

### Definition of Done

- Golden-тест проходит на всех трёх бэкендах
- ADR фиксирует: является ли кортеж (result of `zip`) новым 14-м типом данных или синтаксическим сахаром над `List` с гетерогенными элементами (выбор влияет на type checker)
- Explicit test: список из 5 акторов, ранжирование, дифференциал между топ-2 — покрывает реальный кейс ОСП Phase 3a

---

## PROBLEM C — Schema-as-code для архива отделов

### Диагноз

`research_sources`, `analysis`, `yana_events` упомянуты в коде app.mlog, но отсутствуют в `init_db()` на Python-стороне. Единственный источник схемы БД — внешний Python файл, не связанный с декларацией отдела в mlog. При создании нового отдела (Кузница, `/recruit`) нет способа объявить архивную таблицу **вместе** с профилем и скиллами — их создание рассинхронизировано между языками.

### Prior art

Rails migrations (schema as versioned code), Prisma schema (то, что использовалось в V1 архитектуре до перехода на mlog — прямая параллель, полезно перенести идею декларативной схемы), sqlx compile-time query checking (уже в стеке Phase 6 согласно §7 устава).

### Контракт

**`examples/dept_schema.mlog`:**

```mlog
// Декларация архивной схемы прямо в файле отдела — заменяет внешний init_db()
schema osp_analysis {
  table analysis {
    id: Int primary_key auto_increment
    topic: String
    full_result: Text
    status: String default("drafted")  // drafted|published|watching|verified|missed|partial|superseded
    verification_date: DateTime nullable
    confidence: String  // low|medium|high
    created_at: DateTime default(now())
  }
  table research_source {
    id: Int primary_key auto_increment
    analysis_id: Int references(analysis.id)
    url: String nullable
    title: String
    tier: String default("secondary")  // primary|secondary|tertiary
    claim: Text
    created_at: DateTime default(now())
  }
}

pattern SaveAnalysis(topic: String, result: String) -> Int {
  let id = db_insert("analysis", { topic: topic, full_result: result, status: "drafted" })
  return id
}

pattern AttachSource(analysis_id: Int, title: String, claim: String) -> Bool {
  db_insert("research_source", { analysis_id: analysis_id, title: title, claim: claim })
  return true
}

flow Main {
  let id = SaveAnalysis("Топливный кризис РФ", "полный текст анализа...")
  AttachSource(id, "Bloomberg НПЗ атаки", "38 атак янв-май")
  let saved = query("SELECT * FROM analysis WHERE id = " + to_string(id))
  return saved[0]["topic"]
}
```

**`examples/dept_schema.expected`:**

```
Топливный кризис РФ
```

### Что реализовать

1. `schema <name> { table <name> { field: Type modifiers... } }` — новый вид объявления (расширяет `Db`, который уже в списке 24 — уточнить, был ли `Db` спроектирован под это или под что-то более узкое, зафиксировать в ADR)
2. Модификаторы полей: `primary_key`, `auto_increment`, `nullable`, `default(...)`, `references(table.field)`
3. Автоматическая миграция при старте: `mlog serve` при обнаружении новой/изменённой `schema` декларации — создаёт/обновляет таблицу через `db_execute` (additive only, без drop — прямая параллель с правилом "Prisma миграции — additive" из старой методологии Fosved, тот же принцип переносится)
4. `db_insert(table, {field: value, ...})` — builtin, параметризованный INSERT (безопасность by design — устав §6.8, raw SQL конкатенация запрещена синтаксически)
5. Связь `references()` — минимум foreign key объявление, полноценные constraints — опционально для MVP (ADR решает объём)

### Definition of Done

- Golden-тест проходит, таблицы реально создаются в SQLite при первом запуске
- ADR: additive-only миграция policy, поведение при конфликте типов существующей колонки (fail loud, не молчаливая порча данных)
- Проверено: `schema` декларация в файле отдела (`dept/osp.mlog`) работает несмотря на упомянутую в архитектуре проблему "placeholder-файлы, реальная логика инлайнена в app.mlog из-за mlog binary import caching" — если caching ломает `schema` объявления в dept-файлах, это отдельный баг import-системы, фиксируется до реализации Problem C, не после

---

## PROBLEM D — Hook-декларация для Telegram callback/web_app_data

### Диагноз

`Hook` уже заявлен как один из 24 видов объявлений в уставе (§4), но архитектура V2 прямым текстом говорит: «web_app_data обработчики (OSP-кнопки в Mini App) заблокированы на mlog binary — нельзя изменить без пересборки». Это значит либо `Hook` никогда не был реализован для этого конкретного класса событий (Telegram callback_query / web_app_data), либо реализован для другого класса хуков и не generic.

**Первый шаг — не разработка, а диагностика:** проверить существующую реализацию `Hook` в интерпретаторе/компиляторе прежде чем проектировать новую конструкцию поверх старой.

### Prior art

Webhook паттерны Actix-web/Axum (route registration с handler function), Telegram Bot API callback_query спецификация, event-driven архитектуры общего вида (publish/subscribe с типизированным payload).

### Диагностический контракт (выполнить первым)

```bash
grep -rn "Hook" src/ --include="*.rs" | head -30
grep -rn "HookDecl\|hook" src/parser/ src/interpreter/ 2>/dev/null
```

Ответить на три вопроса прежде чем писать контракт реализации:
1. `Hook` парсится грамматикой pest — да/нет, для какого синтаксиса?
2. Если парсится — исполняется ли интерпретатором, или падает `Unimplemented`?
3. Если исполняется — под какой класс событий спроектирован (HTTP route? файловая система? нечто иное)?

### Контракт (при подтверждении что Hook нужно расширять/достраивать)

**`examples/telegram_hook.mlog`:**

```mlog
hook telegram_callback(pattern: "dept:osp:watch:*") {
  // pattern matches callback_data вида "dept:osp:watch:42"
  let analysis_id = extract_param(callback.data, 2)  // "42"
  mark_watched(to_int(analysis_id))
  answer_callback(callback.id, "Поставлено на наблюдение")
}

hook telegram_webapp_data(dept: "osp") {
  let action = json_get(webapp.data, "action")
  match action {
    "watch"   -> mark_watched(json_get(webapp.data, "id"))
    "redteam" -> trigger_redteam(json_get(webapp.data, "id"))
    "clarify" -> handle_clarify(webapp.data, webapp.chat_id)
    else      -> log("unknown action: " + action)
  }
}

flow Main {
  // hooks регистрируются при старте mlog serve, не вызываются напрямую
  return "hooks registered"
}
```

**`examples/telegram_hook.expected`:**

```
hooks registered
```

(функциональный тест хуков — отдельный интеграционный тест с mock Telegram update, не golden-file на stdout)

### Что реализовать (если диагностика показала «нет generic реализации»)

1. `hook telegram_callback(pattern: String) { ... }` — регистрация обработчика на паттерн `callback_data`
2. `hook telegram_webapp_data(dept: String) { ... }` — регистрация обработчика на `web_app_data` для конкретного отдела
3. `extract_param(text, index)` — builtin, парсинг `:`-разделённого callback_data
4. `answer_callback(callback_id, text)` — builtin, HTTP-вызов Telegram `answerCallbackQuery` (через существующий OmniRoute/llm_proxy слой или напрямую, если `http_post` уже даёт достаточно)
5. Диспетчер в `mlog serve`: входящий webhook update с `callback_query` или `web_app_data` полем маршрутизируется в зарегистрированные хуки по паттерну/отделу, а не падает как необрабатываемый

### Definition of Done

- Диагностический отчёт сдан **до** начала кодирования (может показать, что Problem D дешевле, чем казалось — если Hook уже частично работает)
- Если требуется реализация: golden-тест с mock Telegram payload проходит
- ADR: почему регистрация хуков через `dept:` namespace, а не глобальный роутинг — обоснование через существующую архитектуру (dept-изоляция уже принцип системы)
- **Критический тест:** Action Bar кнопки из старых Mini App V2 нарядов (Копировать/PDF/Поделиться/На наблюдение/Контр-анализ) — хотя бы одна реально доходит до `mark_watched()` через полный цикл Telegram → OmniRoute → mlog hook → SQLite

---

## ПОРЯДОК РЕАЛИЗАЦИИ

```
Problem D диагностика (2-3 часа, ДЁШЕВО, может отменить остальную работу по D)
    │
    ├── если Hook уже работает → Problem D становится конфигурационной, не языковой
    └── если не работает → полноценная реализация (см. контракт)

Problem C (schema-as-code)      ← независим, можно параллельно с D
    │
    ↓ (research_sources нужен для полного ОСП V3 жёсткого правила №8)

Problem A (tiered skill index)  ← самый большой ROI, разблокирует нормальную работу ОСП V3
    │
    ↓ (после A — FORMAT_INSTRUCTION конфликт решается сам:
       Tier 1 master-скилл несёт формат-инструкцию,
       два места дублирования в llm_proxy.py и app.mlog устраняются,
       единственный источник истины — skill_index)

Problem B (actor potential)     ← нужен для Phase 3a ОСП V3, независим от A/C/D
```

**Минимальный жизнеспособный набор для разблокировки ОСП V3 полностью:** A + B + C. Problem D — отдельная ветка ценности (интерактивность Mini App), не блокирует корректность самого препарирования.

---

## ЧТО ЭТОТ НАРЯД НЕ ДЕЛАЕТ

- Не трогает Python-слой (`llm_proxy.py`, OmniRoute) — только mlog-язык и его builtins
- Не переносит существующие 11 отделов на новую skill_index структуру — это отдельный наряд после того, как Problem A реализована и проверена на ОСП
- Не проектирует полную auth/ACL модель для hooks — MVP предполагает единственного владельца (текущая система single-tenant), multi-user out of scope

---

## STOP ТРИГГЕРЫ

1. `EntityType` уже покрывает то, что просится как `struct` в Problem B → не дублировать конструкцию, писать ADR о переиспользовании существующего типа
2. `Db` декларация уже спроектирована под то, что просится как `schema` в Problem C → расширять существующее, не создавать параллельную конструкцию
3. Диагностика Problem D показывает, что Hook принципиально не может держать HTTP webhook (архитектурное ограничение рантайма, не недоделанность) → эскалация владельцу, возможен пересмотр вообще самой идеи interactive Mini App на текущем стеке
4. `fit_to_budget` с целыми скиллами всё равно превышает разумный контекст LLM (25000 токенов Tier 1+2+3 ОСП может быть больше, чем принимает дешёвая модель в fosved-free комбо) → бюджет должен быть per-model, не глобальной константой — фиксируется в ADR как известное ограничение MVP

---

**Конец наряда. Метод обратной итерации: после закрытия A+B+C+D ни один будущий отдел не должен писать substring-хаки, ручные Python-таблицы или ждать пересборки Rust ради кнопки в Mini App.**
