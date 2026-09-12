# Testing Evidence — Metalogos

Материалы для грантовых заявок (NLnet/Restack — раздел Testing Evidence):
сколько и какие свойства проверяются автоматически, с числами. Всё ниже
исполняется CI на каждый PR (blocking) или по расписанию (тренды).

## Property-based тесты (наряд №277, proptest)

Быстрые, детерминированные (seeds), зелёные в блокирующем CI:

| Файл | Свойства | Числа |
|---|---|---|
| `tests/property_builtin_nopanic.rs` | **No-panic по всем чистым билтинам**: случайные `Value`-аргументы (включая unicode, deep nesting до 6 уровней, граничные float) в случайные функции → либо значение, либо Result-ошибка, НЕ паника. Реестр (`BUILTIN_REGISTRY`) перечисляется в рантайме — новые билтины попадают в свип автоматически | **162 чистых билтина** покрываются напрямую (детерминированный свип + 128 proptest-кейсов); 23 stub-записи пропущены честно (вызов — громкая ошибка, пинится другими тестами); side-effectful категории (bot/web/io/email/llm/db/voice/…) исключены и перечислены в выводе теста |
| `tests/property_json_roundtrip.rs` | `json_encode` любого вложенного `Value` → валидный JSON; canonical stability (parse→encode стабилен со второго круга); `json_get` возвращает ровно те листья, которые были положены по построенным dot-путям; дефолт на произвольных путях — без паник | 256 кейсов × 3 свойства |
| `tests/property_string_invariants.rs` | `reverse∘reverse = id` на произвольном unicode; `len(s) == chars().count()` (документированная семантика); `substring/char_at` = посимвольные слайсы на всех границах; `escape_html` без сырых угловых скобок | 512 кейсов × 4 свойства |
| `tests/property_tw_vm_parity.rs` | Программы, сгенерированные из консервативного подмножества грамматики (литералы, арифметика, конкатенация, let, вызовы билтинов, вызовы паттернов), исполняются ИДЕНТИЧНО в TW и VM. Исключения — документированная граница ADR-0105 (`match`, `BlockIfElse`-as-value, memory/learnable/server/IO) | 192 кейса × 2 формы программ |

**Найдено и починено property-тестами сразу при написании (№277):** `strip()`
паниковал, когда оба конца строки полностью состояли из strip-символов
(`strip("&", "Ⱥ&")` → slice panic `start > len-end`); починен в
`builtin_strip`, минимизатор и обычные формы запинены тестом
`regression_strip_overlap_ends_no_panic`. Задокументированный (не краш):
`json_encode` печатает float с отклонением до 1 ulp от канонического
кратчайшего представления serde (наблюдение в J2-комментарии) — фиксация
поведения, не изменение.

## Mutational testing (наряд №277, cargo-mutants smoke)

- Цель: `src/builtins/json.rs` — плотная escaping/парсинг/навигация логика.
- Killer: `tests/property_json_roundtrip` (roundtrip-свойства ловят
  большинство мутаций сериализации).
- Прогон: еженедельно (понедельник 06:00 UTC) + ручной dispatch —
  `.github/workflows/mutants.yml`, НЕ блокирующий мерж, только тренд.
- Артефакт: `mutants.out/` + `mut-score.txt` (mut-score = killed / (killed +
  missed + timeouts)) — публикуется как GitHub Actions artifact каждого
  прогона.
- Почему не `src/audit.rs` (3232 строки) и не `src/builtins/string.rs`
  (905 строк) из постановки: часовой масштаб недельного прогона без роста
  ценности smoke-контракта; выбор модуля — осознанное отклонение,
  зафиксированное в шапке workflow и PR наряда.

## Смежные контуры (уже существовали)

- **Fuzzing** (наряд №256): 3 cargo-fuzz цели (парсер, байткод, url_decode),
  дымовой прогон в CI (2 мин/цель), non-blocking.
- **Blocking-набор**: 15 check-runs на каждый PR (lib/integration/crosscheck/
  candle/vision/registry-arity/llm-cache/minimal-build/fmt/clippy/ADR-numbering/
  module-size/vscode/cargo-audit/branch-freshness) — мерж только при
  полностью зелёном наборе на мерж-коммите.
- **Crosscheck**: TW vs VM parity — отдельный blocking-тест +
  property-расширение из №277 (см. выше).
