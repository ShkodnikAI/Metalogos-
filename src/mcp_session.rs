// ── src/mcp_session.rs — the MCP stdio session client (№793) ──────────
//
// Naryad №793 (issue #1531, Wave 64 P1 — the gateway G1, the C1 (b)
// owner decision, ADR-0196): the MCP contour gains a SESSION layer.
// The pre-№793 client (src/builtins/mcp.rs, ADR-0132) spawned a server
// process per builtin call — stateless by construction; stateful
// upstreams (a browser, a db session, a file watcher) were unusable
// and every call paid the process start.
//
// WHAT LIVES HERE:
//   - the connection primitives MOVED from src/builtins/mcp.rs
//     verbatim in behavior (McpProcess/McpConn, the framing, the
//     handshake, tools/list, tools/call, the timeout and the
//     allowlist gates) — the stateless builtins keep their exact
//     per-call contract through the same primitives (the back-compat
//     form the naryad requires);
//   - the protocol revision: the client now DECLARES `2025-11-25`
//     (the current revision; it was already in the accepted set) and
//     refuses a version outside the known set LOUDLY, naming both
//     versions — the modern-only servers are served, the legacy
//     fallback stays (the negotiation matrix, ADR-0132 D1 updated by
//     the №793 scope);
//   - `McpSession` — ONE live connection held for the caller's
//     lifetime: the gateway upstream pool form. A dead session is
//     reconnected LOUDLY (stderr, never silent) on the next use; a
//     spawn failure is a loud typed refusal, not a degraded answer;
//   - `parse_upstreams_spec` — the METALOGOS_MCP_UPSTREAMS grammar
//     (`name=argv0,arg1,...;...`, the №259 format convention, read
//     once at the gateway startup — the №263 discipline).
//
// The gates ride the session, not the caller: the allowlist gate on
// argv[0] and the exec gate stay the caller's responsibility where
// they were (the builtins gate before spawn — №253-А; the gateway
// gates at the declaration parse — №793), the taint discipline is
// untouched (the results are untrusted data, ADR-0132 D4).

use crate::builtins::json::json_value_to_mlog_value;
use crate::interpreter::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Таймаут НА ФАЗУ протокола (handshake / tools/list / tools/call), сек.
/// `METALOGOS_MCP_TIMEOUT_SECS`, дефолт 30, кламп 1..=300
/// (паттерн `METALOGOS_EXEC_TIMEOUT_SECS`).
const MCP_DEFAULT_TIMEOUT_SECS: u64 = 30;
const MCP_MAX_TIMEOUT_SECS: u64 = 300;

/// Актуальная ревизия, которую клиент объявляет в `initialize` (№793:
/// the current revision — the upgrade from the legacy-only
/// `2025-03-26` declaration; the negotiation matrix ADR-0132 D1).
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

/// Версии, которые клиент принимает в ответе сервера. Ответ с версией
/// вне списка — громкий отказ с названием обеих версий (совместимость
/// с неизвестной ревизией не заявлена — матрица спеки).
pub const MCP_KNOWN_VERSIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-11-25"];

// ── Процесс-хозяйство ────────────────────────────────────────────────

