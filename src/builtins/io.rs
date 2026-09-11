// ── I/O builtins: print, env, file operations, exec, git_push, html_render ──
//
// Наряд №88: exec() hardened with timeout + file audit log.
// html_render added — browser-based HTML→image via exec_restricted (no shell).
// Audit log (append_subprocess_audit) used by both exec and html_render.

use crate::interpreter::Value;

use super::core::expect_string_arg;

use std::io::Write;
use std::time::Duration;

// ── Constants for exec() hardening (Наряд №88 Блок 1) ──

/// Default timeout for `exec()` — 30 seconds.
/// Matches the http_post default (Наряд-26 P0-1).
const EXEC_DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Upper ceiling for `exec()` timeout — 5 minutes.
/// Same category as http_post max (300s).
const EXEC_MAX_TIMEOUT_SECS: u64 = 300;

/// Default timeout for `html_render()` — 30 seconds.
/// Browser rendering may take longer than a typical shell command;
/// 30s accommodates complex pages with inline images while still
/// bounding resource usage.
const HTML_RENDER_DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Upper ceiling for `html_render()` timeout — 120 seconds.
/// Intentionally more generous than exec() because browser rendering
/// of complex self-contained HTML (inline SVG, data: URIs) can be slow.
const HTML_RENDER_MAX_TIMEOUT_SECS: u64 = 120;

// ── File audit log (Наряд №88 Блок 1.2 / Блок 3) ──

/// Append a line to the subprocess audit log.
///
/// Format: `{iso_timestamp}\t{operation}\t{detail}\t{exit_status}\n`
///
/// Path controlled by `METALOGOS_AUDIT_LOG_PATH` env var,
/// defaults to `metalogos_subprocess_audit.log` in the working directory.
///
/// **Soft-failure:** errors writing to the audit log do NOT propagate
/// to the caller — same category as append_file, consistent with the
/// language's I/O soft-failure convention.
///
/// pub(crate) с наряда №268: MCP-билтины (`src/builtins/mcp.rs`) пишут spawn
/// своего сервера в тот же audit-канал (ADR-0132 D3 — reuse, не дублировать).
pub(crate) fn append_subprocess_audit(operation: &str, detail: &str, exit_status: &str) {
    let path = std::env::var("METALOGOS_AUDIT_LOG_PATH")
        .unwrap_or_else(|_| "metalogos_subprocess_audit.log".to_string());

    let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%z");
    let line = format!(
        "{}\t{}\t{}\t{}\n",
        timestamp, operation, detail, exit_status
    );

    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
    // Soft-failure: audit log write error must NOT crash the operation.
}

// ── exec_restricted (Наряд №88 Блок 1.3) ──

/// Execute a binary directly with an argument array — NO shell interpretation.
///
/// This is the safe alternative to `sh -c` string concatenation.
/// Arguments are passed as separate OS array elements, so shell
/// metacharacters in arguments (;, |, $(), etc.) are NOT interpreted.
/// This closes the entire class of shell injection vulnerabilities.
///
/// Used internally by `html_render` and available for future builtins
/// that need controlled subprocess execution.
fn exec_restricted(
    binary: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let mut cmd = std::process::Command::new(binary);
    cmd.args(args);

    // Spawn the process and apply timeout manually.
    // std::process::Command does not natively support timeout,
    // so we use the spawn + wait_with_timeout pattern via
    // a child process + thread-based timeout.
    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("exec_restricted: failed to spawn '{}': {}", binary, e))?;

    // Use try_wait in a loop with a 100ms poll interval.
    // This avoids pulling in an external crate just for child timeout.
    let deadline = std::time::Instant::now() + timeout;
    let mut child = child;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process has exited — collect output
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("exec_restricted: failed to read output: {}", e))?;
                // wait_with_output already consumed the child, but status is what we got
                // Actually wait_with_output returns the final Output with the status.
                // We need to reconstruct: the status from try_wait may differ if
                // wait_with_output reaps differently. Let's just use wait_with_output's status.
                let _ = status; // use output.status instead
                return Ok(output);
            }
            Ok(None) => {
                // Still running — check deadline
                if std::time::Instant::now() >= deadline {
                    // Timeout — kill the child
                    let _ = child.kill();
                    let _ = child.wait(); // reap to avoid zombie
                    return Err(format!(
                        "exec_restricted: timeout after {}s for '{}'",
                        timeout.as_secs(),
                        binary
                    ));
                }
                // Brief sleep before polling again
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!(
                    "exec_restricted: failed to wait on '{}': {}",
                    binary, e
                ));
            }
        }
    }
}

// ── Public builtins ──

pub(crate) fn builtin_print(args: &[Value]) -> Result<Value, String> {
    // Наряд №114: refuse Secret / other opaque values with a clear message.
    // expect_string_arg would also reject Secret, but with a generic wording.
    if let Some(arg) = args.first() {
        if crate::interpreter::values::is_nonprintable(arg) {
            return Err(format!(
                "print() refused: {} values cannot be printed (Secret and other opaque types)",
                arg.type_name()
            ));
        }
    }
    let s = expect_string_arg("print", args, 0)?;
    eprintln!("[print] {}", s);
    Ok(Value::String(s))
}

pub(crate) fn builtin_env(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("env", args, 0)?;
    // Наряд №259: в serve-роут-контексте чтение переменных процесса
    // по умолчанию ЗАПРЕЩЕНО (громко, ENV_NOT_PERMITTED) — до №259 один
    // вызов env("...") в теле роута отдавал секреты процесса (LLM API-ключи,
    // ключи БД, токены деплоя) недоверенному коду. SSOT-гейт — env_gate;
    // контекст определяется той же thread-local механикой, что и у
    // exec_gate (№253-А): ServeRouteExecGuard в spawn_blocking роут-путей.
    // Вне serve (mlog run / check / repl / верхний уровень serve) поведение
    // НЕ меняется: локальный скрипт читает своё окружение — это контракт.
    env_gate(current_exec_context(), &key)?;
    match std::env::var(&key) {
        Ok(val) => Ok(Value::String(val)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure: empty string if not found
    }
}

/// Whether the file being sandbox-checked is expected to already exist.
/// When `ForRead`, the file itself is canonicalized.
/// When `ForWrite`, only the parent directory is canonicalized (the file
/// may not exist yet — e.g. `write_file("new.txt", ...)`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SandboxMode {
    ForRead,
    ForWrite,
}

/// Sanitize a file path for I/O sandbox.
///
/// Three-layer defence (Наряд №131):
///   1. Text checks: reject absolute paths and `..` components.
///   2. Symlink canonicalization: resolve the path and verify it stays
///      inside the sandbox base directory. For writes to new files,
///      canonicalize only the parent directory (the file may not exist).
///   3. Prefix check: the canonicalized path must start with the
///      canonicalized base directory.
///
/// Наряд №252: the returned path is now SAFE TO USE, not merely
/// checked. Previously the canonical path was validated but the
/// ORIGINAL string was returned — so the actual file operation
/// re-traversed any symlinks planted between the check and the use
/// (TOCTOU). Now:
///   - `ForRead` returns the canonicalized file path (reads resolve
///     through in-sandbox symlinks exactly as before, but can no
///     longer race past the check).
///   - `ForWrite` returns `<canonical parent>/<final component>`.
///     The parent is prefix-verified; the final component is the
///     caller's responsibility — pair with [`open_sandbox_write`],
///     which closes the final-component symlink race (O_NOFOLLOW).
pub(crate) fn sandbox_path(path: &str) -> Result<std::path::PathBuf, String> {
    sandbox_path_ex(path, SandboxMode::ForRead)
}

