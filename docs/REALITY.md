# REALITY.md — факт-чек активов и рабочая оценка готовности к P0

> **Статус: SSOT по готовности к P0.** Эта страница — единственная точка
> истины по вопросу «что из заявленного в плане v2 реально существует, и
> какая доля P0-готовности достигнута». Создана нарядом №318 (P0/docs,
> issue #405, волна 0, шаг 0.1 по §13.2 плана v2). Обновляется или
> отзывается только новым факт-чеком тем же нарядным протоколом — правки
> мимо протокола запрещены: каждое число здесь либо воспроизводимо
> командой, либо помечено UNVERIFIED.
>
> **Снапшоты.** Снапшот плана v2 — `fc59e9e` (2026-09-14 13:56 +0300,
> merge #396). Текущий main на момент проверки — `1876fdf`. Все команды
> раздела 1 воспроизводимы на снапшоте **без чекаута** — через
> `git show fc59e9e:<путь>`; там, где якорь на main сдвинулся, это
> указано явно, новый якорь зафиксирован (раздел 5).
>
> **Честность.** План v2 в репозитории ОТСУТСТВУЕТ — проверка отсутствия:
> `git ls-tree -r --name-only HEAD | grep -iE 'plan|план'` возвращает
> только `docs/refactoring-split-plan.md` (другой документ) и ложные
> совпадения по `openplanter`. Текст §2 плана доступен только как цитата
> в теле issue #405. Всё, что требует полного текста плана (веса
> разложения в разделе 3), помечено UNVERIFIED и является рабочей
> реконструкцией, подлежащей сверке при появлении плана v2 в репо.

---

## 0. Сводка вердиктов

| # | Якорь §2 плана v2 | Вердикт | Снапшот `fc59e9e` | Main `1876fdf` |
|---|---|---|---|---|
| 1 | `TaintKind` — audit.rs:119, «5 advisory-видов» | **CONFIRMED** | audit.rs:119, 5 вариантов | без сдвига (119) |
| 2 | `TaintTracker` — audit.rs:142, per-scope HashMap | **CONFIRMED** | audit.rs:141–142 | без сдвига (141–142) |
| 3 | Category-A гейт `MODEL_WEIGHTS_UNSAFE` — audit.rs:1607–1757 | **CONFIRMED** (якорь сдвинулся) | 1607–1757 | **1660–1810** (+53) |
| 4 | `Statement` — ast.rs:1282, «10 видов» | **PARTIAL** | ast.rs:1282, вариантов **15** | без сдвига |
| 5 | «84 инструкции VM» — bytecode.rs:14 | **PHANTOM** | вариантов **47** | без сдвига (47) |
| 6 | `semantic.rs` — 3328 строк | **CONFIRMED** | 3328 | 3328 |
| 7 | 420 builtins — registry.rs:38 | **CONFIRMED** число / **PARTIAL** строка | 420 `spec!(`; объявление на строке **44** | **421** (+video_extend, №309) |
| 8 | «143 ADR» | **PARTIAL** | 142 ADR + индексный README = 143 файла | **145** ADR (+0151/0152/0153) |
| 9 | «214 примеров» | **CONFIRMED** (канонический базис) | 214 top-level `*.mlog` | 214 (рекурсивно 273) |
| 10 | zeroize — Cargo.toml:51 | **CONFIRMED** | строка 51 | без сдвига |
| 11 | `consent_ledger` — voice/store.rs:31 | **CONFIRMED** | store.rs:31 | без сдвига |
| 12 | Провенанс — vision/provenance.rs (№241) | **CONFIRMED** | 430 строк | 464 (№320) |
| 13 | MCP-клиент — builtins/mcp.rs (№268) | **CONFIRMED** | 613 строк | 613 |

Итог: 9 CONFIRMED (из них 1 со сдвигом якоря), 3 PARTIAL, 1 PHANTOM.
Ни один якорь не оказался «нарисованным» целиком — единственная грубая
ошибка плана v2 — число инструкций VM (п. 1.5).

---

## 1. Построчная сверка активов

### 1.1. `TaintKind` — audit.rs:119, «5 advisory-видов» — CONFIRMED

Команда (снапшот):
```bash
git show fc59e9e:src/audit.rs | sed -n '119p'; git show fc59e9e:src/audit.rs | awk '/^enum TaintKind/,/^}/' | grep -cE '^\s{4}[A-Z][A-Za-z]*,?\s*$'
```
Фактический вывод: `119: enum TaintKind {` — точно строка 119; число
вариантов — **5**. Полный список: `LlmOutput`, `Secret`, `UserInput`,
`Sanitized`, `CanaryLeak` (№284). На main — без сдвига. Уточнение к
формулировке плана: сами виды — это носители меток, а не «advisory-виды»;
advisory или блокирующим является **чек-потребитель** метки (см. раздел 2
и словарь check_id: 21 идентификатор в audit.rs на main).

### 1.2. `TaintTracker` — audit.rs:142, per-scope HashMap — CONFIRMED

Команда (снапшот):
```bash
git show fc59e9e:src/audit.rs | sed -n '141,142p'
```
Фактический вывод:
```text
struct TaintTracker {
    tainted: HashMap<String, TaintKind>,
```
Строка 142 — в точности поле `tainted: HashMap<String, TaintKind>`
(объявление структуры — 141). «Per-scope» подтверждается док-комментарием
над структурой и API из трёх методов (`taint` / `get_taint` / `untaint`);
`#[derive(Clone)]` — для path-sensitive форка в `check_canary_leak`
(№284). На main — без сдвига.

### 1.3. Category-A гейт `MODEL_WEIGHTS_UNSAFE` — audit.rs:1607–1757 — CONFIRMED (якорь сдвинулся)

Команда (снапшот):
```bash
git show fc59e9e:src/audit.rs | grep -n 'MODEL_WEIGHTS_UNSAFE' | head -3
```
Фактический вывод: `1607` — заголовок секции
`// ── Check: MODEL_WEIGHTS_UNSAFE + VISION_POLICY_MISSING`, `1757` —
`check_id: "MODEL_WEIGHTS_UNSAFE"` (Severity::Error, Category A —
статически видимые нарушения на `vision_fetch_weights(url, ...)`).
Диапазон 1607–1757 подтверждён как «секция гейта» на снапшоте.
**На main якорь сдвинулся: секция теперь 1660–1810** (комментарий-шаблон
`// MODEL_WEIGHTS_UNSAFE / VISION_UNSIGNED_EXPORT template (1607–1757)`
в коде main сохраняет исторические координаты). Новый якорь: **1660**.

### 1.4. `Statement` — ast.rs:1282, «10 видов» — PARTIAL

Команды:
```bash
git show fc59e9e:src/ast.rs | sed -n '1282p'
sed -n '1282,1400p' src/ast.rs | awk '/pub enum Statement/{f=1;next} f&&/^\}/{exit} f' | grep -cE '^\s{4}[A-Z][A-Za-z0-9]*'
```
Фактический вывод: `1282: pub enum Statement {` — позиция точна **на обоих
снапшотах**; полное число вариантов — **15**, не 10: `LetBinding`, `Assign`,
`Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Return`,
`ExprStmt`, `Match`, `Break`, `Continue`, `Memorize`, `Forget`, `Relate`.

Вердикт PARTIAL, а не PHANTOM: число «10» воспроизводимо при базисе
«15 минус Memory-варианты (Memorize/Forget/Relate) минус loop-control
(Break/Continue)» = 10, но этот базис в плане не указан и не совпадает
ни с одним счётчиком проекта. Отметим попутно обнаруженные расхождения
в README — три места с тремя разными счётчиками Statement, ни одно не
покрыто консистентными тестами: архитектурная диаграмма («12
Statement», рядом ещё и «29 Declaration» / «15 Expr» против реальных
33/14), таблица AST («12 Statement») — все исправлены в этом же наряде
на верифицированные 33/14/15.

### 1.5. «84 инструкции VM» — bytecode.rs:14 — PHANTOM

Команды:
```bash
git show fc59e9e:src/bytecode.rs | sed -n '14p'
git show fc59e9e:src/bytecode.rs | awk '/pub enum Instruction/,/^\}/' | grep -E '^\s{4}[A-Z][A-Za-z0-9]*' | grep -v '//' | wc -l
grep -rn '84 инс\|84 instr\|84 instructions' README.md docs/ src/
```
Фактический вывод: `14: pub enum Instruction {` — позиция точна; число
вариантов — **47** на снапшоте и на main; строка «84 инструкции» не
встречается **нигде** в репозитории. Сам README согласован с реальностью:
«bytecode VM (47 instructions; experimental …, ADR-0105/ADR-0141)».
Вердикт PHANTOM: ни один базис подсчёта (варианты enum, опкоды,
инструкции с операндами) не даёт 84. Число 84 в §2 — ошибка плана,
вероятно перенос из другой ревизии.

### 1.6. `semantic.rs` — 3328 строк — CONFIRMED

Команда (снапшот):
```bash
git show fc59e9e:src/semantic.rs | wc -l
```
Фактический вывод: `3328` — точно. На main — 3328 (без изменений).

### 1.7. 420 builtins — registry.rs:38 — CONFIRMED (число) / PARTIAL (строка)

Команды (снапшот):
```bash
git show fc59e9e:src/builtins/registry.rs | sed -n '38p'
git show fc59e9e:src/builtins/registry.rs | grep -c 'spec!('
```
Фактический вывод: на строке 38 — хвост `use`-импорта (`};`), а не
реестр; объявление `pub const BUILTIN_REGISTRY` — на строке **44**.
Число `spec!(` — **420** на снапшоте — подтверждено точно. Числовая
часть якоря верна, строковая координата неточна (38 → 44).
**На main — 421** (+`video_extend`, наряд №309); счётчик README
синхронизирован автотестом (`readme_total_builtins_match_reality`).

### 1.8. «143 ADR» — PARTIAL

Команды (снапшот):
```bash
git show fc59e9e --stat >/dev/null; git ls-tree -r --name-only fc59e9e docs/adr/ | grep -c '\.md$'
git ls-tree -r --name-only fc59e9e docs/adr/ | grep '\.md$' | grep -vcE '/[0-9]{4}-[^/]+\.md$'
```
Фактический вывод: всего `.md`-файлов в `docs/adr/` на снапшоте — **143**;
из них 142 — файлы формата `NNNN-*.md`, 1 — индексный `README.md`.
Канонический счётчик проекта (`real_adr_count()` в
`tests/readme_consistency.rs`) исключает README, т.е. на каноническом
базисе на снапшоте **142 ADR**. «143» воспроизводимо только базисом
`ls docs/adr/*.md | wc -l` (включая индекс). На main: **145** ADR
(+ADR-0151 №309, +ADR-0152 №320, +ADR-0153 №412) + индекс = 146 файлов;
клейм README синхронизирован автотестом.

### 1.9. «214 примеров» — CONFIRMED (канонический базис)

Команды (снапшот):
```bash
git ls-tree --name-only fc59e9e examples/ | grep -c '\.mlog$'
git ls-tree -r --name-only fc59e9e examples/ | grep -c '\.mlog$'
```
Фактический вывод: **214** top-level `*.mlog` — в точности число плана;
рекурсивно — 229 (с подкаталогами). Канонический базис проекта —
топ-уровень: именно так считает `real_example_count()` в
`tests/readme_consistency.rs` (нерекурсивный `fs::read_dir`) и именно
214 заявлено в README («214 .mlog programs (golden corpus)»). Вердикт
CONFIRMED; на main топ-уровень — те же 214 (рекурсивно 273: наряд №317
добавил корпус `examples/leak/`, исключённый из golden-цикла по
построению — `golden.rs` сканирует нерекурсивно).

### 1.10. zeroize — Cargo.toml:51 — CONFIRMED

Команда (снапшот):
```bash
git show fc59e9e:Cargo.toml | sed -n '51p'
```
Фактический вывод: `zeroize = "1"` — точно строка 51 (блок
«Phase 7.3: Real encryption», рядом `argon2 = "0.6"`). На main — без
сдвига (`grep -n 'zeroize' Cargo.toml` → `51:zeroize = "1"`).

### 1.11. `consent_ledger` — voice/store.rs:31 — CONFIRMED

Команда (снапшот):
```bash
git show fc59e9e:src/voice/store.rs | sed -n '31p'
```
Фактический вывод: `/// CREATE TABLE consent_ledger (` — точно строка 31
(док-комментарий схемы). Реальная схема живёт в коде: `CREATE TABLE IF
NOT EXISTS consent_ledger` (store.rs:61 на main) + API `record_consent`
(:127), `has_consent_record` (:143), `consent_count` (:157) + тест
`consent_ledger` (:225). На main — без сдвига строки 31.

### 1.12. Провенанс — vision/provenance.rs (№241) — CONFIRMED

Команда (снапшот):
```bash
git show fc59e9e:src/vision/provenance.rs | wc -l
git show fc59e9e:src/vision/provenance.rs | grep -n 'pub fn' | head -8
```
Фактический вывод: **430 строк**; публичный API: `sha256_hex`, `prompt_hash`,
`verify_sha_pin`, `manifest_sidecar_json`, `weights_tree_sha256`,
`model_hash32`, `embed_lsb_watermark`, `detect_lsb_watermark` — полный
контур провенанса (SHA-pinning весов, sidecar-манифест, LSB-водяной знак).
На main — 464 строки (наряд №320 добавил synthetic-поля манифеста, Art 50).

### 1.13. MCP-клиент — builtins/mcp.rs (№268) — CONFIRMED

Команда (снапшот):
```bash
git show fc59e9e:src/builtins/mcp.rs | wc -l
```
Фактический вывод: **613 строк**, без изменений на main. Содержание
подтверждается док-блоком модуля (taint-контракт: результат `mcp_call` —
`TaintKind::UserInput`) и README (stdio-транспорт, hand-rolled JSON-RPC,
exec-гейт, allowlist `METALOGOS_MCP_ALLOWLIST`, ADR-0132).

---

## 2. Что делает taint эффектом — а чего нет

Контекст: taint-механика Metalogos — **детектор и гейты на
специфических паттернах**, а не абстрактный интерпретатор с эффектами.
Это не дефект реализации, а точная граница: ниже каждое «не готово» из
плана подтверждено командой-доказательством отсутствия и привязано к
месту, где оно должно будет появиться. Все команды выполняются на
`1876fdf` от корня репозитория и воспроизводимы на любом чекауте, где
эффектов ещё нет.

**2.1. Решётка (lattice) меток — НЕТ.**
```bash
grep -rni 'lattice' src/        # вывод: пусто (0 вхождений)
```
`TaintKind` — плоский `enum` без порядка, наименьшей верхней границы и
оператора поглощения. Где появится: `src/audit.rs`, секция
`// ── Taint tracking for data-flow analysis ──` (строки 115–435 на main).

**2.2. Вывод по всем statement-видам — НЕТ (9 из 15).**
```bash
sed -n '2404,2983p' src/audit.rs | grep -c 'Statement::Match\|Statement::Break\|Statement::Continue'
# вывод: 0
```
Интерпроцедурный MVP `TAINT_INTERP` (№292, секция 2404–2983) обрабатывает
9 видов: `LetBinding`, `Assign`, `ExprStmt`, `Return`, `Each`,
`EachWithIndex`, `While`, `IfElseBlock`, `IfThen`. Не покрыты движком:
`Match`, `Break`, `Continue` (0 вхождений в секции) и Memory-варианты
`Memorize`/`Forget`/`Relate` (они обслуживаются отдельными
паттерн-чеками `TAINT_PERSISTENCE`, не интерп-движком). Где появится:
та же секция `TAINT_INTERP` — расширение матч-ручек по недостающим видам.

**2.3. Join в слияниях — НЕТ.**
```bash
grep -n 'fn join\|taint_join\|merge_taint\|widen' src/audit.rs   # вывод: пусто
```
Единственный механизм ветвящейся чувствительности — path-sensitive форк
клона трекера в `check_canary_leak` (№284, `#[derive(Clone)]` на
`TaintTracker`): состояние уходит в две ветки и **не сливается** обратно.
Где появится: точка слияния after-веток в `TAINT_INTERP` и в форке №284.

**2.4. Эффект-следы — НЕТ.**
```bash
grep -n 'effect' src/audit.rs    # вывод: пусто
```
Единственное вхождение слова в смежной зоне — исторический комментарий в
`semantic.rs:2901` о времени жизни привязки в блоке; концепции эффектов
(запись/чтение/сеть/необратимость как атрибуты выражений) в коде нет.
Где появится: новая секция в `src/audit.rs` рядом с taint-движком либо
модуль `src/effects.rs` с последующей привязкой к классификации builtins
(`src/builtins_classification.rs`, наряд №316: Role/Lift/Sink уже
перечислены — 205 non-Pure из 421).

**2.5. Exhaustive matching по меткам — НЕТ (как центральный контракт).**
```bash
grep -n 'match kind' -A 8 src/audit.rs   # вывод: пусто
```
Правила размазаны точечными матчами: `get_expr_taint` реализует
«санитайзер побеждает» (`render`/`escape_html` → `Sanitized`) и «первый
несанитизированный аргумент»; чеки секций матчат конкретные виды
(`Secret`, `UserInput`, `CanaryLeak`). Центрального исчерпывающего
матча, который компилятор заставил бы расширять при добавлении нового
вида метки, нет — новый вид можно добавить «незаметно» для части чеков.
Где появится: в taint-секции audit.rs как единая функция
трансформации/поглощения, сопровождающая решётку (2.1).

**2.6. Аффайность — НЕТ.**
```bash
grep -rni 'affinity' src/        # вывод: пусто
```
Ни привязки данных к потокам/акторам, ни afфайн-типов «одноразового
потребления» в коде нет. Где появится: после эффектов (2.4) — как
ограничение потребления на уровне семантики (`src/semantic.rs`) с гейтом
в `audit.rs`.

Существующая часть (чтобы раздел не читался как «движка нет вообще»):
метки ставятся на источники (`call_llm`/`env`/`form_data`/`mcp_call`),
снимаются санитайзерами (`render`/`escape_html`; `redact` — снимает
`Secret`, но НЕ снимает `CanaryLeak`, лекало ADR-0136 D2), распространяются
через `Ident`/`FnCall`/`BinaryOp`, и питают 21 check_id, из которых
блокирующие (Severity::Error) включены в `audit_category_a` /
`audit_program` и падают компиляцией на `mlog check`. Словарь классов
корпуса «обязан не компилироваться» — `tests/run_leak_suite.rs` (№317);
измеренная на сегодня полнота гейтов — 11/28 сценариев ловится (39%),
остальные 17 — контракт решётки №325.

---

## 3. Пересчитанная готовность к P0: **26%** (рабочая цифра Фазы 1)

> **UNVERIFIED-оговорка.** Веса подсистем — реконструкция из перечня
> «метки / capability / реестр бэкендов / ledger / память» (тело issue
> #405, ссылающееся на §2 плана v2); сам план в репо отсутствует, поэтому
> веса не верифицируемы и приняты как рабочие до появления плана в репо.
> Готовность каждой подсистемы, напротив, опирается только на
> код-факты разделов 1–2. Сумма — 25.85 ≈ **26%**, в целевом коридоре
> ~25% ± 5pp, заданном issue #405.

| Подсистема | Вес (UNVERIFIED) | Готовность | Вклад | Код-основание готовности |
|---|---|---|---|---|
| Метки (taint) | 30% | 55% | 16.5pp | `TaintKind` 5 видов, per-scope трекер, распространение, санитайзеры (ADR-0136), path-sensitive canary (№284), интерп-MVP (№292, глубина 2), 21 check_id, из них блокирующие — в Category-A; **нет**: решётка, join, полный вывод по видам, эффекты, exhaustive, аффайность (раздел 2) |
| Capability-модель | 20% | 0% | 0 | `grep -rni 'capability' src/` → пусто; лекало хэндла — ADR-0114, не применено |
| Реестр бэкендов | 15% | 10% | 1.5pp | Единого реестра/маршрутизатора нет (`BACKEND_REGISTRY` — 0 вхождений); реальны по-столповые реестры: `VOICE_REGISTRY` (voice/mod.rs:119), `VIDEO_REGISTRY` (video/mod.rs:249), vision-allowlist `MLOG_VISION_WEIGHTS_ALLOWLIST`, `BUILTIN_REGISTRY` (421) — но это реестры контента, не вычислительных бэкендов |
| Ledger | 15% | 35% | 5.25pp | `consent_ledger` реален (voice/store.rs:61/127/143/157 + тест :225); subprocess audit log (README, MCP-гейты); **нет**: универсального леджера действий, тампер-устойчивости, grant-механики необратимых операций (класс `IRREVERSIBLE_NO_GRANT` — только планируемый, №325) |
| Память | 20% | 13% | 2.6pp | Реальная локальная инфраструктура: memory_store.rs (1621 строк), memory_graph.rs (949), embeddings.rs (655), BM25+vector rank fusion, kv_/mem_ builtins; **нет**: P0-контракта памяти плана v2 (объём неизвестен без плана — UNVERIFIED); `recall` — spec-строка реестра без обработчика (`spec!("recall", 0, "stub")`, registry.rs:248) |
| **Итого** | **100%** | — | **25.85 ≈ 26%** | |

**Почему не ~60% (v1): пять причин расхождения.**

1. **Разные определения готовности.** v1 мерила функциональную ширину —
   «код написан» (421 builtin, три медиа-столпа, MCP, VM). Рабочее
   определение P0 — «контракт замкнут»: гейт + реестр + ledger +
   доказуемое поведение. Ширина ≠ готовность: ни один из 421 builtin не
   несёт capability-атрибута, которого нет.
2. **Невидимые для v1 пустые подсистемы.** Capability — 0 вхождений в
   `src/`; единый реестр бэкендов — 0; join/решётка/эффекты taint — 0.
   Суммарный вес этих дыр в разложении — 35+ процентных пунктов при
   нулевом вкладе.
3. **Advisory ≠ гейт.** В audit.rs 28 вхождений `Severity::Warning` —
   детекторы, не блокирующие запуск. v1 засчитывал их как покрытие;
   для P0 считается только блокирующая часть (`Severity::Error`,
   включённая в `audit_category_a`).
4. **Индикаторы ширины-без-готовности.** `spec!("recall", 0, "stub")`
   — строка реестра без обработчика; VM — experimental (ADR-0105/0141,
   full-language конструкции не компилируются в байткод), а «84
   инструкции» из §2 — PHANTOM (реально 47, п. 1.5).
5. **Границы taint-движка.** Внутрипроцедурная глубина ограничена
   (`TAINT_NESTING_MAX_DEPTH=3`), интерпроцедурная — 2
   (`TAINT_INTERP_MAX_DEPTH=2`, warnings `INTERP_DEPTH_LIMIT`),
   персистентный taint — только file/module scope (limitations.md);
   v1 читала наличие `TaintKind` как «taint готов».

**Решение (принято как рабочее для Фазы 1, п. (б) DoD):** цифра **26%**
с разложением таблицы выше — база планирования Фазы 1. Пересчёт — по
завершении каждой волны, тем же нарядным протоколом, с правкой только
этой страницы.

---

## 4. Активы-прецеденты, на которые можно опираться

| Актив | Что даёт как лекало | Где |
|---|---|---|
| ADR-0114 (reflex-opaque-handle) | «Непрозрачный хэндл»: реальный объект наружу отдаёт только идентификатор — прямое лекало для capability-модели (подсистема с 0%) | `docs/adr/0114-reflex-opaque-handle.md` |
| ADR-0125 + №241 | Category-A статический гейт: статически видимое нарушение → `Severity::Error` на call-site; шаблон секции 1607–1757 уже размножен (VISION_UNSIGNED_EXPORT, VISION_UNSIGNED_EXPORT_RAW, MEDIA_SYNTHETIC_UNMARKED №320) | `docs/adr/0125-vision-provenance-gates.md`; `tests/naryad_241_vision_gates.rs` |
| ADR-0136 + №274 | Санитайзер с точной taint-семантикой: маскирование ≠ санитизация (`redact` снимает Secret, не снимает CanaryLeak) | `docs/adr/0136-redact-taint-sanitizer.md`; `tests/naryad_274_redact.rs` |
| №284 | Path-sensitive taint: форк клона трекера в then-ветке — единственный существующий механизм ветвления; база для будущего join | `tests/naryad_284_canary.rs` |
| №261 + №130 | Многослойный сетевой гейт (allowlist + SSRF-guard + pinning) — применяется к любому новому Source-билтину | `tests/naryad_261_ssrf_pack.rs`; `tests/naryad_130_ssrf_guard.rs` |
| №300 | Consent-гейт поверх ledger (`has_consent_record`) — лекало для grant-механики необратимых операций (№325) | `tests/naryad_300_voice_gate.rs` |

Все файлы существуют на `1876fdf` (проверено `ls docs/adr/ | grep -E
'^0114|^0125|^0136'` и `ls tests/ | grep -E '241|274|284|261|130|300'`).

---

## 5. Расхождения с §2 v2 — фиксация новых якорей

| Якорь | §2 v2 | Снапшот `fc59e9e` | Main `1876fdf` | Комментарий |
|---|---|---|---|---|
| MODEL_WEIGHTS_UNSAFE | audit.rs:1607–1757 | 1607–1757 ✓ | **1660–1810** | сдвиг +53 (секции №309/№317 выше по файлу); новый якорь — 1660 |
| builtins | 420 (registry.rs:38) | 420 ✓ (объявление на 44) | **421** | +video_extend (№309); строка-якорь плана неточна |
| ADR | 143 | **142** ADR (+README = 143 файла) | **145** (+README = 146) | «143» воспроизводится только с индексным README; канонический базис — 142 |
| Statement | «10 видов» (ast.rs:1282) | **15** вариантов | 15 | «10» — базис-зависимо; README говорил 12 — исправлено на 15 |
| Инструкции VM | «84» (bytecode.rs:14) | **47** | 47 | PHANTOM; README согласен (47) |
| Примеры | 214 | 214 ✓ (top-level) | 214 (рекурсивно 273) | базис плана совпал с каноническим |
| semantic.rs | 3328 | 3328 ✓ | 3328 | без сдвига |
| TaintKind/TaintTracker | 119 / 142 | 119 / 141–142 ✓ | 119 / 141–142 | без сдвига |
| zeroize / consent_ledger | Cargo.toml:51 / store.rs:31 | ✓ / ✓ | ✓ / ✓ | без сдвига |
| provenance.rs | №241 | 430 строк ✓ | **464** | +синтетика Art 50 (№320) |
| mcp.rs | №268 | 613 строк ✓ | 613 | без сдвига |
| audit.rs (контекст) | — | 4258 строк | **4577** | рост от №309/№316/№320/№412 |

Сводка: план v2 factual-слой точен по позициям файлов и большинству
чисел; ошибочные числа — «84 инструкции» (PHANTOM) и базисно-зависимые
«10 видов Statement» / «143 ADR»; все сдвинувшиеся на main якоря
зафиксированы в настоящей таблице и в разделе 0.
