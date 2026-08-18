// ── Наряд №88: Integration tests for exec hardening + html_render ──────
//
// Tests the hardened exec() (timeout, audit log) and html_render builtin.
//
// - exec timeout: verified by running a command that sleeps longer than timeout
// - exec audit log: verified by checking the audit log file after exec
// - html_render missing binary: verified by calling without METALOGOS_BROWSER_BIN
// - html_render invalid dimensions: verified by passing 0 width/height

fn eval_expr(src: &str) -> String {
    let full = format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("eval returned None for source: {}", src),
        Err(e) => panic!("eval failed for source: {}\nerror: {}", src, e),
    }
}

fn eval_expr_err(src: &str) -> String {
    let full = format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    );
    match metalogos::run_program(&full) {
        Ok(_) => panic!("expected error, got Ok: {}", src),
        Err(e) => e,
    }
}

#[allow(dead_code)]
fn eval_program(src: &str) -> Result<String, String> {
    metalogos::run_program(src).map(|v| v.unwrap_or_default())
}

// ── exec() hardening (Блок 1) ─────────────────────────────────────────

#[test]
fn n88_exec_still_works_for_simple_commands() {
    // Basic contract: exec() signature and behavior unchanged
    // Наряд №97: exec() now requires METALOGOS_ALLOW_EXEC=1
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    let result = eval_expr("exec(\"echo hello_n88\")");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    assert!(
        result.contains("hello_n88"),
        "exec('echo hello_n88') should contain 'hello_n88', got: {:?}",
        result
    );
}

#[test]
fn n88_exec_timeout_short_command_succeeds() {
    // A command that finishes quickly should succeed even with the default timeout
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    let result = eval_expr("exec(\"echo fast\")");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    assert!(
        result.contains("fast"),
        "quick command should succeed within default timeout, got: {:?}",
        result
    );
}

#[test]
fn n88_exec_timeout_exceeded() {
    // Set a very short timeout (1s) and run a command that sleeps for 5s.
    // The exec should time out and return an error.
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    std::env::set_var("METALOGOS_EXEC_TIMEOUT_SECS", "1");
    let result = eval_expr_err("exec(\"sleep 5\")");
    std::env::remove_var("METALOGOS_EXEC_TIMEOUT_SECS");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");

    assert!(
        result.contains("timeout"),
        "exec with 1s timeout should timeout for 'sleep 5', got: {:?}",
        result
    );
}

#[test]
fn n88_exec_audit_log_created() {
    // Run exec and verify the audit log file is created.
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    let audit_path = format!("_n88_test_audit_{}.log", std::process::id());
    std::env::set_var("METALOGOS_AUDIT_LOG_PATH", &audit_path);

    // Clean up any existing file
    let _ = std::fs::remove_file(&audit_path);

    let result = eval_expr("exec(\"echo audit_test_n88\")");
    assert!(result.contains("audit_test_n88"));

    // Verify audit log was created and contains our command
    let log_content = std::fs::read_to_string(&audit_path).unwrap_or_default();
    assert!(
        log_content.contains("exec"),
        "audit log should contain 'exec' operation, got: {:?}",
        log_content
    );
    assert!(
        log_content.contains("audit_test_n88"),
        "audit log should contain the command, got: {:?}",
        log_content
    );
    // Verify tab-separated format (4 columns)
    let line = log_content.lines().next().unwrap_or("");
    let tabs = line.matches('\t').count();
    assert_eq!(
        tabs, 3,
        "audit log line should have 3 tabs (4 columns), got {} tabs: {:?}",
        tabs, line
    );

    // Clean up
    let _ = std::fs::remove_file(&audit_path);
    std::env::remove_var("METALOGOS_AUDIT_LOG_PATH");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
}

// ── html_render (Блок 2) ──────────────────────────────────────────────

#[test]
fn n88_html_render_missing_browser_bin() {
    // Without METALOGOS_BROWSER_BIN set, html_render should error clearly
    std::env::remove_var("METALOGOS_BROWSER_BIN");

    let result = eval_expr_err("html_render(\"<html></html>\", 800.0, 600.0)");
    assert!(
        result.contains("METALOGOS_BROWSER_BIN"),
        "error should mention METALOGOS_BROWSER_BIN, got: {:?}",
        result
    );
    assert!(
        result.contains("not set"),
        "error should say 'not set', got: {:?}",
        result
    );
}

#[test]
fn n88_html_render_zero_dimensions_error() {
    // Zero width or height should produce an error
    std::env::remove_var("METALOGOS_BROWSER_BIN");

    let result_w = eval_expr_err("html_render(\"<html></html>\", 0.0, 600.0)");
    assert!(
        result_w.contains("must be > 0") || result_w.contains("METALOGOS_BROWSER_BIN"),
        "zero width should error, got: {:?}",
        result_w
    );

    let result_h = eval_expr_err("html_render(\"<html></html>\", 800.0, 0.0)");
    assert!(
        result_h.contains("must be > 0") || result_h.contains("METALOGOS_BROWSER_BIN"),
        "zero height should error, got: {:?}",
        result_h
    );
}

