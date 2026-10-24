//! MCP (Model Context Protocol) stdio-клиент — Наряд №268, ADR-0132 (Accepted).
//!
//! Два stateless-билтина: `mcp_call` и `mcp_list_tools`. На КАЖДЫЙ вызов:
//! spawn сервера → legacy `initialize`-handshake → (`tools/call` | `tools/list`)
//! → shutdown (закрытие stdin, затем wait/kill — сирот не остаётся на любом
//! пути выхода, включая паники: `McpProcess` Drop-гарант).
//!
//! Транспорт (ADR-0132 D1): newline-delimited JSON-RPC 2.0 поверх
//! `std::process::Child` stdin/stdout. Сообщения не содержат встроенных `\n`
//! (serde_json-сериализация это гарантирует). stderr сервера — логи: в v1
//! направляется в null (спека: клиент SHOULD NOT считать его ошибкой).
//!
//! Граница v1 (ADR-0132 D1/D4, честно): клиент говорит legacy-диалект
//! Граница v1 (ADR-0132 D1/D4, №793): клиент объявляет актуальную ревизию
//! `2025-11-25` (сессионный слой `src/mcp_session.rs`), принимает известные
//! ревизии 2024-11-05…2025-11-25, несовпадение — громкий отказ с названием
//! версий. Пагинация tools/list (`nextCursor`), `structuredContent`,
//! sampling/resources — не входят в v1 и вскрываются громкими ошибками, не
//! тишиной. Долгоживущие upstream-сессии шлюза живут в `src/mcp_session.rs`
//! (`McpSession`) — здесь билтины остаются stateless (обратная совместимость).
//!
//! Security (ADR-0132 D3, решение владельца от 2026-09-12):
//! 1. Exec-гейт: SSOT `exec_gate(current_exec_context())` (наряд №253-А) —
//!    `METALOGOS_ALLOW_EXEC=1` в процесс-контексте, `METALOGOS_SERVE_ALLOW_EXEC=1`
//!    в телах роутов (замена, не AND). Код отказа — `EXEC_NOT_PERMITTED`.
//! 2. Allowlist: `METALOGOS_MCP_ALLOWLIST` — comma-separated, trim, пустые
//!    элементы игнорируются (конвенция №259), точное совпадение argv[0].
//!    Unset — не сужает; пустая строка — deny all; непустая — только
//!    перечисленные. Отказ — `MCP_NOT_ALLOWLISTED` (конвенция ADR-0131).
//! 3. Taint: результат `mcp_call` — недоверенные данные `TaintKind::UserInput`
//!    (reuse; решение владельца). Метаданные `mcp_list_tools` (имена,
//!    descriptions, inputSchema) — без taint: в security-sinks они не попадают,
//!    решение включить descriptions в LLM-контекст принимает программа явно.
//! 4. Аудит: каждый разрешённый spawn — запись в `METALOGOS_AUDIT_LOG_PATH`
//!    (operation `mcp_call` / `mcp_list_tools`), тот же канал, что и `exec()`.
//! 5. Наследование env: дочерний процесс наследует окружение интерпретатора —
//!    ровно как `exec()`/`exec_argv()` (паритет, «reuse, не новая политика»).
//!    ENV-гейт №259 регулирует `env()`-чтения в телах роутов, а не наследование
//!    окружения детьми; развёртываниям, которым нужна изоляция, следует
//!    запускать интерпретатор с редуцированным окружением (задокументировано
//!    в threat-model и REFERENCE).
//!
//! Ошибки — громкие, с фазой и (для протокольных ошибок) кодом сервера:
//! `MCP_SPAWN_FAILED`, `MCP_TIMEOUT`, `MCP_IO_ERROR`, `MCP_PROTOCOL_ERROR`,
//! `MCP_TOOL_NOT_FOUND`, `MCP_TOOL_ERROR` (+`EXEC_NOT_PERMITTED`,
//! `MCP_NOT_ALLOWLISTED` от гейтов). Все коды идут в тексте ошибки — тот же
//! режим, что у №253/№259 до посадки реестра диагностик ADR-0131.

use super::io::{append_subprocess_audit, current_exec_context, exec_gate};
use super::Value;
use crate::interpreter::values::wrap_error_preserving_code;
use crate::mcp_session::{
    mcp_allowlist_gate, mcp_timeout, mcp_tools_call, mcp_tools_list, McpConn,
};
use std::time::Duration;

// ── Билтины ──────────────────────────────────────────────────────────

/// Общий скелет: exec-гейт → allowlist → spawn → handshake → `op` → shutdown.
/// Аудит: каждый spawn ПОСЛЕ гейтов — запись с исходом (ok / ERROR).
/// Shutdown гарантирован Drop'ом `McpConn` на любом пути.
fn with_mcp_server<T>(
    builtin: &str,
    command: &str,
    argv: &[String],
    op: impl FnOnce(&mut McpConn, Duration) -> Result<T, String>,
) -> Result<T, String> {
    let timeout = mcp_timeout();
    let mut conn = match McpConn::spawn(command, argv) {
        Ok(c) => c,
        Err(e) => {
            append_subprocess_audit(
                builtin,
                &mcp_audit_detail(command, argv),
                &format!("ERROR: {}", e),
            );
            // №413: the spawn failure already carries its own origin stamp
            // (`[MCP_SPAWN_FAILED] …`, whitelisted for `try`) — pass it
            // through untouched.
            return Err(e);
        }
    };
    // Handshake (фаза 1). Любая ошибка — аудит + громкий отказ; процесс
    // закроется Drop'ом при выходе из функции.
    if let Err(e) = conn.handshake(timeout) {
        append_subprocess_audit(
            builtin,
            &mcp_audit_detail(command, argv),
            &format!("ERROR: {}", e),
        );
        // №413: keep the origin stamp at position 0 — the naive prefix
        // buried `[MCP_PROTOCOL_ERROR]` mid-message and demoted the try
        // classification to RUNTIME_ERROR.
        return Err(wrap_error_preserving_code(&format!("{}()", builtin), &e));
    }
    match op(&mut conn, timeout) {
        Ok(v) => {
            append_subprocess_audit(builtin, &mcp_audit_detail(command, argv), "ok");
            Ok(v)
        }
        Err(e) => {
            append_subprocess_audit(
                builtin,
                &mcp_audit_detail(command, argv),
                &format!("ERROR: {}", e),
            );
            // №413: stamp-preserving wrap (see the handshake site above).
            Err(wrap_error_preserving_code(&format!("{}()", builtin), &e))
        }
    }
}