/// Наряд №254: классификация отказа `sandbox_path(ForRead)` — это «пути нет»
/// (мягкий исход, контракт сохранён), а не нарушение политики песочницы
/// (громкая ошибка `[SANDBOX_VIOLATION]`).
///
/// Различие принципиально: `read_file("опечатка.txt")` и
/// `read_file("../secrets")` до №254 были неотличимы — обе возвращали пустую
/// строку, маскируя дефект программы. Классификация:
///
/// - текстовые нарушения (абсолютный путь, `..`) — ВСЕГДА нарушение,
///   независимо от существования пути;
/// - `symlink_metadata` пути падает (файла нет; недоступный родитель) —
///   мягкий исход: «нечитаем — soft-failure как сегодня»;
/// - путь СУЩЕСТВУЕТ (включая битый symlink — metadata звена не следует
///   по ссылке), но canonicalize/префикс упали — нарушение (отказ №131
///   становится громким, не молчалкой).
pub(crate) fn sandbox_path_missing(path: &str) -> bool {
    let p = std::path::Path::new(path);
    if p.is_absolute()
        || p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false; // текстовые нарушения — громко
    }
    match std::env::current_dir() {
        Ok(base) => std::fs::symlink_metadata(base.join(p)).is_err(),
        Err(_) => false, // не можем stat — не рискуем: трактуем как нарушение
    }
}

/// Наряд №254: громкий формат отказа песочницы со стабильным кодом
/// `SANDBOX_VIOLATION` (конвенция ADR-0131: код — контракт, текст может меняться).
pub(crate) fn sandbox_violation(msg: impl std::fmt::Display) -> String {
    format!("[SANDBOX_VIOLATION] {}", msg)
}

/// Like `sandbox_path` but allows specifying whether the operation is
/// a read (file must exist) or a write (file may be new).
pub(crate) fn sandbox_path_ex(path: &str, mode: SandboxMode) -> Result<std::path::PathBuf, String> {
    let p = std::path::Path::new(path);
    // Reject absolute paths
    if p.is_absolute() {
        return Err(format!(
            "file I/O sandbox: absolute paths not allowed: '{}'",
            path
        ));
    }
    // Reject path traversal
    for component in p.components() {
        if let std::path::Component::ParentDir = component {
            return Err(format!(
                "file I/O sandbox: path traversal ('..') not allowed: '{}'",
                path
            ));
        }
    }

    // ── Наряд №131: symlink canonicalization ──
    // Resolve the base directory (cwd) and the target path.
    // Verify the resolved target stays inside the base.
    let base = std::env::current_dir().map_err(|e| format!("sandbox: {}", e))?;
    let canonical_base = base.canonicalize().map_err(|e| format!("sandbox: {}", e))?;

    let full = base.join(p);
    let canonical_target = match mode {
        // For reads: canonicalize the full path (file must exist).
        SandboxMode::ForRead => full.canonicalize(),
        // For writes: canonicalize only the parent dir; the file itself
        // may not exist yet (e.g. write_file to a new file).
        SandboxMode::ForWrite => match full.parent() {
            Some(parent) => parent.canonicalize(),
            None => return Err(format!("file I/O sandbox: path has no parent: '{}'", path)),
        },
    };

    let canonical_target = match canonical_target {
        Ok(c) => c,
        // If canonicalization fails (e.g. broken symlink), reject.
        Err(_) => {
            return Err(format!("file I/O sandbox: cannot resolve path: '{}'", path));
        }
    };

    if !canonical_target.starts_with(&canonical_base) {
        return Err(format!(
            "file I/O sandbox: resolved path escapes sandbox: '{}'",
            path
        ));
    }

    // Наряд №252: return a path that is safe to USE, not the original
    // string (TOCTOU fix — see doc comment above).
    match mode {
        SandboxMode::ForRead => Ok(canonical_target),
        SandboxMode::ForWrite => {
            let name = p
                .file_name()
                .ok_or_else(|| format!("file I/O sandbox: path has no file name: '{}'", path))?;
            Ok(canonical_target.join(name))
        }
    }
}

/// Наряд №252: symlink-safe open for sandbox write targets.
///
/// `target` must come from `sandbox_path_ex(_, SandboxMode::ForWrite)`
/// (canonical parent + final component).
///
/// Two-phase open closes the final-component TOCTOU:
///   1. `create_new(true)` — if the file is created fresh, no symlink
///      can sit at the final component (creation is atomic).
///   2. On `AlreadyExists`: canonicalize the full path (resolves any
///      symlink), re-verify the prefix against the sandbox base, then
///      reopen the CANONICAL path with `O_NOFOLLOW` (unix) — so a
///      symlink swapped in after the check cannot be followed.
///
/// Honest boundary: intermediate directory components swapped between
/// canonicalize and open are still out of scope (would need per-component
/// O_NOFOLLOW or Linux openat2 RESOLVE_BENEATH — revisit if a real
/// use case appears; planted-final-component file escape is the
/// reproduced class from №252).
///
/// Non-unix: step 2 opens the canonical path without O_NOFOLLOW
/// (symlink creation there requires elevated privileges; documented
/// boundary).
pub(crate) fn open_sandbox_write(
    target: &std::path::Path,
    append: bool,
) -> Result<std::fs::File, String> {
    let attempt = if append {
        std::fs::OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(target)
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
    };

    match attempt {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let base = std::env::current_dir()
                .map_err(|e| format!("file I/O sandbox: {}", e))?
                .canonicalize()
                .map_err(|e| format!("file I/O sandbox: {}", e))?;
            let canonical = target.canonicalize().map_err(|_| {
                sandbox_violation(format!(
                    "file I/O sandbox: cannot resolve path: '{}'",
                    target.display()
                ))
            })?;
            if !canonical.starts_with(&base) {
                return Err(sandbox_violation(format!(
                    "file I/O sandbox: resolved path escapes sandbox: '{}'",
                    target.display()
                )));
            }
            let mut opts = std::fs::OpenOptions::new();
            if append {
                opts.append(true);
            } else {
                opts.write(true);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.custom_flags(libc::O_NOFOLLOW);
            }
            opts.open(&canonical).map_err(|e| {
                format!(
                    "file I/O sandbox: cannot open '{}': {}",
                    target.display(),
                    e
                )
            })
        }
        Err(e) => Err(format!(
            "file I/O sandbox: cannot create '{}': {}",
            target.display(),
            e
        )),
    }
}