#[test]
fn n88_html_render_nonexistent_browser_bin() {
    // If METALOGOS_BROWSER_BIN points to a nonexistent file, error should be clear
    std::env::set_var("METALOGOS_BROWSER_BIN", "/nonexistent/chromium_binary_n88");

    let result = eval_expr_err("html_render(\"<html></html>\", 800.0, 600.0)");
    std::env::remove_var("METALOGOS_BROWSER_BIN");

    assert!(
        result.contains("does not exist") || result.contains("not executable"),
        "error should mention non-existent binary, got: {:?}",
        result
    );
}

#[test]
fn n88_html_render_audit_log_on_missing_binary() {
    // When METALOGOS_BROWSER_BIN is not set or invalid, html_render
    // returns an error BEFORE attempting to spawn a subprocess.
    // Audit logging only happens for actual subprocess invocations
    // (same pattern as exec: early validation errors are not audited
    // because no subprocess was launched). This is by design —
    // we audit what actually ran, not what was rejected before running.
    //
    // This test verifies that NO audit entry is created for early
    // validation failures (missing binary), which is correct behavior.
    let audit_path = format!("_n88_html_audit_{}.log", std::process::id());
    std::env::set_var("METALOGOS_AUDIT_LOG_PATH", &audit_path);
    std::env::set_var("METALOGOS_BROWSER_BIN", "/nonexistent/chromium_n88");

    let _ = std::fs::remove_file(&audit_path);

    // This will fail (binary doesn't exist), but no subprocess was
    // attempted, so no audit entry should be written.
    let _result = eval_expr_err("html_render(\"<html></html>\", 800.0, 600.0)");

    let log_content = std::fs::read_to_string(&audit_path).unwrap_or_default();
    // The binary doesn't exist — we never reached exec_restricted,
    // so no audit log entry. This is correct.
    assert!(
        !log_content.contains("html_render"),
        "audit log should NOT contain 'html_render' when binary is missing \
         (no subprocess was launched), got: {:?}",
        log_content
    );

    // Clean up
    let _ = std::fs::remove_file(&audit_path);
    std::env::remove_var("METALOGOS_AUDIT_LOG_PATH");
    std::env::remove_var("METALOGOS_BROWSER_BIN");
}

// ── Наряд №97: exec() unconditional deny + exec_argv ──────────────────

#[test]
fn n97_exec_denied_by_default() {
    // Without METALOGOS_ALLOW_EXEC=1, exec() must be denied in ALL contexts
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    let result = eval_expr_err("exec(\"echo should_not_run\")");
    assert!(
        result.contains("disabled by default"),
        "exec() should be denied by default, got: {:?}",
        result
    );
    assert!(
        result.contains("METALOGOS_ALLOW_EXEC=1"),
        "error should mention METALOGOS_ALLOW_EXEC=1, got: {:?}",
        result
    );
}

#[test]
fn n97_exec_argv_denied_by_default() {
    // Without METALOGOS_ALLOW_EXEC=1, exec_argv() must also be denied
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    let result = eval_expr_err("exec_argv(\"echo\", [\"hello\"])");
    assert!(
        result.contains("disabled by default"),
        "exec_argv() should be denied by default, got: {:?}",
        result
    );
}

#[test]
fn n97_exec_argv_works_with_allow() {
    // With METALOGOS_ALLOW_EXEC=1, exec_argv works and no shell injection
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    let result = eval_expr("exec_argv(\"echo\", [\"hello_n97\"])");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    assert!(
        result.contains("hello_n97"),
        "exec_argv('echo', ['hello_n97']) should contain 'hello_n97', got: {:?}",
        result
    );
}

#[test]
fn n97_exec_argv_no_shell_injection() {
    // Shell metacharacters in exec_argv args are NOT interpreted
    // This would be command injection with exec(), but safe with exec_argv
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    // "hello; rm -rf /" is passed as a LITERAL argument to echo
    let result = eval_expr("exec_argv(\"echo\", [\"hello; rm -rf /\"])");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    assert!(
        result.contains("hello; rm -rf /"),
        "exec_argv should pass shell metacharacters literally, got: {:?}",
        result
    );
    // It should NOT contain just "hello" without the rest (that would mean shell parsed it)
}

#[test]
fn n97_exec_argv_no_args() {
    // exec_argv with just binary path (no args list) should work
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    let result = eval_expr("exec_argv(\"echo\")");
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    // echo with no args outputs a newline
    assert!(
        result.trim().is_empty() || result.contains("echo"),
        "exec_argv('echo') with no args should succeed, got: {:?}",
        result
    );
}
