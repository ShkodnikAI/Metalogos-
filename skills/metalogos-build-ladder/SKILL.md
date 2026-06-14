---
name: metalogos-build-ladder
description: >
  Build the Metalogos interpreter bottom-up, one milestone at a time. USE THIS SKILL
  whenever you are about to write or extend the compiler/interpreter, start a new
  milestone (M1–M5) or roadmap phase, set up the project skeleton, decide what to build
  next, or wire a test harness for .mlog programs. Trigger it even if the user just says
  "let's start building", "implement entity/pattern/flow", "add rules", "make learnable
  work", or "set up the repo" — it defines the required build order, the contract-first
  method, and the definition of done for every step. Do not write interpreter code without
  consulting this skill first.
---

# Metalogos — лестница постройки

Метод обратной итерации: **сначала целевая программа, потом ровно столько рантайма, чтобы она
запустилась.** Никогда не строй «вперёд» по подсистемам — строй «назад» от запускаемого `.mlog`.

## Цикл на каждую фичу

1. **Контракт.** Напиши минимальную `.mlog`-программу в `examples/`, демонстрирующую фичу.
2. **Падающий тест.** Golden-тест: вход `prog.mlog` + ожидаемый `prog.expected`. Сейчас он красный.
3. **Минимальный код.** Допиши лексер/парсер/AST/интерпретатор ровно настолько, чтобы тест позеленел.
4. **Зелёное + рефактор.** Весь сьют зелёный (включая прошлые вехи). Почисти. ADR на решения семантики.
5. **Следующая фича.** Никогда не бери две сразу.

## Архитектура интерпретатора (стартовая, tree-walking)

```
.mlog → Lexer (pest/chumsky) → tokens → Parser → AST → Interpreter → Effects
                                                   │                    ├─ stdout
                                       (позже: Type Check)              ├─ LLM call (M3)
                                                                        └─ Memory store (M4)
```

Крейты на старте: `mlog-lexer`, `mlog-parser`, `mlog-ast`, `mlog-runtime` (интерпретатор + эффекты),
`mlog-cli` (команда `mlog run prog.mlog`). Остальные крейты roadmap появляются позже по вехам.
Bytecode-VM/JIT — это фаза 4, **не сейчас**.

## Тест-харнесс (golden files)

Каждый пример — пара файлов. Раннер исполняет `mlog run`, сравнивает stdout с `.expected`.
Для `learnable`/`adapt` (недетерминированные) — мокай LLM детерминированной заглушкой в тестовом
режиме; проверяй структуру вывода и сработавшие правила, а не точный текст модели.

```
examples/
  m1_hello.mlog        m1_hello.expected
  m2_triage.mlog       m2_triage.expected
  ...
tests/run_examples.rs  # прогоняет все пары, падает при расхождении
```

---

## Вехи

> Каждая веха = один работающий язык. Контракты ниже — это и есть «определение готовности».

### M1 · Ядро — `entity` + `pattern` (чистый) + `flow` (линейный пайп)
Доказывает, что петля lexer→parser→AST→interpreter замкнута end-to-end.

```mlog
entity greeting: String = "Hello, Metalogos!"

pattern Shout(s: String) -> String { return upper(s) + "!" }

flow Main { input: String = greeting -> Shout -> output }
```
**Строим:** литералы/идентификаторы, объявление `entity`, чистый `pattern` с одним `return`,
встроенный `upper`, линейный `flow` с `->`, печать `output`.
**Готово, когда:** `mlog run m1_hello.mlog` печатает `HELLO, METALOGOS!!`.

### M2 · Уверенность — `Float`-confidence + `rule` + ветвление по confidence во `flow`
Теперь язык «ощущается» вероятностным.

```mlog
entity Message { text: String, urgency: Float = 0.0 }
entity m: Message = { text: "срочно нужна помощь", urgency: 0.0 }

rule If(m.text contains "срочно") then m.urgency = 0.9 with priority=10

flow Main {
  input: Message = m -> Classify -> output
  Classify {
    high (m.urgency > 0.8)        -> Escalate
    medium (m.urgency 0.4..0.8)   -> Queue
    low  (m.urgency < 0.4)        -> Ignore
  }
}
```
**Строим:** поля-сущности с типами и дефолтами, движок правил (приоритет + порядок срабатывания,
см. скилл `metalogos-language-semantics`), `contains`, ветвление `flow` по порогам confidence.
**Готово, когда:** правило поднимает urgency и поток уходит в ветку `Escalate`.

### M3 · `learnable` = вызов LLM — первый «вау»-момент
Паттерн исполняется как промпт к модели. Это уже полезный агент на естественном языке.

```mlog
learnable pattern Classify(text: String) -> Category {
  prompt: "Классифицируй сообщение: question | complaint | greeting"
}

flow Main {
  input: String = "ваш сервис ужасен" -> Classify -> Respond -> output
}
```
**Строим:** ключевое слово `learnable`, HTTP-клиент к API модели, парсинг ответа в тип-результат,
привязку confidence к выходу. В тестах — мок-модель.
**Готово, когда:** реальный вызов модели классифицирует сообщение и поток отвечает.
**Стадии после MVP:** few-shot накопление → дообучение (фаза 2, PyO3).

### M4 · Память — `memorize` / `recall` / `forget` / `decay`
```mlog
memory { episodic: { retention: 30.days }, semantic: { recall: similarity } }
memorize "пользователь любит острое" with priority=0.8
let pref = recall "вкусовые предпочтения" with min_confidence=0.5
forget outdated after 30.days
```
**Строим:** in-memory store + персист через `serde`; затухание по времени; `recall` по сходству
(на старте — строковое/эмбеддинг-заглушка; векторная БД qdrant — когда упрёшься в масштаб).
**Готово, когда:** `recall` достаёт `memorize`-факт, а `decay`/`forget` его убирают по времени.

### M5 · `adapt` — самомодификация в безопасной форме
```mlog
adapt Classify with new_example feedback=user_correction
mutate pattern Classify { add_example: ("спасибо!", greeting) rollback_if: accuracy < 0.9 }
sandbox experimental { allowed: [read_data, compute], forbidden: [network, write_permanent], timeout: 60.seconds }
```
**Строим:** дописывание few-shot примеров к `learnable`-паттерну, прогон тест-сьюта, откат при
падении точности, исполнение только в sandbox. **Не строим:** genetic/gradient/random_search (research).
**Готово, когда:** коррекция меняет поведение `Classify`, а ухудшающая мутация автоматически откатывается.

---

## После M5 — путь к универсальному языку (фазы roadmap)

Лестница не заканчивается на M5 — она открывает дорогу к миссии:
- Фаза 1: полноценная система типов (`mlog-types`, Fluid Types), семантический анализ, Entity Store, codegen.
- Фаза 2: PyO3/PyTorch/ONNX, граф знаний, реальное обучение и transfer learning.
- Фаза 3: CLI/REPL, LSP, `mlogpkg`, стандартная библиотека, mdbook.
- Фаза 4: bytecode-VM/JIT, оптимизации, самохостинг компилятора на Metalogos.

Каждая последующая подсистема строится тем же циклом: контракт → падающий тест → минимальный код → зелёное.
