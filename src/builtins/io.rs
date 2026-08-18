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
fn append_subprocess_audit(operation: &str, detail: &str, exit_status: &str) {
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
    let s = expect_string_arg("print", args, 0)?;
    eprintln!("[print] {}", s);
    Ok(Value::String(s))
}

pub(crate) fn builtin_env(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("env", args, 0)?;
    match std::env::var(&key) {
        Ok(val) => Ok(Value::String(val)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure: empty string if not found
    }
}

pub(crate) fn sandbox_path(path: &str) -> Result<std::path::PathBuf, String> {
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
    Ok(std::path::PathBuf::from(path))
}

/// `read_file(path)` — read file contents as String.
/// Soft-failure: returns empty string on error (file not found, permission denied, etc.).
pub(crate) fn builtin_read_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("read_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    match std::fs::read_to_string(&safe_path) {
        Ok(content) => Ok(Value::String(content)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
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
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    match std::fs::write(&safe_path, &content) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
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
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&safe_path)
    {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(_) => Ok(Value::String("ok".to_string())),
            Err(_) => Ok(Value::String(String::new())), // soft-failure
        },
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `delete_file(path)` — delete a file.
/// Soft-failure: returns empty string on error.
pub(crate) fn builtin_delete_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("delete_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure
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
    // Security (Наряд №97): unconditional deny by default.
    // The previous in_server heuristic (METALOGOS_PORT / METALOGOS_DB)
    // was structurally broken — neither variable is ever set by the
    // server itself, so the check never triggered. Now exec() requires
    // METALOGOS_ALLOW_EXEC=1 in ALL contexts — mlog run, check, serve.
    if std::env::var("METALOGOS_ALLOW_EXEC").unwrap_or_default() != "1" {
        return Err("exec() is disabled by default. Set METALOGOS_ALLOW_EXEC=1 \
             to enable — this applies to mlog run, check, and serve alike."
            .to_string());
    }

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
                            format!("{}...", &cmd[..200])
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
/// Requires `METALOGOS_ALLOW_EXEC=1` — same gate as `exec()`.
///
/// **Наряд №97 Блок 2 (P1):** added alongside `exec()` (Путь А — not replacing).
pub(crate) fn builtin_exec_argv(args: &[Value]) -> Result<Value, String> {
    // Security: same gate as exec() — unconditional deny without METALOGOS_ALLOW_EXEC=1
    if std::env::var("METALOGOS_ALLOW_EXEC").unwrap_or_default() != "1" {
        return Err(
            "exec_argv() is disabled by default. Set METALOGOS_ALLOW_EXEC=1 \
             to enable — this applies to mlog run, check, and serve alike."
                .to_string(),
        );
    }

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