/// Владелец дочернего MCP-процесса. Drop-гарант: закрыть stdin, подождать до
/// 500 мс, затем kill — stateless-модель не оставляет сирот ни на одном пути
/// выхода (включая `?` и панику раскруткой).
pub struct McpProcess {
    pub(crate) child: Child,
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        // Спека stdio-транспорта: shutdown = закрыть поток. Закрытие stdin
        // (уже сделано полем stdin перед этим Drop — порядок полей McpConn)
        // даёт серверу шанс завершиться самому.
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
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

/// Живое соединение с MCP-сервером. В v1-билтинах — на один вызов
/// (stateless, ADR-0132 D4); в сессионном слое (№793) — удерживается
/// `McpSession` на время жизни шлюза.
/// Порядок полей ВАЖЕН: `stdin` объявлен раньше `_proc` — при Drop сначала
/// закрывается пайп (graceful), затем срабатывает килл-гарант.
pub struct McpConn {
    pub(crate) stdin: ChildStdin,
    pub(crate) _proc: McpProcess,
    pub(crate) rx: Receiver<std::io::Result<String>>,
    pub(crate) next_id: u64,
    /// №793: the reader thread sets this when the stdout stream ENDS
    /// (EOF/exit) — the deterministic death signal that closes the
    /// reap-race window (a child can sit unreaped for a beat after
    /// exit; the pipe EOF is the fact).
    pub(crate) stream_ended: Arc<AtomicBool>,
}

impl McpConn {
    /// Spawn сервера + поток-читатель stdout. Наследует env интерпретатора
    /// (паритет с `exec()`/`exec_argv()` — см. доку модуля, пункт 5).
    pub fn spawn(command: &str, argv: &[String]) -> Result<Self, String> {
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
        let stream_ended = Arc::new(AtomicBool::new(false));
        let ended_flag = Arc::clone(&stream_ended);
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if tx.send(line).is_err() {
                    break; // получатель умер — процесс закрывается
                }
            }
            ended_flag.store(true, Ordering::SeqCst);
        });
        Ok(McpConn {
            stdin,
            _proc: McpProcess { child },
            rx,
            next_id: 1,
            stream_ended,
        })
    }

    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Отправка JSON-RPC сообщения (request или notification) одной строкой.
    pub fn send(&mut self, msg: &serde_json::Value, phase: &str) -> Result<(), String> {
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
    pub fn recv(
        &self,
        id: u64,
        phase: &str,
        timeout: Duration,
    ) -> Result<serde_json::Value, String> {
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

    /// Handshake: `initialize` (объявляем актуальную ревизию — №793) →
    /// проверка версии → `notifications/initialized`.
    pub fn handshake(&mut self, timeout: Duration) -> Result<(), String> {
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
                "[MCP_PROTOCOL_ERROR] phase=handshake: server speaks protocol version {} — the client supports {:?} only; the mismatch is a loud refusal naming both versions (№793, ADR-0132 D1)",
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
pub fn mcp_timeout() -> Duration {
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
pub fn mcp_allowlist_gate(command: &str) -> Result<(), String> {
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
            if entries.contains(&command) {
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
pub fn mcp_tools_list(conn: &mut McpConn, timeout: Duration) -> Result<Value, String> {
    let id = conn.next_id();
    let req = serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/list", "params": {}});
    conn.send(&req, "tools/list")?;
    let result = conn.recv(id, "tools/list", timeout)?;
    if result.get("nextCursor").is_some_and(|c| !c.is_null()) {
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
        let schema = match tool.get("inputSchema") {
            Some(s) if !s.is_null() => json_value_to_mlog_value(s, "$")?,
            _ => Value::Unit,
        };
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Value::String(name));
        fields.insert("description".to_string(), Value::String(description));
        fields.insert("input_schema".to_string(), schema);
        out.push(Value::Struct {
            type_name: "McpTool".to_string(),
            fields,
        });
    }
    Ok(Value::List(out))
}

/// `tools/call` → String (конкатенация text-блоков result.content).
/// isError=true → `MCP_TOOL_ERROR` с текстом сервера; JSON-RPC error с кодом
/// -32602 в фазе tools/call → `MCP_TOOL_NOT_FOUND` (Invalid params на вызов
/// инструмента = неизвестный инструмент); structuredContent — громкий отказ.
pub fn mcp_tools_call(
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
        .is_some_and(|v| !v.is_null())
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
pub fn collect_text_blocks(result: &serde_json::Value) -> Option<String> {
    let content = result.get("content")?.as_array()?;
    let mut parts = Vec::new();
    for block in content {
        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                parts.push(text.to_string());
            }
        }
    }
    Some(parts.join("\n"))
}

// ── Сессионный слой (№793): долгоживущий upstream ────────────────────

/// Долгоживущая сессия с ОДНИМ объявленным upstream-сервером (шлюз G1).
/// Один процесс на upstream на время жизни держателя (шлюза); смерть
/// процесса обнаруживается по факту (try_wait) и лечится громким
/// переподключением на следующем использовании — never silent.
pub struct McpSession {
    name: String,
    command: String,
    argv: Vec<String>,
    conn: Option<McpConn>,
}

impl McpSession {
    /// Объявленная сессия (без spawn — первый `ensure_alive` поднимает
    /// процесс; шлюз делает это eagerly на старте и громко печатает деградацию).
    pub fn new(name: &str, command: &str, argv: Vec<String>) -> Result<Self, String> {
        if name.is_empty() {
            return Err("[MCP_PROTOCOL_ERROR] upstream declaration: name is empty".to_string());
        }
        if command.is_empty() {
            return Err(format!(
                "[MCP_PROTOCOL_ERROR] upstream `{}`: argv[0] (command) is empty",
                name
            ));
        }
        Ok(McpSession {
            name: name.to_string(),
            command: command.to_string(),
            argv,
            conn: None,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// Живое соединение: поднимает процесс при первом использовании и
    /// громко переподключает после смерти upstream'а. Ошибка spawn/handshake —
    /// typed громкий отказ (the caller records the reason).
    pub fn ensure_alive(&mut self) -> Result<&mut McpConn, String> {
        let was_live = self.conn.is_some();
        let dead = match &mut self.conn {
            None => true,
            Some(c) => {
                matches!(c._proc.child.try_wait(), Ok(Some(_)))
                    || c.stream_ended.load(Ordering::SeqCst)
            }
        };
        if dead {
            if was_live {
                eprintln!(
                    "[mcp-upstream] session `{}` ({} {:?}) died — reconnecting loudly (№793: never silent)",
                    self.name, self.command, self.argv
                );
            }
            self.conn = None;
            let mut conn = McpConn::spawn(&self.command, &self.argv)?;
            conn.handshake(mcp_timeout())?;
            self.conn = Some(conn);
        }
        // The invariant: after the dead-branch the connection was JUST
        // established; the alive-branch holds it by definition. The typed
        // arm is unreachable — the honest form, not an unwrap.
        self.conn.as_mut().ok_or_else(|| {
            "[MCP_IO_ERROR] phase=ensure_alive: session state lost (unreachable)".to_string()
        })
    }

    /// `tools/list` через живую сессию.
    pub fn tools_list(&mut self) -> Result<Value, String> {
        let conn = self.ensure_alive()?;
        mcp_tools_list(conn, mcp_timeout())
    }

    /// `tools/call` через живую сессию.
    pub fn tools_call(
        &mut self,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<Value, String> {
        let conn = self.ensure_alive()?;
        mcp_tools_call(conn, tool, arguments, mcp_timeout())
    }

    /// Пометить сессию мёртвой, если ошибка — класс сломанного потока
    /// (EOF/закрытый stdout): следующий `ensure_alive` переподключится
    /// громко. Сам вызов НЕ ретраится автоматически (у вызова могла быть
    /// сторона на upstream'е — решение за вызывающим).
    fn mark_dead_on_stream_error(&mut self, err: &str) {
        if err.contains("closed stdout") || err.contains("stream broken") {
            self.conn = None;
        }
    }

    /// `tools/list` в JSON-виде — для агрегации шлюзом (префиксация имён
    /// остаётся JSON-плоскостью, без Value-конверсии туда-обратно).
    pub fn tools_list_json(&mut self) -> Result<serde_json::Value, String> {
        let conn = self.ensure_alive()?;
        let id = conn.next_id();
        let req =
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/list", "params": {}});
        conn.send(&req, "tools/list")?;
        let result = conn.recv(id, "tools/list", mcp_timeout())?;
        if result.get("nextCursor").is_some_and(|c| !c.is_null()) {
            return Err(
                "[MCP_PROTOCOL_ERROR] phase=tools/list: server paginates tool list (nextCursor) — \
                 pagination is not supported in v1 (ADR-0132 D4, Future)"
                    .to_string(),
            );
        }
        match result.get("tools").cloned().ok_or_else(|| {
            "[MCP_PROTOCOL_ERROR] phase=tools/list: result.tools is missing".to_string()
        }) {
            Ok(v) => Ok(v),
            Err(e) => {
                self.mark_dead_on_stream_error(&e);
                Err(e)
            }
        }
    }

    /// `tools/call` через живую сессию, текст ответа (конкатенация
    /// text-блоков) — форма ответа шлюза совпадает с локальными tool'ами.
    pub fn tools_call_text(
        &mut self,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<String, String> {
        let outcome = (|| -> Result<String, String> {
            let conn = self.ensure_alive()?;
            let v = mcp_tools_call(conn, tool, arguments, mcp_timeout())?;
            Ok(match v {
                Value::String(s) => s,
                other => format!("{}", other),
            })
        })();
        match outcome {
            Ok(text) => Ok(text),
            Err(e) => {
                self.mark_dead_on_stream_error(&e);
                Err(e)
            }
        }
    }
}

/// Грамматика `METALOGOS_MCP_UPSTREAMS` (№793): записи через `;`, каждая
/// `name=argv0,arg1,...` (форматная конвенция №259 — запятая внутри argv,
/// пробелы вокруг полей обрезаются). Имя — `[a-z0-9_-]+` (префикс `name.tool`
/// в агрегированном tools/list обязан быть однозначным). Пустая строка /
/// unset — upstream'ов нет (переменная читается ОДИН раз на старте шлюза —
/// дисциплина №263).
pub fn parse_upstreams_spec(raw: &str) -> Result<Vec<McpSession>, String> {
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for entry in raw.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, argv_raw) = entry.split_once('=').ok_or_else(|| {
            format!(
                "[MCP_PROTOCOL_ERROR] METALOGOS_MCP_UPSTREAMS: entry `{}` lacks `name=argv` form",
                entry
            )
        })?;
        let name = name.trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(format!(
                "[MCP_PROTOCOL_ERROR] METALOGOS_MCP_UPSTREAMS: name `{}` must be non-empty [a-z0-9_-] (the `name.tool` prefix must stay unambiguous)",
                name
            ));
        }
        if seen.iter().any(|s| s == name) {
            return Err(format!(
                "[MCP_PROTOCOL_ERROR] METALOGOS_MCP_UPSTREAMS: duplicate upstream name `{}`",
                name
            ));
        }
        seen.push(name.to_string());
        let argv: Vec<String> = argv_raw
            .split(',')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(str::to_string)
            .collect();
        if argv.is_empty() {
            return Err(format!(
                "[MCP_PROTOCOL_ERROR] METALOGOS_MCP_UPSTREAMS: upstream `{}` has an empty argv (need at least argv[0])",
                name
            ));
        }
        out.push(McpSession::new(name, &argv[0], argv[1..].to_vec())?);
    }
    Ok(out)
}
