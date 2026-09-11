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
//! (protocolVersion `2025-03-26`, понимает ответы 2024-11-05…2025-11-25);
//! modern-only серверы (ревизия 2026-07-28 без legacy-режима) не обслуживаются.
//! Пагинация tools/list (`nextCursor`), `structuredContent`, sampling/resources
//! — не входят в v1 и вскрываются громкими ошибками, не тишиной.
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
use super::json::json_value_to_mlog_value;
use super::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Таймаут НА ФАЗУ протокола (handshake / tools/list / tools/call), сек.
/// `METALOGOS_MCP_TIMEOUT_SECS`, дефолт 30, кламп 1..=300
/// (паттерн `METALOGOS_EXEC_TIMEOUT_SECS`).
const MCP_DEFAULT_TIMEOUT_SECS: u64 = 30;
const MCP_MAX_TIMEOUT_SECS: u64 = 300;

/// Legacy-версия, которую клиент объявляет в `initialize` (ADR-0132 D1).
const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

/// Версии, которые v1-клиент принимает в ответе сервера. Ответ с версией вне
/// списка — громкий отказ (совместимость legacy-клиента с modern-only сервером
/// не заявлена — матрица спеки, ADR-0132 D1).
const MCP_KNOWN_VERSIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-11-25"];

// ── Процесс-хозяйство ────────────────────────────────────────────────

/// Владелец дочернего MCP-процесса. Drop-гарант: закрыть stdin, подождать до
/// 500 мс, затем kill — stateless-модель не оставляет сирот ни на одном пути
/// выхода (включая `?` и панику раскруткой).
struct McpProcess {
    child: Child,
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        // Спека stdio-транспорта: shutdown = закрыть поток. Закрытие stdin
        // (уже сделано полем stdin перед этим Drop — порядок полей McpConn)
        // даёт серверу шанс завершиться самому.
        match self.child.try_wait() {
            Ok(Some(_)) => return,
            _ => {}
        }
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Живое соединение с MCP-сервером на один вызов билтина.
/// Порядок полей ВАЖЕН: `stdin` объявлен раньше `_proc` — при Drop сначала
/// закрывается пайп (graceful), затем срабатывает килл-гарант.
/// `_proc` не читается напрямую: его смысл — владельческий Drop-гарант.
struct McpConn {
    stdin: ChildStdin,
    _proc: McpProcess,
    rx: Receiver<std::io::Result<String>>,
    next_id: u64,
}

impl McpConn {
    /// Spawn сервера + поток-читатель stdout. Наследует env интерпретатора
    /// (паритет с `exec()`/`exec_argv()` — см. доку модуля, пункт 5).
    fn spawn(command: &str, argv: &[String]) -> Result<Self, String> {
        let mut cmd = Command::new(command);
        cmd.args(argv)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "[MCP_SPAWN_FAILED] cannot spawn MCP server `{}`: {}",
                command, e
            )
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "[MCP_SPAWN_FAILED] MCP server stdin is not piped".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "[MCP_SPAWN_FAILED] MCP server stdout is not piped".to_string())?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if tx.send(line).is_err() {
                    break; // получатель умер — процесс закрывается
                }
            }
        });
        Ok(McpConn {
            stdin,
            _proc: McpProcess { child },
            rx,
            next_id: 1,
        })
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Отправка JSON-RPC сообщения (request или notification) одной строкой.
    fn send(&mut self, msg: &serde_json::Value, phase: &str) -> Result<(), String> {
        let mut line = serde_json::to_string(msg).map_err(|e| {
            format!(
                "[MCP_PROTOCOL_ERROR] phase={}: cannot serialize request: {}",
                phase, e
            )
        })?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.flush())
            .map_err(|e| {
                format!(
                    "[MCP_IO_ERROR] phase={}: cannot write to MCP server stdin (server died?): {}",
                    phase, e
                )
            })
    }

    /// Чтение ответа с таймаутом НА ФАЗУ. Игнорирует нотификации и чужие id
    /// (v1 не подписывается на серверные запросы — ADR-0132 D4); JSON-RPC
    /// error на наш id — громкий отказ с кодом сервера.
    fn recv(&self, id: u64, phase: &str, timeout: Duration) -> Result<serde_json::Value, String> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| {
                    format!(
                        "[MCP_TIMEOUT] phase={}: no response within {}s",
                        phase,
                        timeout.as_secs()
                    )
                })?;
            match self.rx.recv_timeout(remaining) {
                Ok(Ok(line)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let msg: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
                        format!(
                            "[MCP_PROTOCOL_ERROR] phase={}: MCP server stdout is not valid JSON-RPC (garbage on stdout?): {}",
                            phase, e
                        )
                    })?;
                    let is_response = msg.get("id").and_then(|v| v.as_u64()) == Some(id);
                    if is_response {
                        if let Some(err) = msg.get("error") {
                            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                            let message =
                                err.get("message").and_then(|m| m.as_str()).unwrap_or("");
                            return Err(format!(
                                "[MCP_PROTOCOL_ERROR] phase={}: MCP server returned JSON-RPC error {}: {}",
                                phase, code, message
                            ));
                        }
                        return Ok(msg.get("result").cloned().unwrap_or(serde_json::Value::Null));
                    }
                    // Нотификация/чужой id — не наш ответ, читаем дальше.
                }
                Ok(Err(e)) => {
                    return Err(format!(
                        "[MCP_IO_ERROR] phase={}: MCP server stdout stream broken: {}",
                        phase, e
                    ))
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(format!(
                        "[MCP_TIMEOUT] phase={}: no response within {}s",
                        phase,
                        timeout.as_secs()
                    ))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!(
                        "[MCP_IO_ERROR] phase={}: MCP server closed stdout before responding (crashed?)",
                        phase
                    ))
                }
            }
        }
    }

    /// Legacy-handshake: `initialize` → проверка версии → `notifications/initialized`.
    fn handshake(&mut self, timeout: Duration) -> Result<(), String> {
        let id = self.next_id();
        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "metalogos", "version": env!("CARGO_PKG_VERSION")}
            }
        });
        self.send(&init, "handshake")?;
        let result = self.recv(id, "handshake", timeout)?;
        let server_version = result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                "[MCP_PROTOCOL_ERROR] phase=handshake: initialize response lacks protocolVersion"
                    .to_string()
            })?;
        if !MCP_KNOWN_VERSIONS.contains(&server_version) {
            return Err(format!(
                "[MCP_PROTOCOL_ERROR] phase=handshake: server speaks protocol version {} — client v1 supports legacy versions {:?} only (modern-only servers are out of v1 scope, ADR-0132 D1)",
                server_version, MCP_KNOWN_VERSIONS
            ));
        }
        // Нотификация подтверждения — без id, ответа не ждём (спека legacy).
        let note = serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        self.send(&note, "handshake")
    }
}