/// `read_file(path)` — read file contents as String.
/// Soft-failure: returns empty string when the file is missing or unreadable
/// (Наряд №254: the contract is preserved). Sandbox violations — absolute
/// paths, `..`, symlink escapes, broken symlinks — are a LOUD error with the
/// stable code `[SANDBOX_VIOLATION]` (ADR-0131): they are programmer errors,
/// not environmental failures, and swallowing them hid real defects.
pub(crate) fn builtin_read_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("read_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(e) => {
            // Наряд №254: разделение исходов. Файла нет / нечитаем —
            // мягкий отказ (контракт сохранён: пустая строка). Нарушение
            // песочницы (абсолютный путь, `..`, symlink-побег, битый
            // symlink) — громкая ошибка с кодом SANDBOX_VIOLATION:
            // это дефект программы, молча проглатывать его значит прятать баг.
            if sandbox_path_missing(&path) {
                return Ok(Value::String(String::new())); // soft-failure: файла нет
            }
            return Err(sandbox_violation(e));
        }
    };
    match std::fs::read_to_string(&safe_path) {
        Ok(content) => Ok(Value::String(content)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure (нечитаем)
    }
}

/// `write_file(path, content)` — write string to file (overwrite).
/// Returns "ok" on success, empty string on soft-failure.
pub(crate) fn builtin_write_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("write_file", args, 0)?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Ok(Value::String(String::new())), // soft-failure
    };
    // Наряд №131: ForWrite — file may not exist yet.
    // Наряд №252: sandbox violations are LOUD here (they are programmer
    // errors, not environmental failures).
    // Наряд №254: громкие отказы несут стабильный код SANDBOX_VIOLATION.
    // Ordinary OS-level write errors keep the soft-failure contract.
    let safe_path = sandbox_path_ex(&path, SandboxMode::ForWrite).map_err(sandbox_violation)?;
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    // sandbox escape / unresolvable target — loud
    let mut file = open_sandbox_write(&safe_path, false)?;
    match file.write_all(content.as_bytes()) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure (OS-level)
    }
}

/// `append_file(path, content)` — append string to file.
/// Returns "ok" on success, empty string on soft-failure.
pub(crate) fn builtin_append_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("append_file", args, 0)?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Ok(Value::String(String::new())), // soft-failure
    };
    // Наряд №131: ForWrite — file may not exist yet.
    // Наряд №252: sandbox violations are LOUD (see builtin_write_file).
    // Наряд №254: громкие отказы несут стабильный код SANDBOX_VIOLATION.
    let safe_path = sandbox_path_ex(&path, SandboxMode::ForWrite).map_err(sandbox_violation)?;
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    // sandbox escape / unresolvable target — loud
    let mut file = open_sandbox_write(&safe_path, true)?;
    match file.write_all(content.as_bytes()) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure (OS-level)
    }
}

/// `delete_file(path)` — delete a file.
/// Soft-failure: returns empty string when the file is missing/unreadable
/// (Наряд №254: preserved). Sandbox violations are a loud `[SANDBOX_VIOLATION]`.
pub(crate) fn builtin_delete_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("delete_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(e) => {
            // Наряд №254: тот же разбор, что и в read_file — файла нет:
            // мягкий отказ; нарушение песочницы: громко с кодом.
            if sandbox_path_missing(&path) {
                return Ok(Value::String(String::new())); // soft-failure: файла нет
            }
            return Err(sandbox_violation(e));
        }
    };
    match std::fs::remove_file(&safe_path) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `file_exists(path)` — check if a file exists. Returns Bool.
pub(crate) fn builtin_file_exists(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("file_exists", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::Bool(false)), // soft-failure on sandbox violation
    };
    Ok(Value::Bool(safe_path.exists()))
}

/// `list_dir(path)` — list files in a directory. Returns List of Strings.
pub(crate) fn builtin_list_dir(args: &[Value]) -> Result<Value, String> {
    let path = if args.is_empty() {
        ".".to_string()
    } else {
        expect_string_arg("list_dir", args, 0)?
    };
    let safe_path = sandbox_path(&path)?;
    let entries: Vec<Value> = std::fs::read_dir(&safe_path)
        .map_err(|e| format!("list_dir('{}'): {}", path, e))?
        .filter_map(|entry| {
            entry
                .ok()
                .map(|e| Value::String(e.file_name().to_string_lossy().to_string()))
        })
        .collect();
    Ok(Value::List(entries))
}

// ═══ Нaryad №253: exec-гейт serve-контекста (Вариант А владельца) ═══
//
// Политика:
// - Процесс-контекст (mlog run / check / верхний уровень serve — регистрация
//   роутов): требуется `METALOGOS_ALLOW_EXEC=1` (Наряд №97, без изменений).
// - Тело роута serve (обработчик HTTP-запроса): требуется ТОЛЬКО
//   `METALOGOS_SERVE_ALLOW_EXEC=1`. Семантика — «замена», не «AND»:
//   процесс-флаг на тела роутов НЕ распространяется. Код маршрута — часто
//   чужая/сгенерированная программа, поэтому наследование флага процесса
//   (дающее каждому роуту полный `sh -c`) закрыто.
//
// Механизм: тела роутов исполняются на выделенном blocking-потоке
// (spawn_blocking, ADR-0096). `ServeRouteExecGuard` (RAII) ставится первой
// строкой внутри spawn_blocking-замыкания обоих роут-путей
// (`execute_route_body`, `execute_route_body_vm`) в src/server.rs и
// помечает поток thread-local'ом на всё время исполнения тела.

/// Контекст вызова `exec()` / `exec_argv()` (Наряд №253, Вариант А).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExecContext {
    /// mlog run / check / верхний уровень serve: гейт `METALOGOS_ALLOW_EXEC=1`.
    Process,
    /// Тело роута serve: гейт `METALOGOS_SERVE_ALLOW_EXEC=1` (замена, не AND).
    ServeRoute,
}

