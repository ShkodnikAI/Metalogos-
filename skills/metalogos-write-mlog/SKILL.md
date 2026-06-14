---
name: metalogos-write-mlog
description: >
  Write correct, idiomatic Metalogos (.mlog) source code. USE THIS SKILL whenever you author or
  edit any .mlog file — example programs, contract programs for a milestone, standard-library code,
  test fixtures, or documentation snippets. Trigger it any time you need the syntax of entity,
  pattern, learnable/hybrid pattern, flow, rule, memory, learn, adapt, or sandbox, and whenever you
  are creating the "contract" program that a new feature must satisfy. It is the source of truth for
  surface syntax and idioms, distilled from the language README.
---

# Metalogos — как писать `.mlog`

Файл-пример **есть контракт**: он фиксирует синтаксис и поведение прежде, чем пишется код интерпретатора.
Держи примеры минимальными — одна программа демонстрирует одну фичу. Каждый пример сопровождается
golden-файлом `*.expected` (см. скилл `metalogos-build-ladder`).

## Семь столпов → конструкции

| Столп | Конструкция | Заменяет |
|---|---|---|
| Entity | `entity` | переменные/объекты |
| Pattern | `pattern` / `learnable pattern` / `hybrid pattern` | функции/методы |
| Flow | `flow` с `->` или `|>` | if/else/for |
| Memory | `memory { }`, `memorize`/`recall`/`forget`/`decay` | RAM/heap |
| Rule | `rule If(...) then ... with priority=N` | условные операторы |
| Learn | `learn ... with { }` | внешние ML-фреймворки |
| Adapt | `adapt` / `mutate` / `evolve` (+ `sandbox`) | — |

## Сущности (Entity)

```mlog
entity greeting: String = "Hello"

entity User {
  id: UUID
  name: String
  confidence: Float = 1.0
  knows: Relation<User, strength=Float>
}

entity alice: User = { id: uuid(), name: "Alice", knows: [(bob, 0.9)] }
```
Поля имеют тип и опциональный дефолт. `Float`-поля несут уверенность. `Relation<T, ...>` — связи с весом.

## Текучие и семантические типы

```mlog
entity y: Fluid<Number[0.8], Text[0.2]> = input          // суперпозиция, коллапс в точке использования
entity email: Semantic<"email_address"> = "u@example.com" // тип несёт смысл
entity age: Semantic<"human_age", range=0..150> = 25
```

## Паттерны (Pattern)

```mlog
pattern Shout(s: String) -> String { return upper(s) + "!" }   // чистый

learnable pattern Classify(text: String) -> Category {          // = вызов LLM (M3)
  prompt: "Классифицируй: question | complaint | greeting"
}

hybrid pattern Triage(m: Message) -> Category {                 // правило + откат на learned
  rule: if m.text contains "invoice" -> Category.Finance
  fallback: Classify(m.text)
  confidence_threshold: 0.7
}
```

## Потоки (Flow)

```mlog
flow Main { input: String = greeting -> Shout -> output }       // линейный

flow Process {                                                  // ветвление по уверенности
  input -> Parse -> Validate -> output
  Validate {
    high   (>0.9)      -> Classify
    medium (0.5..0.9)  -> AskClarification -> Classify
    low    (<0.5)      -> EscalateToHuman
  }
}

parallel flow AnalyzeImage(img: Image) -> Analysis {            // авто-распараллеливание
  branch_a: DetectObjects(img) -> objects
  branch_b: ExtractText(img)   -> text
  merge: Combine(objects, text) -> Analysis
}
```
`->` и `|>` — конвейер данных. Ветви внутри шага задают пороги уверенности.

## Правила (Rule)

```mlog
rule If(user.age >= 18) then can_vote(user) = true
rule If(weather == "rainy") then need_umbrella = 0.85          // значение-уверенность
rule If(score > 90) then grade = "A" with priority=10           // приоритет разрешает конфликты
```

## Память (Memory)

```mlog
memory {
  working:  { capacity: 7,  decay: 0.1 }
  semantic: { structure: KnowledgeGraph, indexing: vector_embedding }
  episodic: { structure: TemporalGraph, retention: 90.days }
}
memorize user_preference = User.likes_spicy_food with priority=0.8
let fact = recall "planetary orbits" with min_confidence=0.5
forget outdated_info after 30.days
decay old_memories with rate=0.01
```

## Обучение и адаптация (Learn / Adapt)

```mlog
learn Classify with { data: corpus, epochs: 10, optimizer: Adam(lr=0.001) }   // фаза 2

adapt Classify with new_example feedback=user_correction                       // few-shot (M5)
mutate pattern Classify { add_example: ("спасибо!", greeting)  rollback_if: accuracy < 0.9 }

sandbox experimental {                                                         // обязательна для adapt
  allowed:   [read_data, compute, write_temp]
  forbidden: [write_permanent, network_access, modify_other_patterns]
  timeout:   60.seconds
}
```

## Идиомы

- **Один пример — одна фича.** Контракт должен падать до реализации и зеленеть после.
- **Уверенность — первого класса.** Где это уместно, типы и выходы несут `Float`-уверенность; ветвись по ней.
- **Ошибки — мягкие.** Вместо краша возвращай результат с низкой уверенностью (см. soft-failure в скилле
  `metalogos-language-semantics`).
- **`learnable` ≠ магия.** На старте это промпт к модели; в тестах — детерминированный мок.

## Канонические якоря

- **M1 hello** (минимум): `entity` + чистый `pattern` + линейный `flow`.
- **M5 цель** — «умный ассистент»: `Understand` (learnable) + `HandleRequest` (hybrid) + `flow MainLoop` с
  записью взаимодействий, `memory`, правилами приоритета и `adapt` по падению удовлетворённости. Это финальный
  контракт ядра — к нему ведёт вся лестница.