// ── Гейты и конфигурация ─────────────────────────────────────────────

/// Таймаут на фазу: `METALOGOS_MCP_TIMEOUT_SECS`, дефолт 30, кламп 1..=300.
fn mcp_timeout() -> Duration {
    let secs = std::env::var("METALOGOS_MCP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(MCP_DEFAULT_TIMEOUT_SECS)
        .clamp(1, MCP_MAX_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// Allowlist серверных команд `METALOGOS_MCP_ALLOWLIST` (ADR-0132 D3,
/// форматная конвенция №259). Точное совпадение argv[0] (команды как записана).
/// Unset — не сужает (действует только exec-гейт); пустая строка — deny all;
/// непустая — только перечисленные, отказ `MCP_NOT_ALLOWLISTED`.
fn mcp_allowlist_gate(command: &str) -> Result<(), String> {
    match std::env::var("METALOGOS_MCP_ALLOWLIST") {
        Err(_) => Ok(()),
        Ok(raw) => {
            let entries: Vec<&str> = raw
                .split(',')
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .collect();
            if entries.is_empty() {
                return Err(
                    "[MCP_NOT_ALLOWLISTED] METALOGOS_MCP_ALLOWLIST is set to an empty value — \
                     all MCP servers are denied (Naryad #268, ADR-0132 D3)"
                        .to_string(),
                );
            }
            if entries.iter().any(|e| *e == command) {
                Ok(())
            } else {
                Err(format!(
                    "[MCP_NOT_ALLOWLISTED] MCP server `{}` is not in METALOGOS_MCP_ALLOWLIST (allowed: {})",
                    command,
                    entries.join(",")
                ))
            }
        }
    }
}

// ── Протокольные операции ────────────────────────────────────────────

/// `tools/list` → List[Struct{name, description, input_schema}].
/// Пагинация (`nextCursor`) — громкий отказ (Future в v1).
fn mcp_tools_list(conn: &mut McpConn, timeout: Duration) -> Result<Value, String> {
    let id = conn.next_id();
    let req = serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/list", "params": {}});
    conn.send(&req, "tools/list")?;
    let result = conn.recv(id, "tools/list", timeout)?;
    if result.get("nextCursor").map_or(false, |c| !c.is_null()) {
        return Err(
            "[MCP_PROTOCOL_ERROR] phase=tools/list: server paginates tool list (nextCursor) — \
             pagination is not supported in v1 (ADR-0132 D4, Future)"
                .to_string(),
        );
    }
    let tools = result
        .get("tools")
        .and_then(|t| t.as_array())
        .ok_or_else(|| {
            "[MCP_PROTOCOL_ERROR] phase=tools/list: result.tools must be an array".to_string()
        })?;
    let mut out: Vec<Value> = Vec::with_capacity(tools.len());
    for tool in tools {
        let name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| {
                "[MCP_PROTOCOL_ERROR] phase=tools/list: tool entry lacks `name`".to_string()
            })?
            .to_string();
        let description = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let input_schema = match tool.get("inputSchema") {
            Some(schema) => json_value_to_mlog_value(schema),
            None => Value::Unit,
        };
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Value::String(name));
        fields.insert("description".to_string(), Value::String(description));
        fields.insert("input_schema".to_string(), input_schema);
        out.push(Value::Struct {
            type_name: "Tool".to_string(),
            fields,
        });
    }
    Ok(Value::List(out))
}

/// `tools/call` → String (конкатенация text-блоков result.content).
/// isError=true → `MCP_TOOL_ERROR` с текстом сервера; JSON-RPC error с кодом
/// -32602 в фазе tools/call → `MCP_TOOL_NOT_FOUND` (Invalid params на вызов
/// инструмента = неизвестный инструмент); structuredContent — громкий отказ.
fn mcp_tools_call(
    conn: &mut McpConn,
    tool: &str,
    arguments: serde_json::Value,
    timeout: Duration,
) -> Result<Value, String> {
    let id = conn.next_id();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}
    });
    conn.send(&req, "tools/call")?;
    let result = match conn.recv(id, "tools/call", timeout) {
        Ok(r) => r,
        Err(e) => {
            // Код -32602 (Invalid params) в tools/call = неизвестный инструмент
            // (спека: Unknown tool). Переименование фазы — честная диагностика,
            // код сервера сохраняется в тексте.
            if e.contains("phase=tools/call") && e.contains("error -32602") {
                return Err(e.replacen(
                    "[MCP_PROTOCOL_ERROR] phase=tools/call",
                    "[MCP_TOOL_NOT_FOUND] phase=tools/call",
                    1,
                ));
            }
            return Err(e);
        }
    };
    if result.get("isError").and_then(|v| v.as_bool()) == Some(true) {
        let text = collect_text_blocks(&result).unwrap_or_default();
        return Err(format!(
            "[MCP_TOOL_ERROR] tool `{}` reported isError=true: {}",
            tool,
            if text.is_empty() {
                "(server gave no detail)"
            } else {
                &text
            }
        ));
    }
    if result
        .get("structuredContent")
        .map_or(false, |v| !v.is_null())
    {
        return Err(
            "[MCP_PROTOCOL_ERROR] phase=tools/call: server returned structuredContent — \
             structured tool output is not supported in v1 (ADR-0132 D4, Future)"
                .to_string(),
        );
    }
    let content = result.get("content").ok_or_else(|| {
        "[MCP_PROTOCOL_ERROR] phase=tools/call: result.content is missing".to_string()
    })?;
    if !content.is_array() {
        return Err(
            "[MCP_PROTOCOL_ERROR] phase=tools/call: result.content must be an array".to_string(),
        );
    }
    let text = collect_text_blocks(&result).unwrap_or_default();
    Ok(Value::String(text))
}

/// Конкатенация text-блоков content (type=="text"). Не-text блоки (image,
/// audio, resource) в v1 молча НЕ теряются — они просто не дают текста;
/// полное отсутствие content-массива ловится вызывающей стороной.
fn collect_text_blocks(result: &serde_json::Value) -> Option<String> {
    let content = result.get("content")?.as_array()?;
    let mut parts: Vec<String> = Vec::new();
    for block in content {
        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                parts.push(text.to_string());
            }
        }
    }
    Some(parts.join("\n"))
}

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
        return Err(format!("{}(): {}", builtin, e));
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
            Err(format!("{}(): {}", builtin, e))
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