thread_local! {
    static SERVE_ROUTE_CONTEXT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// RAII-сторож serve-роут-контекста (Наряд №253).
///
/// Пока жив — текущий поток считается serve-роут-контекстом
/// (`current_exec_context() == ExecContext::ServeRoute`). Ставится внутри
/// spawn_blocking-замыкания роут-хендлеров; Drop снимает метку при выходе
/// из замыкания (включая ранние `?`-возвраты и паники раскруткой).
pub struct ServeRouteExecGuard;

impl ServeRouteExecGuard {
    pub fn new() -> Self {
        SERVE_ROUTE_CONTEXT.with(|c| c.set(true));
        ServeRouteExecGuard
    }
}

impl Default for ServeRouteExecGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ServeRouteExecGuard {
    fn drop(&mut self) {
        SERVE_ROUTE_CONTEXT.with(|c| c.set(false));
    }
}

/// Контекст exec текущего потока (Наряд №253).
pub fn current_exec_context() -> ExecContext {
    if SERVE_ROUTE_CONTEXT.with(std::cell::Cell::get) {
        ExecContext::ServeRoute
    } else {
        ExecContext::Process
    }
}

/// SSOT-гейт `exec()` / `exec_argv()` (Наряд №253).
///
/// До №253 одно и то же условие дублировалось в `builtin_exec` и
/// `builtin_exec_argv` (политика №97). Теперь оба вызывают эту функцию;
/// контекст определяет, какой флаг требуется:
///
/// - [`ExecContext::Process`] → `METALOGOS_ALLOW_EXEC=1`;
/// - [`ExecContext::ServeRoute`] → `METALOGOS_SERVE_ALLOW_EXEC=1`
///   (процесс-флаг игнорируется — «замена», Вариант А).
///
/// Ошибки несут стабильный диагностический код `EXEC_NOT_PERMITTED`
/// (конвенция ADR-0131: UPPER_SNAKE_CASE, код — контракт, текст может
/// меняться) и называют точный флаг текущего контекста.
pub fn exec_gate(context: ExecContext) -> Result<(), String> {
    match context {
        ExecContext::ServeRoute => {
            if std::env::var("METALOGOS_SERVE_ALLOW_EXEC").unwrap_or_default() == "1" {
                Ok(())
            } else {
                Err(
                    "[EXEC_NOT_PERMITTED] exec() is denied in serve route handlers. \
                     In route bodies set METALOGOS_SERVE_ALLOW_EXEC=1 to allow exec; \
                     the process-level METALOGOS_ALLOW_EXEC flag does NOT apply to \
                     route bodies (Naryad #253, Variant A)."
                        .to_string(),
                )
            }
        }
        ExecContext::Process => {
            if std::env::var("METALOGOS_ALLOW_EXEC").unwrap_or_default() == "1" {
                Ok(())
            } else {
                Err("[EXEC_NOT_PERMITTED] exec() is disabled by default. Set \
                     METALOGOS_ALLOW_EXEC=1 to enable — this applies to mlog run, \
                     check, and serve top-level alike. For serve route bodies set \
                     METALOGOS_SERVE_ALLOW_EXEC=1 instead (Naryad #253)."
                    .to_string())
            }
        }
    }
}

// ═══ Наряд №259: env-гейт serve-контекста ══════════════════════════════
//
// Политика:
// - Процесс-контекст (mlog run / check / repl / верхний уровень serve):
//   БЕЗ гейта — локальный скрипт читает своё окружение, это контракт
//   (№259: поведение вне serve не меняется).
// - Тело роута serve: по умолчанию ЗАПРЕЩЕНО (громко, ENV_NOT_PERMITTED).
//   Эскейп-хэтчи с ЗАМЕНАЮЩЕЙ семантикой (не AND, как в №253-А) —
//   альтернативы, любое из условий открывает чтение:
//     * `METALOGOS_SERVE_ALLOW_ENV=1` — разрешить ЛЮБОЕ чтение env в serve;
//     * `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"` — перечисленные имена
//       читаются без гейта (пустой/не задан = deny всех).
//
// Механизм: переиспользован SSOT контекста из №253-А — тот же thread-local
// (`SERVE_ROUTE_CONTEXT`) и тот же `ServeRouteExecGuard`, ставящийся первой
// строкой spawn_blocking-замыкания обоих роут-путей (TW и VM) в
// src/server.rs. Второй флаг-хак / отдельный guard НЕ вводятся (запрет
// наряда №259: «не плодить второй флаг-хак»).

/// SSOT-гейт `env()` (Наряд №259).
///
/// Контекст определяет политику:
/// - [`ExecContext::Process`] → без гейта (контракт: локальный скрипт
///   читает своё окружение; allowlist/флаги вне serve не требуются);
/// - [`ExecContext::ServeRoute`] → отказ с `ENV_NOT_PERMITTED`, если не
///   задан `METALOGOS_SERVE_ALLOW_ENV=1` (разрешить всё) ИЛИ имя не
///   входит в `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"` (точечное
///   разрешение; пустой/не заданный список = deny всех).
///
/// Ошибки несут стабильный диагностический код `ENV_NOT_PERMITTED`
/// (конвенция ADR-0131: UPPER_SNAKE_CASE, код — контракт, текст может
/// меняться) и называют точные имена переменных-флагов.
pub fn env_gate(context: ExecContext, key: &str) -> Result<(), String> {
    match context {
        // №259: вне serve поведение не меняется — гейта нет.
        ExecContext::Process => Ok(()),
        ExecContext::ServeRoute => {
            // Эскейп-хэтч 1: разрешить всё в serve.
            if std::env::var("METALOGOS_SERVE_ALLOW_ENV").unwrap_or_default() == "1" {
                return Ok(());
            }
            // Эскейп-хэтч 2: точечный allowlist имён.
            if env_allowlist_contains(key) {
                return Ok(());
            }
            Err(format!(
                "[ENV_NOT_PERMITTED] env(\"{}\") is denied in serve route handlers. \
                 In route bodies set METALOGOS_SERVE_ALLOW_ENV=1 to allow all env \
                 reads, or add the variable name to \
                 METALOGOS_ENV_ALLOWLIST=\"NAME1,NAME2\" to allow specific \
                 variables (Naryad #259).",
                key
            ))
        }
    }
}

/// Точный поиск имени в `METALOGOS_ENV_ALLOWLIST` (Наряд №259).
///
/// Формат: имена через запятую; пробелы по краям элемента срезаются
/// (`"A, B"` = `["A", "B"]`); пустые элементы игнорируются (`"A,,B"`,
/// `""`, пробел). Переменная не задана или список пуст → deny (`false`).
/// Сравнение точное (case-sensitive) — имена переменных процесса
/// регистрозависимы на целевых платформах.
fn env_allowlist_contains(key: &str) -> bool {
    let raw = match std::env::var("METALOGOS_ENV_ALLOWLIST") {
        Ok(v) => v,
        Err(_) => return false, // не задан = deny всех
    };
    raw.split(',')
        .map(str::trim)
        .any(|name| !name.is_empty() && name == key)
}

/// `exec(cmd)` — execute a shell command and return stdout.
///
/// **Signature unchanged** (Наряд №88 Блок 1: hardened, not re-contracted).
///
/// Enhancements over pre-Н88:
/// - **Timeout** (default 30s, max 300s, configurable via
///   `METALOGOS_EXEC_TIMEOUT_SECS` env var). Process killed on timeout.
/// - **File audit log** — every invocation logged via `append_subprocess_audit`.
///   Path via `METALOGOS_AUDIT_LOG_PATH`, defaults to
///   `metalogos_subprocess_audit.log`.
///
/// The existing `sh -c` invocation is preserved for backward compatibility
/// with all .mlog code that uses `exec()`. New internal callers
/// (html_render, future builtins) use `exec_restricted` instead.
pub(crate) fn builtin_exec(args: &[Value]) -> Result<Value, String> {
    // Security (Наряд №97 → №253): unconditional deny by default.
    // №97: the previous in_server heuristic (METALOGOS_PORT / METALOGOS_DB)
    // was structurally broken — replaced by the process-flag gate.
    // №253 (Вариант А): гейт вынесен в SSOT `exec_gate(context)`; в
    // serve-роут-контексте требуется отдельный METALOGOS_SERVE_ALLOW_EXEC=1
    // (процесс-флаг на тела роутов не распространяется).
    exec_gate(current_exec_context())?;

    let cmd = expect_string_arg("exec", args, 0)?;

    // ── Timeout (Наряд №88 Блок 1.1) ──
    // Pattern: same as http_post — configurable, clamped, default with ceiling.
    let timeout_secs: u64 = match std::env::var("METALOGOS_EXEC_TIMEOUT_SECS") {
        Ok(s) => {
            let parsed = s.parse::<u64>().unwrap_or(EXEC_DEFAULT_TIMEOUT_SECS);
            let clamped = parsed.clamp(1, EXEC_MAX_TIMEOUT_SECS);
            if parsed > EXEC_MAX_TIMEOUT_SECS {
                eprintln!(
                    "[exec] timeout clamped from {} to {}s",
                    parsed, EXEC_MAX_TIMEOUT_SECS
                );
            }
            clamped
        }
        Err(_) => EXEC_DEFAULT_TIMEOUT_SECS,
    };
    let timeout = Duration::from_secs(timeout_secs);

    // Spawn with timeout
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("exec(): failed to run command: {}", e))?;

    // Poll-based timeout (same pattern as exec_restricted)
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Process exited — collect output
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("exec(): failed to read output: {}", e))?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let exit_status_str = format!("{}", output.status);

                // Audit log (Блок 1.2 / Блок 3)
                append_subprocess_audit("exec", &cmd, &exit_status_str);

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    return Err(format!(
                        "exec() command exited with {}: {}",
                        output.status,
                        stderr.trim()
                    ));
                }
                return Ok(Value::String(stdout));
            }
            Ok(None) => {
                // Still running — check deadline
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait(); // reap

                    // Audit: timeout event
                    append_subprocess_audit(
                        "exec",
                        &cmd,
                        &format!("TIMEOUT after {}s", timeout_secs),
                    );

                    return Err(format!(
                        "exec(): timeout after {}s for command: {}",
                        timeout_secs,
                        if cmd.len() > 200 {
                            format!("{}...", crate::util::safe_byte_truncate(&cmd, 200))
                        } else {
                            cmd.clone()
                        }
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("exec(): failed to wait on child: {}", e));
            }
        }
    }
}

