// ── I/O builtins: print, env, file operations, exec, git_push ──

use crate::interpreter::Value;

use super::core::expect_string_arg;

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
    use std::io::Write;
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

pub(crate) fn builtin_exec(args: &[Value]) -> Result<Value, String> {
    // Security: disable in server context unless explicitly allowed
    if std::env::var("METALOGOS_ALLOW_EXEC").unwrap_or_default() != "1" {
        // Check if we're likely in server mode (has METALOGOS_PORT or METALOGOS_DB env)
        let in_server =
            std::env::var("METALOGOS_PORT").is_ok() || std::env::var("METALOGOS_DB").is_ok();
        if in_server {
            return Err(
                "exec() is disabled in server mode. Set METALOGOS_ALLOW_EXEC=1 to enable."
                    .to_string(),
            );
        }
    }

    let cmd = expect_string_arg("exec", args, 0)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| format!("exec(): failed to run command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!(
            "exec() command exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    Ok(Value::String(stdout))
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
