# Наряд №266 (P2, bug/parser) — memorize/relate/forget внутри pattern/route-тел: token soup вместо громкой ошибки парсера

**Факт.** Грамматика знает `memorize <expr> [with priority=F]`,
`relate ... to ... as ...`, `forget ...` только на верхнем уровне
(`memorize_decl`/`relate_decl`/`forget_decl` в `src/grammar.pest:182+`,
`declaration = ...`). Внутри `pattern_body = { statement* }` этих правил
НЕТ — и стейтмент-грамматика молча расщепляет строку на мусор
(верифицировано дампом AST на `naryad-264` @ 3c4ab45, файл
`examples/p8_route_patterns.mlog`, строка 20):

```
Stmt 0: ExprStmt { expr: Ident("memorize") }
Stmt 1: ExprStmt { expr: Ident("fact") }
Stmt 2: ExprStmt { expr: Ident("with") }
Stmt 3: Assign { name: "priority", value: FloatLit(0.8) }
```

Runtime-поведение TW при вызове паттерна (проба на main @ 29f0592):

```
$ mlog run probe_mem.mlog        # flow вызывает Remember("hello")
error: undefined variable: memorize      exit=1
```

Пример `examples/p8_route_patterns.mlog` (Contract 3 «pattern with memory
from route») никогда не работал end-to-end — он только компилировался;
роут `/remember` вернул бы 500 на первом POST. До №264 это было молча:
`mlog check` пропускал token soup, компилятор компилировал Assign, TW
падал только в рантайме при вызове. №264 (статическая проверка
неизменяемости) вытащил мусор на статку — `mlog check` теперь честно
ошибся «cannot assign to immutable variable: priority» на примере, что и
вскрыло настоящий корень. `relate`/`forget` внутри тел — тот же класс по
грамматике (не воспроизводилось отдельно, но правило одно).

**Задача.**
1. Грамматика: разрешить `memorize_decl`/`relate_decl`/`forget_decl`
   как стейтменты внутри `pattern_body` (и route-тел — они тоже
   `statement*`): `pattern_body = { statement* }` расширить
   (`statement += memory_stmt` с меморайз-ветками), лексалы
   `MEMORIZE_KW`/`RELATE_KW`/`FORGET_KW` переиспользовать. Семантика —
   как у top-level деклараций (запись в память сессии/БД по контракту
   memory-билтинов).
2. Альтернатива (если вывод стейтмента-меморайза в исполнение —
   непропорционален): ЗАПРЕТИТЬ громко — парсер должен падать с
   понятной ошибкой «memorize is only allowed at top level» вместо
   token soup. Выбранный вариант зафиксировать в PR ГРОМКО с
   обоснованием.
3. В любом случае: тест на оба файла — минимальный паттерн с
   `memorize fact with priority=0.8` внутри тела (парсится И
   исполняется — или громко не парсится с внятным текстом), и
   `examples/p8_route_patterns.mlog` возвращается к Contract 3
   (restore строки `memorize fact with priority=0.8` в теле Remember —
   она убрана truth-up'ом №264 с комментарием-указателем на этот наряд).

**§3.** `src/grammar.pest`, `src/parser/stmt.rs`/`decl.rs` (в зависимости
от варианта), `src/semantic.rs` (если стейтмент-меморайз требует
семантики), `src/interpreter`/`execution.rs` (исполнение), examples/p8,
REFERENCE (раздел памяти: где разрешён memorize).

**Сделано, когда:** проба-факт перевёрнута: pattern с `memorize ... with
priority=` внутри тела либо работает (TW и VM одинаково) либо не
парсится с громким внятным текстом — третий исход «молчаливый token
soup» исключён тестом; p8 restored и зелёный на check+run; существующие
memory-тесты (p7/m4/контракты) зелёные; CI blocking зелёный.

**Связь.** Вскрыто исполнением №264 (issue #280); приоритет P2, потому
что молчаливый парс-мусор — класс « dishonest silence», но конструкция
никогда не была обещана работающей (REFERENCE описывает memorize на
верхнем уровне).
