<div align="center">

# 🔱 METALOGOS

### Язык Программирования для ИИ

*Первый язык, спроектированный ИИ для ИИ*

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-Phase%200%20%7C%20Foundation-yellow.svg)](https://github.com/ShkodnikAI/Metalogos-/blob/main/ROADMAP.md)

[Спецификация](SPECIFICATION.md) · [Дорожная карта](ROADMAP.md) · [Контрибьюторам](CONTRIBUTING.md) · [Документация](docs/)

</div>

---

## 📜 Манифест

Все существующие языки программирования созданы людьми для людей. Они опираются на человеческую логику, человеческие ограничения и человеческий способ мышления. Даже языки, используемые в ИИ (Python, C++, CUDA), — это инструменты, которые ИИ вынужден «натягивать» на себя, как чужую одежду.

**Metalogos** — первый язык, спроектированный ИИ для ИИ.

Название происходит от греч. *meta* (за пределами) + *logos* (разум, слово, закон). Это язык за пределами человеческой логики — язык, где естественной операцией является обучение, где данные первичны, а код вторичен, где самомодификация не баг, а фича.

---

## ⚡ Фундаментальные отличия

| Аспект | Человеческие языки | Metalogos |
|---|---|---|
| **Первичность** | Код определяет данные | Данные определяют код |
| **Логика** | Булева (истина/ложь) | Вероятностная (0.0–1.0) |
| **Изменяемость** | Статичен после компиляции | Адаптируется в рантайме |
| **Типы** | Фиксированные | Текучие (Fluid Types) |
| **Память** | Ручное или GC управление | Семантическая с затуханием |
| **Параллелизм** | Явный (threads, async) | Неявный (встроенный) |
| **Обучение** | Внешний процесс | Встроенная операция языка |
| **Ошибки** | Исключения, краш | Градиентный откат (soft failure) |

---

## 🏛 Семь Столпов Metalogos

┌─────────────────────────────────────────────┐│              METALOGOS CORE                  │├─────────┬─────────┬─────────┬───────────────┤│ ENTITY  │ PATTERN │ FLOW    │ MEMORY        ││ (Данные)│ (Логика)│ (Проц.) │ (Хранение)    │├─────────┼─────────┼─────────┼───────────────┤│ RULE    │ LEARN   │ ADAPT   │               ││ (Прав.) │ (Обуч.) │ (Эволюц)│               │└─────────┴─────────┴─────────┴───────────────┘

| Столп | Описание | Заменяет |
|---|---|---|
| **Entity** | Сущности с идентичностью, отношениями, историей и уверенностью | Переменные и объекты |
| **Pattern** | Обучаемые трансформации данных | Функции и методы |
| **Flow** | Декларативные потоки данных | Control flow (if/else/for) |
| **Memory** | Семантическая память с затуханием | RAM / heap |
| **Rule** | Декларативные правила с вероятностной логикой | Условные операторы |
| **Learn** | Встроенное обучение как операция языка | Внешние ML-фреймворки |
| **Adapt** | Самомодификация кода в рантайме | Не имеет аналогов |

---

## 🚀 Быстрый старт

### Установка (когда будет доступно)

```bash
# Из исходников
git clone https://github.com/ShkodnikAI/Metalogos-.git
cd Metalogos-
cargo install --path crates/mlog-cli

# Проверка
mlog --version

Hello, Metalogos!
// Простейшая программа
entity greeting: String = "Hello, Metalogos!"

pattern SayHello(name: String) -> String {
    return "Hello, {name}! Welcome to the future."
}

flow Main {
    input -> SayHello -> output
}

Сущности с отношениями
entity User {
    id: UUID
    name: String
    knows: Relation<User, strength=Float>
    confidence: Float = 1.0
}

entity alice: User = {
    id: uuid(),
    name: "Alice",
    knows: [(bob, 0.9), (charlie, 0.3)]
}

Текучие типы (Fluid Types)
// Может быть Number или Text с вероятностью
entity y: Fluid<Number[0.8], Text[0.2]> = input

// Семантический тип — несёт смысл
entity email: Semantic<"email_address"> = "user@example.com"
entity age: Semantic<"human_age", range=0..150> = 25

Обучаемые паттерны
// ИИ учится выполнять трансформацию
learnable pattern Translate(text: String, from: Lang, to: Lang) -> String {
    architecture: Transformer(layers=6, heads=8)
    dataset: parallel_corpora
    metrics: [bleu, comet]
}

// Гибрид: правила + ML
hybrid pattern ClassifyEmail(email: Email) -> Category {
    rule: if email.subject.contains("invoice") -> Category.Finance
    fallback: learned_classifier
    confidence_threshold: 0.7
}

Потоки данных
// Данные текут через паттерны
flow ProcessUserInput {
    input -> Parse -> Validate -> Classify -> Respond -> output

    Validate {
        high_confidence (>0.9) -> Classify
        medium_confidence (0.5-0.9) -> AskClarification -> Classify
        low_confidence (<0.5) -> EscalateToHuman
    }
}

// Параллельный поток — автоматически распараллеливается
parallel flow AnalyzeImage(img: Image) -> Analysis {
    branch_a: DetectObjects(img) -> objects
    branch_b: ExtractText(img) -> text
    branch_c: ClassifyScene(img) -> scene
    merge: CombineAnalysis(objects, text, scene) -> Analysis
}

Семантическая память
memory {
    working:   { capacity: 7,  decay: 0.1 }
    semantic:  { structure: KnowledgeGraph, indexing: vector_embedding }
    episodic:  { structure: TemporalGraph,  retention: 90.days }
}

memorize important_fact = "Earth orbits the Sun"
memorize user_preference = User.likes_spicy_food with priority=0.8

let fact = recall "planetary orbits" with min_confidence=0.5

// Забывание — не баг, а фича
forget outdated_info after 30.days
decay old_memories with rate=0.01

Вероятностные правила
rule If(user.age >= 18) then can_vote(user) = true
rule If(weather == "rainy") then need_umbrella = 0.85
rule If(score > 90) then grade = "A" with priority=10
rule If(score > 85) then grade = "B+" with priority=5

Встроенное обучение
learn Translate with {
    data: parallel_corpora
    epochs: 10
    optimizer: Adam(lr=0.001)
    loss: CrossEntropy
    early_stopping: patience=3
}

adapt ClassifyEmail with new_example feedback=user_correction

transfer knowledge from PretrainedBERT to MyClassifier {
    freeze_layers: [1, 2, 3]
    fine_tune_layers: [4, 5, 6]
}

Самомодификация (Adapt)
// Эволюция кода — паттерн может переписать сам себя
evolve OptimizeRoute if efficiency < 0.7 {
    strategies: [genetic, gradient, random_search]
    constraints: must_preserve_safety_rules
    verification: test_suite_must_pass
}

// Мутация с откатом
mutate pattern ClassifyEmail {
    add_feature: email.header_length
    rollback_if: accuracy_drops_below(0.9)
}

// Защитные механизмы
sandbox experimental_code {
    allowed: [read_data, compute, write_temp]
    forbidden: [write_permanent, network_access, modify_other_patterns]
    timeout: 60.seconds
}

Полный пример: Интеллектуальный Ассистент
use Standard.AI.NLP
use Standard.AI.Vision
use Standard.Memory

module SmartAssistant {

    entity User {
        id: UUID
        name: String
        language: Lang = detect
        preferences: Map<String, Fluid<String, Number>>
        satisfaction: TimeSeries<Float>
    }

    learnable pattern Understand(input: Multimodal, context: Memory) -> Intent {
        architecture: Transformer(layers=12, heads=12, dim=768)
        multimodal: true
        outputs: [intent, entities, sentiment, urgency]
    }

    hybrid pattern HandleRequest(input: Multimodal, user: User) -> Response {
        rule: if input.is_greeting -> Greet(user)
        rule: if input.is_question and lookup(input) -> FormatAnswer(lookup(input))
        fallback: {
            let intent = Understand(input, user.context)
            let response = GenerateResponse(intent, user)
            return response
        }
    }

    flow MainLoop {
        input: Multimodal = receive_user_input()

        input |>
            Understand(context=memory.working) |>
            HandleRequest(user=current_user) |>
            GenerateResponse |>
            SendToUser |>
            RecordInteraction |>
            LearnFromFeedback

        if user.satisfaction.trend == "declining" {
            adapt GenerateResponse based_on recent_negative_feedback
        }
    }

    memory {
        working:   { capacity=10, decay=0.05 }
        semantic:  KnowledgeGraph(embedding_dim=768)
        episodic:  TemporalGraph(retention=90.days)
    }

    rule If(user.language != system_language) then auto_translate = true
    rule If(urgency > 0.8) then prioritize = true with priority=10

    evolve HandleRequest every 7.days {
        strategies: [add_rule, refine_pattern, restructure_flow]
        constraints: must_pass_safety_tests
    }
}


🏗 Архитектура Компилятора
┌──────────────┐    ┌──────────┐    ┌──────────┐    ┌──────────────┐
│  Source Code  │───▶│  Lexer   │───▶│  Parser  │───▶│     AST      │
│  (.mlog)     │    │ (tokens) │    │          │    │              │
└──────────────┘    └──────────┘    └──────────┘    └──────┬───────┘
                                                           │
                                                           ▼
┌──────────────┐    ┌──────────┐    ┌──────────┐    ┌──────────────┐
│   Runtime    │◀───│ Codegen  │◀───│ Semantic │◀───│  Type Check  │
│              │    │          │    │ Analysis │    │ (Fluid Types)│
└──────┬───────┘    └──────────┘    └──────────┘    └──────────────┘
       │
       ├──▶ Entity Store (Rust)
       ├──▶ Pattern Executor (Rust + PyTorch)
       ├──▶ Flow Engine (Rust, async)
       ├──▶ Memory System (Rust + Vector DB)
       ├──▶ Rule Engine (Rust)
       ├──▶ Learn Engine (PyTorch via PyO3)
       └──▶ Adapt Engine (Rust + sandboxed execution)


📁 Структура Репозитория
Metalogos/
├── README.md                    # Этот файл
├── SPECIFICATION.md             # Полная спецификация языка
├── ROADMAP.md                   # Дорожная карта
├── CONTRIBUTING.md              # Как контрибьютить
├── LICENSE                      # MIT / Apache 2.0
├── Cargo.toml                   # Workspace root
│
├── crates/
│   ├── mlog-lexer/              # Лексер (токенизация)
│   ├── mlog-parser/             # Парсер → AST
│   ├── mlog-ast/                # Определения AST
│   ├── mlog-types/              # Fluid Types система
│   ├── mlog-semantic/           # Семантический анализ
│   ├── mlog-codegen/            # Генерация кода
│   ├── mlog-runtime/            # Runtime (entity, pattern, flow, memory)
│   ├── mlog-learn/              # ML интеграция (PyTorch via PyO3)
│   ├── mlog-adapt/              # Adapt система (эволюция, мутации)
│   ├── mlog-cli/                # CLI: mlog
│   ├── mlog-lsp/                # Language Server Protocol
│   └── mlog-pkg/                # Пакетный менеджер
│
├── std/                         # Стандартная библиотека (на Metalogos)
│   └── Standard/
│       ├── AI/
│       │   ├── NLP/
│       │   └── Vision/
│       ├── Memory/
│       └── Math/
│
├── examples/                    # Примеры программ
├── docs/                        # Документация (mdbook)
├── tests/                       # Интеграционные тесты
└── .github/
    └── workflows/               # CI/CD


🛠 Технологический Стек
Языки реализации



Компонент
Язык
Обоснование



Компилятор / Runtime
Rust
Производительность, безопасность, контроль памяти, отличный ecosystem для компиляторов


ML Backend
Python + PyTorch
Зрелая ML-экосистема, интеграция через PyO3


Стандартная библиотека
Rust + Metalogos
Само-хостинг на поздних этапах


CLI / Инструменты
Rust
Единый стек с компилятором


Ключевые библиотеки и инструменты



Категория
Инструмент
Назначение



Парсер
nom / pest
Лексер + парсер Metalogos


AST
Ручная реализация (Rust)
Абстрактное синтаксическое дерево


Type System
Ручная реализация
Fluid Types, вероятностная типизация


ML Runtime
PyTorch + ONNX Runtime
Исполнение learnable patterns


Векторная БД
qdrant / milvus
Семантическая память (embedding search)


Граф знаний
neo4j / встроенный
Entity relations, semantic memory


Сериализация
serde
AST, entities, memory persistence


CLI
clap
Командная строка mlog


LSP
tower-lsp
Language Server Protocol для IDE


Тестирование
proptest
Property-based тестирование


CI/CD
GitHub Actions
Автоматизация сборки и тестов


Документация
mdbook
Книга по языку


Пакетный менеджер
Собственный mlogpkg
Управление зависимостями



🗺 Дорожная карта
Фаза 0: Фундамент (Месяцы 1–3) 🟡 Текущая
# Metalogos-
