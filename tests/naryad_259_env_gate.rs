// ── НАРЯД №259: env-гейт serve-контекста ────────────────────────────
//
// Контракт (issue #275, docs/naryads-258-265-audit-tails.md §259):
// 1. serve-роут вызывает env("FAKE_API_SECRET_TOKEN") без флагов →
//    500/ошибка со стабильным кодом ENV_NOT_PERMITTED (TW и VM);
// 2. serve + METALOGOS_ENV_ALLOWLIST=FAKE_API_SECRET_TOKEN → читается;
// 3. serve + METALOGOS_SERVE_ALLOW_ENV=1 → читается всё;
// 4. процесс-контекст (mlog run) → читается как раньше (регрессия):
//    allowlist/флаги вне serve НЕ нужны;
// 5. SSOT env_gate: семантика «замена»/альтернатива напрямую (юнит).
//
// Механизм: переиспользован SSOT №253-А — тот же thread-local
// serve-контекст (`ServeRouteExecGuard` в spawn_blocking обоих роут-путей),
// отдельный флаг-хак НЕ вводился. Тела роутов в тестах сравнивают
// прочитанное значение с ожидаемым и отдают КОНСТАНТУ: утечка env-значения
// в respond() — это SECRET_LEAK (Category A, compile-time error), реальный
// serve такой роут не поднял бы; гейт проверяется до чтения, форма роута
// отражает легальный прод-код.
//
// Аудит-инвариант теста: до №259 проба env("FAKE_API_SECRET_TOKEN") в
// serve-роуте возвращала sk-supersecret без каких-либо флагов.

#![cfg(feature = "server")]
// Служебное исключение (громко, по правилам репо — лекало naryad_253):
// serve-тесты обязаны держать ENV_LOCK через .await — env-переменные
// процесса читаются серверными потоками во время запроса, и снятие замка
// между set_var и HTTP-запросом открыло бы гонку с параллельным тестом.
#![allow(clippy::await_holding_lock)]

use metalogos::builtins::{env_gate, ExecContext};
use metalogos::server::ServeBackend;
use std::sync::Mutex;

// Лекало naryad_253/naryad_244: env-переменные процесса глобальны —
// сериализуем тесты одного файла одним poison-tolerant мьютексом,
// держим его ВСЁ тело теста, включая негативные кейсы (урок №261:
// позитив соседнего потока подсадил флаг негативному кейсу).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const ENV_ROUTE_SOURCE: &str = r#"
mlogserver {
  port: 8094
  route "/envread" method=GET {
    let v = env("FAKE_API_SECRET_TOKEN")
    if v == "sk-supersecret-259" {
      return respond("200", "ENV_READ_OK")
    } else {
      return respond("200", "ENV_READ_MISS")
    }
  }
}
"#;

const PROCESS_ENV_PROGRAM: &str = r#"
pattern EnvRead(input: String) -> String {
  return "token=" + env("FAKE_API_SECRET_TOKEN")
}
entity start: String = ""
flow Main { input: String = start -> EnvRead -> output }
"#;

const SECRET_NAME: &str = "FAKE_API_SECRET_TOKEN";
const SECRET_VALUE: &str = "sk-supersecret-259";

async fn start_server(
    backend: ServeBackend,
) -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    metalogos::server::run_test_server_with_backend_in_dir(ENV_ROUTE_SOURCE, backend, base_dir)
        .await
        .expect("test server should start")
}

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body should be readable");
    (status, body)
}

/// Чистит env-состояние наряда: секретная переменная и оба эскейп-хэтча.
fn clean_env_state() {
    std::env::remove_var(SECRET_NAME);
    std::env::remove_var("METALOGOS_SERVE_ALLOW_ENV");
    std::env::remove_var("METALOGOS_ENV_ALLOWLIST");
}

// ── 1. TW: serve + нет флагов → 500 + ENV_NOT_PERMITTED ─────────────

