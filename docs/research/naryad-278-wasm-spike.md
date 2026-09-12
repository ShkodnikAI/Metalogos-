# Naryad №278 — WASM-разведка Playground: гигиена зависимостей, блокеры ядра, Go/No-Go

**Дата:** 2026-09-12
**Диспатч:** #316, волна 4 (issue #314)
**Связь:** `docs/refactoring-split-plan.md` (наряд №37 Block 2), ADR-0105 (VM experimental scope), №111 (фичевые гейты), №271 п.4 (sqlite-vec под wasm)
**Вердикт:** **No-Go** для Playground в текущей форме → путь к Go через хирургию зависимостей (план ниже).

## 1. Что проверено фактом (снапшот main 26fd63ac, волна 4)

### 1.1. Гигиена tokio — ПОДТВЕРЖДЕНА use-графом и ЗЕМЛЕНА на main

`tokio` был жёсткой зависимостью (`features = ["full"]`, не optional), хотя
его используют только:

- `src/server.rs` (модуль уже целиком `#[cfg(feature = "server")]`):
  `use tokio::sync::RwLock`, `tokio::spawn`, `tokio::net::TcpListener`,
  `tokio::task::spawn_blocking`, `tokio::sync::Mutex<rusqlite::Connection>`, `#[tokio::test]`;
- `src/main.rs` → `cmd_serve` (функция уже `#[cfg(feature = "server")]`,
  как и ветка `Commands::Serve`): единственный `tokio::runtime::Builder`.

Вне server-стека (`grep 'tokio::' src/`): только doc-комментарий в
`builtins/http.rs`. Вывод use-графа: **tokio нужен только фиче `server`**.

Изменение на main (одна строка + комментарий): `tokio` → `optional = true`,
`server = [..., "dep:tokio"]`. Ядро (парсер/компилятор/VM) и бинарь `mlog`
собираются без него — `cargo check --bin mlog --no-default-features
--features "svg,chart,diagram,template,llm"` зелёный; blocking-джоба
`minimal-build` в CI теперь проверяет эту конфигурацию на каждый PR.
`mlog-lsp` — отдельный крейт со своим tokio, не затронут.

**Числа:**

| Метрика | Было (tokio жёстко) | Стало (tokio за `server`) |
|---|---|---|
| Крейтов в нормальном графе (default features) | 357 | 357 |
| Крейтов без server (`--no-default-features --features "svg,chart,diagram,template,llm"`) | — | **339 (−18)** |
| Ушедшие из графа | — | `axum`, `axum-core`, `axum-macros`, `matchit`, `http-body-util`, `serde_path_to_error`, `serde_urlencoded`, `tower-http`, `tokio-macros`, `tokio-util`, `h2`, `parking_lot(+core)`, `lock_api`, `signal-hook-registry`, `errno`, `fnv` |
| Чистая debug-сборка lib (этот контейнер, cold) | 153 s | 140 s (−13 s, ~8.5 %) |

**Честная оговорка:** tokio САМ остался в графе без server — его тянет
`reqwest` → `hyper` → `tokio` (async-клиент под LLM/HTTP/voice билтины).
Гигиена убрала server-стек и сделала зависимость явной, но полный уход
tokio из не-server сборок упирается в reqwest — см. блокеры ниже. Замер
времени — cold debug `cargo build --lib` в контейнере исполнения (не
GitHub-hosted раннер; базовые ~60 s из FEATURE_INTAKE §5 мерялись в другой
конфигурации — сравнивать абсолютные числа между машинами нельзя, только
дельту на одной машине: **−13 s / −18 крейтов**).

### 1.2. wasm32-unknown-unknown — фактический список блокеров ядра

`rustup target add wasm32-unknown-unknown`; проверка:

```
cargo check --lib --target wasm32-unknown-unknown --no-default-features
```

Целевой граф зависимостей под wasm (--no-default-features, только ядро +
жёсткие зависимости): **303 крейта**. Первый жёсткий стоп — **build script
`openssl-sys v0.9.117`** (exit 101: нет OpenSSL для wasm-таргета). Полный
набор wasm-несовместимых семейств в целевом графе (по инверсиям
`cargo tree -i`, факты резолва + известная поддержка таргетов):

| Семейство | Тянет | Причина блокера |
|---|---|---|
| `openssl-sys` / `openssl` / `native-tls` | `imap`, `lettre`, напрямую metalogos | C-OpenSSL; build script падает на wasm (наблюдаемый первый стоп) |
| `imap` (+`imap-proto`) | email-билтины | нативный TLS-стек |
| `lettre` | email-билтины (SMTP) | нативный TLS-стек |
| `reqwest` → `hyper`/`hyper-util`/`tokio`/`h2` | http/llm/voice билтины | reqwest не поддерживает `wasm32-unknown-unknown` (только web-таргеты через wasm-bindgen; blocking-клиента под wasm нет) |
| `rusqlite` / `libsqlite3-sys` | memory-слой (memory_store, learnable cache persistence) | bundled C-sqlite не собирается под wasm |
| `getrandom 0.4` | crypto-крейты | под `unknown-unknown` требует wasm_js-конфигурацию; дефолт не собирается |
| `socket2` / `libc`-сетевые примитивы | tokio/hyper | нет ОС-сокетов под wasm |
| `tokio` | reqwest/hyper | под wasm частичен (time/util без net); блокер в составе стека reqwest |

Что чисто и wasm-совместимо (это ядро Playground): `pest`/`pest_derive`
(парсер), компилятор, VM-интерпретатор (ADR-0105-скоуп), ast/bytecode,
чистые билтины (string/list/math/json/crypto-примитивы/calendar/time/encoding/svg/chart/diagram —
последние генерируют строки без сети).

## 2. Вердикт: **No-Go** (для Playground в текущей форме)

Критерий Go из постановки — «ядро собирается, .wasm ≤ 5 MB gz, демо
работает». Ядро **не собирается**: 8 семейств блокеров на жёстких
зависимостях, первый — `openssl-sys` (наблюдаемый build-script стоп).
Замер размера .wasm не производился — сборки нет, мерить нечего.

## 3. Путь к Go (план следующего наряда, поверх №37 Block 2)

1. **Хирургия зависимостей** (главный шаг, ценен и без wasm):
   - `imap` + `lettre` → optional за фичей `email` (email-билтины
     регистрируются всегда, handlers дают громкий `FEATURE_DISABLED`-класс
     ошибки без фичи — конвенция №111);
   - `reqwest` → optional за фичей `http` (http/llm/voice билтины);
   - `rusqlite` → optional за фичей `memory` (in-memory HashMap-фоллбэк
     или громкий отказ — решить отдельным ADR);
   - `native-tls` уходит вместе с ними; для wasm сеть всё равно исключена
     (см. п. 2).
2. **Повторный wasm-check** после шага 1: ожидаемый остаток — `getrandom`
   (включить `wasm_js`-конфигурацию для crypto-примитивов) и мелочь;
   ядро + чистые билтины собираются.
3. **cdylib-эксперимент**: временный `crate-type = ["cdylib"]` (в спайке,
   не на main), `run_program`-обвязка `textarea → run → output`, замер
   `.wasm` gz — критерий Go **< 5 MB gz**.
4. **Демо** на GitHub Pages (статическая страница, 3–5 примеров из
   `examples/`, без бэкенда) — отдельный наряд Playground-реализации.

Пока шаги 1–3 не сделаны, приоритет — `docs/refactoring-split-plan.md`
(№37 Block 2): разделение ядра структурно решает и wasm-путь.