/// Строка detail для audit-лога: команда + argv + (опц.) инструмент.
fn mcp_audit_detail(command: &str, argv: &[String]) -> String {
    if argv.is_empty() {
        command.to_string()
    } else {
        format!("{} {:?}", command, argv)
    }
}

fn value_as_string(builtin: &str, pos: usize, v: &Value, what: &str) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        _ => Err(format!(
            "{}(): argument {} must be a String ({})",
            builtin, pos, what
        )),
    }
}

fn value_as_argv(builtin: &str, v: &Value) -> Result<Vec<String>, String> {
    match v {
        Value::List(items) => items
            .iter()
            .map(|item| match item {
                Value::String(s) => Ok(s.clone()),
                _ => Err(format!(
                    "{}(): server args list must contain only Strings",
                    builtin
                )),
            })
            .collect(),
        _ => Err(format!(
            "{}(): server args must be a List of Strings",
            builtin
        )),
    }
}

/// №413 (issue #558, ADR-0169 §3.1 extension): the MCP contour's failure
/// taxonomy (`MCP_SPAWN_FAILED`, `MCP_TIMEOUT`, `MCP_IO_ERROR`,
/// `MCP_PROTOCOL_ERROR`, `MCP_TOOL_NOT_FOUND`, `MCP_TOOL_ERROR`,
/// `MCP_NOT_ALLOWLISTED`) already sits at position 0 of the error strings
/// the contour raises — it is now whitelisted for the `try` classifier in
/// `values::ORIGIN_STAMPED_CODES`. The wrapper below no longer buries
/// those stamps mid-message.
///
/// `mcp_call(command, args_list, tool_name, arguments_json) -> String`.
///
/// Вызов инструмента внешнего MCP-сервера (stdio, stateless — ADR-0132 D4):
/// spawn → initialize-handshake → tools/call → shutdown на КАЖДЫЙ вызов.
/// Возвращает конкатенацию text-блоков ответа. Результат — недоверенные
/// данные (`UserInput` taint, решение владельца): sinks (`respond`,
/// `write_file`, `http_post`, `reflex_train`) проверяются статикой.
/// Гейты: `exec_gate` (№253-А) + `METALOGOS_MCP_ALLOWLIST`; каждый spawn —
/// запись в `METALOGOS_AUDIT_LOG_PATH`.
pub(crate) fn builtin_mcp_call(args: &[Value]) -> Result<Value, String> {
    // Гейт ДО всего: отказ гейта не аудируется (паритет с exec()/exec_argv()).
    exec_gate(current_exec_context())?;
    if args.len() != 4 {
        return Err("mcp_call() requires exactly 4 arguments (command, args_list, tool_name, arguments_json)".to_string());
    }
    let command = value_as_string("mcp_call", 1, &args[0], "server command / argv[0]")?;
    let argv = value_as_argv("mcp_call", &args[1])?;
    let tool = value_as_string("mcp_call", 3, &args[2], "tool name")?;
    let arguments_json = value_as_string(
        "mcp_call",
        4,
        &args[3],
        "tool arguments as JSON object literal",
    )?;
    let arguments: serde_json::Value = serde_json::from_str(&arguments_json).map_err(|e| {
        format!(
            "mcp_call(): arguments_json is not valid JSON: {} (pass a JSON object literal, e.g. \"{{}}\")",
            e
        )
    })?;
    if !arguments.is_object() {
        return Err(format!(
            "mcp_call(): arguments_json must be a JSON OBJECT, got {}",
            json_kind(&arguments)
        ));
    }
    mcp_allowlist_gate(&command)?;
    with_mcp_server("mcp_call", &command, &argv, |conn, timeout| {
        mcp_tools_call(conn, &tool, arguments, timeout)
    })
}

/// `mcp_list_tools(command, args_list) -> List[Struct{name, description, input_schema}]`.
///
/// Реестр инструментов сервера (stdio, stateless): spawn → initialize-handshake
/// → tools/list → shutdown. Метаданные НЕ получают taint (ADR-0132 D3): они не
/// проходят через security-sinks; включение descriptions в LLM-контекст —
/// явное решение программы (поверхность промпт-инъекции зафиксирована в
/// threat-model). `input_schema` — Struct/Unit из JSON inputSchema сервера.
pub(crate) fn builtin_mcp_list_tools(args: &[Value]) -> Result<Value, String> {
    exec_gate(current_exec_context())?;
    if args.len() != 2 {
        return Err(
            "mcp_list_tools() requires exactly 2 arguments (command, args_list)".to_string(),
        );
    }
    let command = value_as_string("mcp_list_tools", 1, &args[0], "server command / argv[0]")?;
    let argv = value_as_argv("mcp_list_tools", &args[1])?;
    mcp_allowlist_gate(&command)?;
    with_mcp_server("mcp_list_tools", &command, &argv, mcp_tools_list)
}

/// Человекочитаемый род JSON-значения для громких ошибок.
fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}