#[tokio::test]
async fn tw_serve_route_env_denied_without_flags() {
    let _env = lock_env();
    clean_env_state();

    let (port, _handle) = start_server(ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/envread").await;

    assert_eq!(
        status, 500,
        "env() в serve-роуте без флагов должен быть отказан громко: {}",
        body
    );
    assert!(
        body.contains("ENV_NOT_PERMITTED"),
        "ошибка должна нести стабильный код ENV_NOT_PERMITTED, got: {}",
        body
    );
    assert!(
        body.contains("METALOGOS_SERVE_ALLOW_ENV=1"),
        "текст должен называть точный флаг «разрешить всё», got: {}",
        body
    );
    assert!(
        body.contains("METALOGOS_ENV_ALLOWLIST"),
        "текст должен называть точный allowlist-флаг, got: {}",
        body
    );
    // Гейт стоит ДО чтения: отказ идентичен и для несуществующей переменной —
    // ошибка не должна стать оракулом существования имён.
    let (_, body_absent) = http_get(port, "/envread").await;
    assert!(
        body_absent.contains("ENV_NOT_PERMITTED"),
        "got: {}",
        body_absent
    );
}

// ── 2. VM: паритет отказа с TW ──────────────────────────────────────

#[tokio::test]
async fn vm_serve_route_env_denied_without_flags() {
    let _env = lock_env();
    clean_env_state();

    let (port, _handle) = start_server(ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/envread").await;

    assert_eq!(status, 500, "VM-путь: env() без флагов отказан: {}", body);
    assert!(
        body.contains("ENV_NOT_PERMITTED"),
        "VM-путь: код ENV_NOT_PERMITTED в ошибке, got: {}",
        body
    );
}

// ── 3. TW: METALOGOS_ENV_ALLOWLIST открывает точечное чтение ────────

#[tokio::test]
async fn tw_serve_route_env_allowlist_reads() {
    let _env = lock_env();
    clean_env_state();
    std::env::set_var(SECRET_NAME, SECRET_VALUE);
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", SECRET_NAME);

    let (port, _handle) = start_server(ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/envread").await;

    assert_eq!(
        status, 200,
        "allowlist должен открыть чтение перечисленного имени: {}",
        body
    );
    assert!(
        body.contains("ENV_READ_OK"),
        "переменная должна прочитаться с точным значением, got: {}",
        body
    );

    // Мягкость чтения сохранена: allowlist-имя, НЕ заданное в процессе,
    // читается как пустая строка (soft-контракт env()), а не как отказ.
    std::env::remove_var(SECRET_NAME);
    let (status_miss, body_miss) = http_get(port, "/envread").await;
    assert_eq!(status_miss, 200, "got: {}", body_miss);
    assert!(
        body_miss.contains("ENV_READ_MISS"),
        "незаданное allowlist-имя — soft empty string, got: {}",
        body_miss
    );
}

// ── 4. TW: METALOGOS_SERVE_ALLOW_ENV=1 разрешает всё в serve ────────

#[tokio::test]
async fn tw_serve_route_env_allow_all_flag_reads() {
    let _env = lock_env();
    clean_env_state();
    std::env::set_var(SECRET_NAME, SECRET_VALUE);
    std::env::set_var("METALOGOS_SERVE_ALLOW_ENV", "1");

    let (port, _handle) = start_server(ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/envread").await;

    assert_eq!(
        status, 200,
        "METALOGOS_SERVE_ALLOW_ENV=1 должен открыть чтение без allowlist: {}",
        body
    );
    assert!(body.contains("ENV_READ_OK"), "got: {}", body);
}

// ── 5. VM: allowlist работает и на VM-пути (паритет) ────────────────

#[tokio::test]
async fn vm_serve_route_env_allowlist_reads() {
    let _env = lock_env();
    clean_env_state();
    std::env::set_var(SECRET_NAME, SECRET_VALUE);
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", SECRET_NAME);

    let (port, _handle) = start_server(ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/envread").await;

    assert_eq!(status, 200, "VM-путь: allowlist открывает чтение: {}", body);
    assert!(body.contains("ENV_READ_OK"), "got: {}", body);
}

// ── 6. Процесс-контекст (mlog run): читается как раньше, регрессия ──

#[test]
fn process_context_reads_env_as_before() {
    let _env = lock_env();
    clean_env_state();
    std::env::set_var(SECRET_NAME, SECRET_VALUE);

    // Никаких флагов/allowlist — процесс-контекст гейта не имеет (№259:
    // поведение вне serve не меняется). Имя, которое в serve потребовало
    // бы allowlist, вне serve читается БЕЗ него.
    let out = metalogos::run_program(PROCESS_ENV_PROGRAM);
    assert!(
        matches!(&out, Ok(Some(s)) if s.contains(SECRET_VALUE)),
        "mlog run должен читать env как раньше, без каких-либо флагов, got: {:?}",
        out
    );

    // Allowlist — механизм serve-контекста: его наличие/отсутствие в
    // процесс-контексте ни на что не влияет (замена, не AND).
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", "SOME_OTHER_NAME");
    let out2 = metalogos::run_program(PROCESS_ENV_PROGRAM);
    assert!(
        matches!(&out2, Ok(Some(s)) if s.contains(SECRET_VALUE)),
        "allowlist не должен быть нужен вне serve (и не должен мешать), got: {:?}",
        out2
    );
}

// ── 7. SSOT env_gate: семантика альтернатив напрямую ────────────────

#[test]
fn env_gate_replacement_semantics() {
    let _env = lock_env();
    clean_env_state();

    // ServeRoute без флагов → отказ с кодом
    let err = env_gate(ExecContext::ServeRoute, SECRET_NAME).unwrap_err();
    assert!(
        err.contains("ENV_NOT_PERMITTED"),
        "код ENV_NOT_PERMITTED в отказе, got: {}",
        err
    );

    // Эскейп-хэтч 1: разрешить всё (даже имя вне allowlist)
    std::env::set_var("METALOGOS_SERVE_ALLOW_ENV", "1");
    assert!(env_gate(ExecContext::ServeRoute, SECRET_NAME).is_ok());
    assert!(env_gate(ExecContext::ServeRoute, "ANY_OTHER_NAME").is_ok());
    std::env::remove_var("METALOGOS_SERVE_ALLOW_ENV");

    // Эскейп-хэтч 2: точный allowlist
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", "MODEL,TEMP");
    assert!(env_gate(ExecContext::ServeRoute, "MODEL").is_ok());
    assert!(env_gate(ExecContext::ServeRoute, "TEMP").is_ok());
    // не входит → отказ; частичные совпадения не считаются
    assert!(env_gate(ExecContext::ServeRoute, SECRET_NAME).is_err());
    assert!(env_gate(ExecContext::ServeRoute, "TE").is_err());
    assert!(env_gate(ExecContext::ServeRoute, "TEMPERATURE").is_err());
    // пробелы по краям элементов срезаются
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", "MODEL, TEMP ,");
    assert!(env_gate(ExecContext::ServeRoute, "TEMP").is_ok());
    // пустой список = deny всех
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", "");
    assert!(env_gate(ExecContext::ServeRoute, "MODEL").is_err());
    std::env::remove_var("METALOGOS_ENV_ALLOWLIST");

    // Процесс-контекст: гейта нет — с флагами и без, всегда Ok
    // (замена, не AND: allowlist в процесс-контексте НЕ нужен)
    assert!(env_gate(ExecContext::Process, SECRET_NAME).is_ok());
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", SECRET_NAME);
    assert!(env_gate(ExecContext::Process, SECRET_NAME).is_ok());
    std::env::set_var("METALOGOS_SERVE_ALLOW_ENV", "1");
    assert!(env_gate(ExecContext::Process, SECRET_NAME).is_ok());
    clean_env_state();
}