/// `exec_argv(binary: String, args: List<String>) -> String`
///
/// Execute a command with explicit argument list — NO shell interpretation.
/// Unlike `exec()`, this function does NOT pass arguments through `sh -c`,
/// so shell metacharacters (`;`, `&&`, `|`, `$()`) are treated as literal
/// argument content, not as shell operators.
///
/// This is the RECOMMENDED way to call external commands when any argument
/// may come from user input. Use `exec()` only for fully literal command
/// strings where no injection is possible.
///
/// Requires `exec_gate` (Наряд №253): `METALOGOS_ALLOW_EXEC=1` в
/// процесс-контексте, `METALOGOS_SERVE_ALLOW_EXEC=1` в телах роутов.
///
/// **Наряд №97 Блок 2 (P1):** added alongside `exec()` (Путь А — not replacing).
pub(crate) fn builtin_exec_argv(args: &[Value]) -> Result<Value, String> {
    // Security (№97 → №253): same gate as exec() — SSOT exec_gate(context),
    // serve-роут-контекст требует METALOGOS_SERVE_ALLOW_EXEC=1 (Вариант А).
    exec_gate(current_exec_context())?;

    if args.is_empty() {
        return Err("exec_argv() requires at least 1 argument (binary path)".to_string());
    }

    let binary = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Err("exec_argv(): first argument must be a string (binary path)".to_string()),
    };

    let argv: Vec<String> = match args.get(1) {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                _ => Err("exec_argv(): all args must be strings".to_string()),
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("exec_argv(): second argument must be a list of strings".to_string()),
        None => vec![],
    };

    let argv_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();

    // Use the same timeout configuration as exec()
    let timeout_secs: u64 = match std::env::var("METALOGOS_EXEC_TIMEOUT_SECS") {
        Ok(s) => {
            let parsed = s.parse::<u64>().unwrap_or(EXEC_DEFAULT_TIMEOUT_SECS);
            parsed.clamp(1, EXEC_MAX_TIMEOUT_SECS)
        }
        Err(_) => EXEC_DEFAULT_TIMEOUT_SECS,
    };
    let timeout = Duration::from_secs(timeout_secs);

    let result = exec_restricted(&binary, &argv_refs, timeout);

    // Audit log
    let detail = format!("{} {:?}", binary, argv_refs);
    match result {
        Ok(output) => {
            let exit_status_str = format!("{}", output.status);
            append_subprocess_audit("exec_argv", &detail, &exit_status_str);

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(format!(
                    "exec_argv() command exited with {}: {}",
                    output.status,
                    stderr.trim()
                ));
            }
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(Value::String(stdout))
        }
        Err(e) => {
            append_subprocess_audit("exec_argv", &detail, &format!("ERROR: {}", e));
            Err(format!("exec_argv({}): {}", binary, e))
        }
    }
}

/// `git_push(message?) -> String` — git add/commit/push via subprocess.
/// Uses GITHUB_TOKEN and GITHUB_REPO env vars for authentication.
/// Usage: git_push("commit message") -> "ok" | "nothing to commit" | error
pub(crate) fn builtin_git_push(args: &[Value]) -> Result<Value, String> {
    let message = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => "Auto commit".to_string(),
    };

    let run = |cmd: &str, cmd_args: &[&str]| -> Result<String, String> {
        let output = std::process::Command::new(cmd)
            .args(cmd_args)
            .output()
            .map_err(|e| format!("git_push(): {} failed: {}", cmd, e))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "git_push(): {} exited with {}: {}",
                cmd,
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    };

    run("git", &["add", "."])?;

    // Check if there's anything to commit
    let status = run("git", &["status", "--porcelain"])?;
    if status.trim().is_empty() {
        return Ok(Value::String("nothing to commit".to_string()));
    }

    run("git", &["commit", "-m", &message])?;

    // Push using token from env
    let token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
    let repo = std::env::var("GITHUB_REPO").unwrap_or_default();
    if token.is_empty() || repo.is_empty() {
        return Err("git_push(): GITHUB_TOKEN or GITHUB_REPO env var not set".to_string());
    }

    let remote = format!("https://{}@github.com/{}.git", token, repo);
    run("git", &["push", &remote, "main"])?;

    Ok(Value::String("ok".to_string()))
}

// ── html_render (Наряд №88 Блок 2) ──

/// `html_render(html, width, height) -> String`
///
/// Render self-contained HTML to a PNG screenshot using a headless
/// Chromium/Chrome binary. Returns the path to the generated PNG file.
///
/// **Configuration:** `METALOGOS_BROWSER_BIN` env var must point to a
/// Chromium/Chrome binary. No default path is assumed — environments
/// differ, and guessing wrong produces confusing errors.
///
/// **Security model:**
/// - Uses `exec_restricted` — arguments passed as OS array, NO shell
///   interpretation. Shell metacharacters in HTML cannot be injected
///   into the command line.
/// - HTML is written to a temporary file before being passed to the
///   browser, avoiding argument-length limits and escaping issues.
/// - **Network isolation is NOT enforced at the OS level** (no
///   namespaces/seccomp). `html_render` is designed for
///   **self-contained HTML** — inline styles, `data:` URIs for images,
///   no external `<img src="http://...">` or `<script src="http://...">`.
///   If the HTML references external resources, the browser MAY fetch
///   them depending on its configuration and network access. This is
///   documented, not hidden: the caller is responsible for ensuring
///   self-contained input.
///
/// **Timeout:** default 30s, max 120s (more generous than exec() because
/// browser rendering of complex pages can be slow). Configurable via
/// `METALOGOS_HTML_RENDER_TIMEOUT_SECS`.
///
/// **Audit:** every invocation logged via `append_subprocess_audit`
/// (same audit trail as `exec`).
pub(crate) fn builtin_html_render(args: &[Value]) -> Result<Value, String> {
    let html = expect_string_arg("html_render", args, 0)?;
    let width = match args.get(1) {
        Some(Value::Float(f)) => *f as u32,
        other => {
            return Err(format!(
                "html_render: width (arg 1) must be a number, got {:?}",
                other
            ))
        }
    };
    let height = match args.get(2) {
        Some(Value::Float(f)) => *f as u32,
        other => {
            return Err(format!(
                "html_render: height (arg 2) must be a number, got {:?}",
                other
            ))
        }
    };

    if width == 0 || height == 0 {
        return Err("html_render: width and height must be > 0".to_string());
    }

    // ── Browser binary (Блок 2: configuration) ──
    let browser_bin = match std::env::var("METALOGOS_BROWSER_BIN") {
        Ok(path) => path,
        Err(_) => {
            return Err("html_render: METALOGOS_BROWSER_BIN not set. \
                 Point it to a Chromium/Chrome binary to enable this feature."
                .to_string());
        }
    };

    // Verify the binary exists before spawning
    if !std::path::Path::new(&browser_bin).exists() {
        return Err(format!(
            "html_render: METALOGOS_BROWSER_BIN '{}' does not exist or is not executable",
            browser_bin
        ));
    }

    // ── Write HTML to temporary file (avoids shell-escaping issues) ──
    let unique_id = uuid::Uuid::new_v4();
    let html_file = format!("_html_render_{}.html", unique_id);
    let out_file = format!("_html_render_{}.png", unique_id);

    std::fs::write(&html_file, html.as_bytes()).map_err(|e| {
        format!(
            "html_render: failed to write temporary HTML file '{}': {}",
            html_file, e
        )
    })?;

    // ── Build argument array (NO shell — exec_restricted) ──
    let window_size = format!("{},{}", width, height);
    let screenshot_arg = format!("--screenshot={}", out_file);

    let browser_args = &[
        "--headless",
        "--disable-gpu",
        "--no-sandbox",
        &screenshot_arg,
        &format!("--window-size={}", window_size),
        "--virtual-time-budget=2000",
        &html_file,
    ];

    // ── Timeout (Блок 2: separate from exec) ──
    let timeout_secs: u64 = match std::env::var("METALOGOS_HTML_RENDER_TIMEOUT_SECS") {
        Ok(s) => {
            let parsed = s.parse::<u64>().unwrap_or(HTML_RENDER_DEFAULT_TIMEOUT_SECS);
            let clamped = parsed.clamp(1, HTML_RENDER_MAX_TIMEOUT_SECS);
            if parsed > HTML_RENDER_MAX_TIMEOUT_SECS {
                eprintln!(
                    "[html_render] timeout clamped from {} to {}s",
                    parsed, HTML_RENDER_MAX_TIMEOUT_SECS
                );
            }
            clamped
        }
        Err(_) => HTML_RENDER_DEFAULT_TIMEOUT_SECS,
    };
    let timeout = Duration::from_secs(timeout_secs);

    // ── Execute via exec_restricted (Блок 2: no shell interpretation) ──
    let result = exec_restricted(&browser_bin, browser_args, timeout);

    // Clean up temp HTML file (best-effort)
    let _ = std::fs::remove_file(&html_file);

    match result {
        Ok(output) => {
            let exit_status_str = format!("{}", output.status);

            // Audit log (Блок 3)
            append_subprocess_audit(
                "html_render",
                &format!("{}x{} -> {}", width, height, out_file),
                &exit_status_str,
            );

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                // Clean up output file if created despite failure
                let _ = std::fs::remove_file(&out_file);
                return Err(format!(
                    "html_render: browser exited with {}: {}",
                    output.status,
                    stderr.trim()
                ));
            }

            // Verify the output file was created
            if !std::path::Path::new(&out_file).exists() {
                return Err(format!(
                    "html_render: browser exited successfully but output file '{}' not found",
                    out_file
                ));
            }

            Ok(Value::String(out_file))
        }
        Err(e) => {
            // Clean up output file if created despite error
            let _ = std::fs::remove_file(&out_file);

            // Audit: error event
            append_subprocess_audit(
                "html_render",
                &format!("{}x{} -> {}", width, height, out_file),
                &format!("ERROR: {}", e),
            );

            Err(format!("html_render: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::{SecretString, Value};

    #[test]
    fn print_rejects_secret() {
        let secret = Value::Secret(SecretString::new("s3cr3t".into()));
        let err = builtin_print(&[secret]).unwrap_err();
        assert!(
            err.contains("Secret") && err.contains("refused"),
            "unexpected message: {}",
            err
        );
    }

    #[test]
    fn print_allows_string() {
        let ok = builtin_print(&[Value::String("hello".into())]).unwrap();
        match ok {
            Value::String(s) => assert_eq!(s, "hello"),
            other => panic!("expected String, got {}", other.type_name()),
        }
    }
}

// ── Наряд №131: sandbox_path — канонизация против обхода через симлинки ──
#[cfg(test)]
mod tests_n131 {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;

    /// Create a temp dir, change CWD to it, run f, restore CWD, cleanup.
    pub(super) fn with_temp_sandbox(name: &str, f: impl FnOnce()) {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("setup");
        let prev = std::env::current_dir().expect("get cwd");
        std::env::set_current_dir(&dir).expect("chdir");
        f();
        std::env::set_current_dir(&prev).expect("restore cwd");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Create a temp dir, return its path (caller manages lifetime).
    pub(super) fn make_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("setup");
        dir
    }

    // ── C1: обычный относительный путь внутри директории ──

    #[test]
    #[serial]
    fn n131_normal_relative_read() {
        with_temp_sandbox("metalogos_n131_c1r", || {
            fs::write("existing.txt", "data").unwrap();
            assert!(sandbox_path("existing.txt").is_ok());
        });
    }

    #[test]
    #[serial]
    fn n131_normal_relative_write() {
        with_temp_sandbox("metalogos_n131_c1w", || {
            // "new.txt" does not exist — ForWrite canonicalizes parent only
            assert!(sandbox_path_ex("new.txt", SandboxMode::ForWrite).is_ok());
        });
    }

    // ── C2: симлинк на файл снаружи — отклонён ──

    #[test]
    #[serial]
    fn n131_symlink_to_outside_file_rejected() {
        let sandbox = make_temp_dir("metalogos_n131_c2a_sandbox");
        let outside = make_temp_dir("metalogos_n131_c2a_outside");

        // Secret file outside sandbox
        let secret = outside.join("secret.txt");
        fs::write(&secret, "TOP SECRET").unwrap();

        // Symlink inside sandbox → outside
        let link = sandbox.join("escape");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&secret, &link).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = sandbox_path("escape");
        std::env::set_current_dir(&prev).unwrap();

        assert!(
            result.is_err(),
            "symlink to outside must be rejected: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("escapes sandbox"),
            "expected 'escapes sandbox', got: {}",
            err
        );

        let _ = fs::remove_dir_all(&sandbox);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    #[serial]
    fn n131_symlink_via_subdir_rejected() {
        let sandbox = make_temp_dir("metalogos_n131_c2b_sandbox");
        let outside = make_temp_dir("metalogos_n131_c2b_outside");

        let secret = outside.join("data.bin");
        fs::write(&secret, "binary").unwrap();

        let sub = sandbox.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let link = sub.join("link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&secret, &link).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = sandbox_path("sub/link");
        std::env::set_current_dir(&prev).unwrap();

        assert!(
            result.is_err(),
            "symlink via subdir must be rejected: {:?}",
            result
        );

        let _ = fs::remove_dir_all(&sandbox);
        let _ = fs::remove_dir_all(&outside);
    }

    // ── C3: write на новый файл — не ломается ──

    #[test]
    #[serial]
    fn n131_write_new_file_passes() {
        with_temp_sandbox("metalogos_n131_c3a", || {
            assert!(sandbox_path_ex("brand_new.txt", SandboxMode::ForWrite).is_ok());
        });
    }

    #[test]
    #[serial]
    fn n131_write_new_in_subdir_passes() {
        with_temp_sandbox("metalogos_n131_c3b", || {
            fs::create_dir_all("sub").unwrap();
            assert!(sandbox_path_ex("sub/new.txt", SandboxMode::ForWrite).is_ok());
        });
    }

    // ── C3b: write через симлинк-директорию наружу — отклонён ──

    #[test]
    #[serial]
    fn n131_write_via_symlink_dir_rejected() {
        let sandbox = make_temp_dir("metalogos_n131_c3c_sandbox");
        let outside = make_temp_dir("metalogos_n131_c3c_outside");

        // Symlink *directory* inside sandbox → outside
        let link = sandbox.join("escape_dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, &link).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = sandbox_path_ex("escape_dir/evil.txt", SandboxMode::ForWrite);
        std::env::set_current_dir(&prev).unwrap();

        assert!(
            result.is_err(),
            "write via symlink dir must be rejected: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("escapes sandbox"),
            "expected 'escapes sandbox', got: {}",
            err
        );

        let _ = fs::remove_dir_all(&sandbox);
        let _ = fs::remove_dir_all(&outside);
    }

    // ── C4: существующие текстовые проверки не сломаны ──

    #[test]
    fn n131_absolute_path_rejected() {
        let result = sandbox_path("/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute paths not allowed"));
    }

    #[test]
    fn n131_dotdot_rejected() {
        let result = sandbox_path("../../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path traversal"));
    }

    // ── Additional: broken symlink ──

    #[test]
    #[serial]
    fn n131_broken_symlink_rejected() {
        let sandbox = make_temp_dir("metalogos_n131_broken");

        let link = sandbox.join("dangling");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/nonexistent/target", &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file("/nonexistent/target", &link).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = sandbox_path("dangling");
        std::env::set_current_dir(&prev).unwrap();

        // Broken symlink: canonicalize fails → rejected
        assert!(
            result.is_err(),
            "broken symlink must be rejected: {:?}",
            result
        );

        let _ = fs::remove_dir_all(&sandbox);
    }
}

// ── Наряд №252: write-path TOCTOU / planted final-component symlink ──
#[cfg(test)]
mod tests_n252 {
    use super::tests_n131::{make_temp_dir, with_temp_sandbox};
    use super::*;
    use serial_test::serial;
    use std::fs;

    // Unix-only: planted symlinks are the repro'd attack class (№252).
    // The full write_file call chain is exercised — not just
    // sandbox_path_ex — because the bug lived between the check and
    // the use (fs::write followed the planted symlink).

    #[test]
    #[cfg(unix)]
    #[serial]
    fn n252_write_through_planted_symlink_denied() {
        let sandbox = make_temp_dir("metalogos_n252_w_link");
        let outside = make_temp_dir("metalogos_n252_w_out");
        fs::write(outside.join("secret"), "TOP SECRET").unwrap();

        // Plant: a.txt (inside sandbox) → outside secret file
        std::os::unix::fs::symlink(outside.join("secret"), sandbox.join("a.txt")).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = builtin_write_file(&[
            Value::String("a.txt".to_string()),
            Value::String("EVIL".to_string()),
        ]);
        std::env::set_current_dir(&prev).unwrap();

        // Loud error, NOT soft "" — and the outside file is untouched.
        assert!(
            result.is_err(),
            "planted-symlink write must be denied: {:?}",
            result
        );
        assert!(
            result.unwrap_err().contains("escapes sandbox"),
            "expected 'escapes sandbox'"
        );
        assert_eq!(
            fs::read_to_string(outside.join("secret")).unwrap(),
            "TOP SECRET",
            "outside file must be untouched"
        );

        let _ = fs::remove_dir_all(&sandbox);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    #[cfg(unix)]
    #[serial]
    fn n252_append_through_planted_symlink_denied() {
        let sandbox = make_temp_dir("metalogos_n252_a_link");
        let outside = make_temp_dir("metalogos_n252_a_out");
        fs::write(outside.join("secret"), "TOP SECRET").unwrap();

        std::os::unix::fs::symlink(outside.join("secret"), sandbox.join("log.txt")).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = builtin_append_file(&[
            Value::String("log.txt".to_string()),
            Value::String("EVIL".to_string()),
        ]);
        std::env::set_current_dir(&prev).unwrap();

        assert!(
            result.is_err(),
            "planted-symlink append must be denied: {:?}",
            result
        );
        assert_eq!(
            fs::read_to_string(outside.join("secret")).unwrap(),
            "TOP SECRET",
            "outside file must be untouched"
        );

        let _ = fs::remove_dir_all(&sandbox);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    #[cfg(unix)]
    #[serial]
    fn n252_write_broken_symlink_loud() {
        let sandbox = make_temp_dir("metalogos_n252_dangling");
        std::os::unix::fs::symlink("/nonexistent/n252_target", sandbox.join("dangling.txt"))
            .unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = builtin_write_file(&[
            Value::String("dangling.txt".to_string()),
            Value::String("x".to_string()),
        ]);
        std::env::set_current_dir(&prev).unwrap();

        assert!(
            result.is_err(),
            "broken symlink write must be loud: {:?}",
            result
        );
        assert!(
            result.unwrap_err().contains("cannot resolve"),
            "expected 'cannot resolve'"
        );

        let _ = fs::remove_dir_all(&sandbox);
    }

    #[test]
    #[cfg(unix)]
    #[serial]
    fn n252_write_dir_instead_of_file_loud() {
        with_temp_sandbox("metalogos_n252_dir", || {
            fs::create_dir("adir").unwrap();
            let result = builtin_write_file(&[
                Value::String("adir".to_string()),
                Value::String("x".to_string()),
            ]);
            assert!(
                result.is_err(),
                "directory write must be loud: {:?}",
                result
            );
        });
    }

    #[test]
    #[cfg(unix)]
    #[serial]
    fn n252_read_via_in_sandbox_symlink_ok() {
        with_temp_sandbox("metalogos_n252_r_link", || {
            fs::write("real.txt", "inner").unwrap();
            std::os::unix::fs::symlink("real.txt", "alias.txt").unwrap();
            // In-sandbox symlink: reads still resolve (canonical return,
            // prefix holds) — the sandbox is not a symlink ban.
            match builtin_read_file(&[Value::String("alias.txt".to_string())]) {
                Ok(Value::String(s)) => assert_eq!(s, "inner"),
                other => panic!("expected inner content, got {:?}", other),
            }
        });
    }

    #[test]
    #[serial]
    fn n252_write_new_and_overwrite_regular_file_ok() {
        with_temp_sandbox("metalogos_n252_plain", || {
            // New file: create_new path
            match builtin_write_file(&[
                Value::String("new.txt".to_string()),
                Value::String("one".to_string()),
            ]) {
                Ok(Value::String(s)) => assert_eq!(s, "ok"),
                other => panic!("new-file write failed: {:?}", other),
            }
            // Existing regular file: AlreadyExists → canonical + O_NOFOLLOW path
            match builtin_write_file(&[
                Value::String("new.txt".to_string()),
                Value::String("two".to_string()),
            ]) {
                Ok(Value::String(s)) => assert_eq!(s, "ok"),
                other => panic!("overwrite write failed: {:?}", other),
            }
            assert_eq!(fs::read_to_string("new.txt").unwrap(), "two");
            // Append into the same contract
            match builtin_append_file(&[
                Value::String("new.txt".to_string()),
                Value::String("+".to_string()),
            ]) {
                Ok(Value::String(s)) => assert_eq!(s, "ok"),
                other => panic!("append failed: {:?}", other),
            }
            assert_eq!(fs::read_to_string("new.txt").unwrap(), "two+");
        });
    }

    #[test]
    #[serial]
    fn n252_forwrite_returns_canonical_parent_join_name() {
        with_temp_sandbox("metalogos_n252_canon", || {
            let got = sandbox_path_ex("new.txt", SandboxMode::ForWrite).unwrap();
            let base = std::env::current_dir().unwrap().canonicalize().unwrap();
            assert_eq!(got, base.join("new.txt"), "must be canonical parent + name");
        });
    }
}

#[cfg(test)]
mod tests_n254 {
    use super::*;
    use serial_test::serial;
    use std::fs;

    /// Хелпер: Value::String -> String (Value не реализует PartialEq).
    fn s(v: Value) -> String {
        match v {
            Value::String(sv) => sv,
            other => panic!(
                "expected Value::String, got different variant ({:?})",
                std::mem::discriminant(&other)
            ),
        }
    }

    // ── Наряд №254: read_file/delete_file разделяют «нет файла» (soft)
    //    и «нарушение песочницы» (loud [SANDBOX_VIOLATION]); write_file/
    //    append_file несут код на громких отказах (громкость — с №252). ──

    #[test]
    #[serial]
    fn n254_read_missing_file_is_soft() {
        super::tests_n131::with_temp_sandbox("metalogos_n254_soft", || {
            let out = builtin_read_file(&[Value::String("нет_такого.txt".to_string())]).unwrap();
            assert_eq!(s(out), "", "нет файла = мягкая пустая строка");
        });
    }

    #[test]
    #[serial]
    fn n254_read_traversal_is_loud() {
        super::tests_n131::with_temp_sandbox("metalogos_n254_trav", || {
            let err = builtin_read_file(&[Value::String("../x".to_string())]).unwrap_err();
            assert!(
                err.contains("[SANDBOX_VIOLATION]"),
                "код обязателен, got: {}",
                err
            );
            assert!(err.contains("path traversal"), "got: {}", err);
        });
    }

    #[test]
    #[serial]
    fn n254_read_absolute_is_loud() {
        super::tests_n131::with_temp_sandbox("metalogos_n254_abs", || {
            let err = builtin_read_file(&[Value::String("/etc/passwd".to_string())]).unwrap_err();
            assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
            assert!(err.contains("absolute paths not allowed"), "got: {}", err);
        });
    }

    #[test]
    #[serial]
    fn n254_read_symlink_escape_is_loud() {
        let sandbox = super::tests_n131::make_temp_dir("metalogos_n254_esc_sandbox");
        let outside = super::tests_n131::make_temp_dir("metalogos_n254_esc_outside");
        let secret = outside.join("secret.txt");
        fs::write(&secret, "TOP SECRET").unwrap();
        let link = sandbox.join("escape");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&secret, &link).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = builtin_read_file(&[Value::String("escape".to_string())]);
        std::env::set_current_dir(&prev).unwrap();

        let err = result.unwrap_err();
        assert!(
            err.contains("[SANDBOX_VIOLATION]") && err.contains("escapes sandbox"),
            "got: {}",
            err
        );
        let _ = fs::remove_dir_all(&sandbox);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    #[serial]
    fn n254_read_dangling_symlink_is_loud() {
        // Битый symlink: звено существует (symlink_metadata Ok), canonicalize
        // падает → нарушение №131 теперь ГРОМКОЕ, не молчалка.
        let sandbox = super::tests_n131::make_temp_dir("metalogos_n254_dangling");
        let link = sandbox.join("dangling");
        #[cfg(unix)]
        std::os::unix::fs::symlink(sandbox.join("nowhere.txt"), &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(sandbox.join("nowhere.txt"), &link).unwrap();

        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&sandbox).unwrap();
        let result = builtin_read_file(&[Value::String("dangling".to_string())]);
        std::env::set_current_dir(&prev).unwrap();

        let err = result.unwrap_err();
        assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
        let _ = fs::remove_dir_all(&sandbox);
    }

    #[test]
    #[serial]
    fn n254_delete_missing_soft_traversal_loud() {
        super::tests_n131::with_temp_sandbox("metalogos_n254_del", || {
            let out = builtin_delete_file(&[Value::String("нет_такого.txt".to_string())]).unwrap();
            assert_eq!(s(out), "", "нет файла = мягко");
            let err = builtin_delete_file(&[Value::String("../x".to_string())]).unwrap_err();
            assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
        });
    }

    #[test]
    #[serial]
    fn n254_write_append_traversal_loud() {
        super::tests_n131::with_temp_sandbox("metalogos_n254_wr", || {
            let err = builtin_write_file(&[
                Value::String("../x".to_string()),
                Value::String("v".to_string()),
            ])
            .unwrap_err();
            assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
            let err = builtin_append_file(&[
                Value::String("../x".to_string()),
                Value::String("v".to_string()),
            ])
            .unwrap_err();
            assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
        });
    }

    #[test]
    #[serial]
    fn n254_positive_path_unbroken() {
        super::tests_n131::with_temp_sandbox("metalogos_n254_ok", || {
            let out = builtin_write_file(&[
                Value::String("f.txt".to_string()),
                Value::String("hi".to_string()),
            ])
            .unwrap();
            assert_eq!(s(out), "ok");
            let out = builtin_read_file(&[Value::String("f.txt".to_string())]).unwrap();
            assert_eq!(s(out), "hi");
            let out = builtin_append_file(&[
                Value::String("f.txt".to_string()),
                Value::String("!".to_string()),
            ])
            .unwrap();
            assert_eq!(s(out), "ok");
            let out = builtin_read_file(&[Value::String("f.txt".to_string())]).unwrap();
            assert_eq!(s(out), "hi!");
            let out = builtin_delete_file(&[Value::String("f.txt".to_string())]).unwrap();
            assert_eq!(s(out), "ok");
            let out = builtin_read_file(&[Value::String("f.txt".to_string())]).unwrap();
            assert_eq!(s(out), "");
        });
    }
}
