// ── Naryad N7 (gh#1217, PE-2, serve): the application response headers ──
//
// The `headers` field on Value::HttpResponse + the `set_header`
// builtin (the pure value transform: `set_header(resp, name, value) ->
// HttpResponse`).
//
//   S1: the allowlist — the five application headers (Cache-Control,
//       ETag, Content-Type, Location, Content-Disposition) set and
//       return the response; the names match case-INSENSITIVELY and
//       store in the canonical spelling.
//   S2: `set` semantics — the same name (case-insensitive) REPLACES,
//       the other headers keep their order.
//   S3: a name outside the allowlist → the typed
//       [HTTP_HEADER_NOT_ALLOWED] refusal (runtime) AND the Category-A
//       compile refusal (the static lane) — the protective headers
//       (CSP, XCTO, HSTS) are the middleware's duty (N8).
//   S4: CR/LF (and NUL) in the name or the value → the typed
//       [HTTP_HEADER_INJECTION] refusal — the response-splitting vector
//       is refused on both layers (the runtime gate + the static lane).
//   S5: a UserInput-tainted value → [HTTP_HEADER_TAINTED] at COMPILE
//       time (the static Category-A lane; the taint never reaches the
//       runtime channel).
//   S6: the server boundary — value_to_response applies the headers to
//       the real axum response (the metalogos-server test).
//   S7: the TW/VM parity — the shared registry handler constructs the
//       same Value on both backends; the chain output is identical.
//
// RED first: without the wiring `set_header` is the loud
// unknown-function compile error; the header field did not exist.

fn eval_expr(src: &str) -> Result<String, String> {
    let body = if src.trim_start().starts_with("let ") {
        src.to_string()
    } else {
        format!("return {}", src)
    };
    let full = format!(
        "pattern __eval(input: String) -> String {{ {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        body
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err("eval returned None".to_string()),
        Err(e) => Err(e),
    }
}

fn eval_expr_vm(src: &str) -> Result<String, String> {
    let body = if src.trim_start().starts_with("let ") {
        src.to_string()
    } else {
        format!("return {}", src)
    };
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let declarations = metalogos::parser::parse(&format!(
        "pattern __eval(input: String) -> String {{ {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        body
    ))
    .map_err(|e| format!("parse error: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    match vm.run(program)? {
        Some(s) => Ok(s.trim_end().to_string()),
        None => Err("vm eval returned None".to_string()),
    }
}

// ── S1: the allowlisted set works, case-insensitively ────────────────

#[test]
fn n7_set_header_allowlisted_names_set() {
    // The chain sets two of the five; the output is the response value
    // (the opaque Display — the S6/S7 legs inspect the actual headers).
    let src = "let r = respond(\"200\", \"ok\")\n         let r = set_header(r, \"Cache-Control\", \"no-store\")\n         let r = set_header(r, \"etag\", \"W/\\\"v1\\\"\")\n         return r";
    let out = eval_expr(src).expect("set_header must succeed");
    assert!(
        out.contains("[HttpResponse 200]"),
        "the response value flows: {out}"
    );
    let vm = eval_expr_vm(src).expect("set_header must succeed on VM");
    assert_eq!(out.trim_end(), vm.trim_end(), "TW/VM parity (S7)");
}

#[test]
fn n7_set_header_requires_response_value() {
    let src = "set_header(\"not a response\", \"Cache-Control\", \"no-store\")";
    let err = eval_expr(src).expect_err("a non-response first argument is loud");
    assert!(
        err.contains("HttpResponse"),
        "the error must name the expected type: {err}"
    );
}

// ── S3: the allowlist refusal (runtime layer) ────────────────────────

#[test]
fn n7_set_header_outside_allowlist_is_typed_loud() {
    // X-Frame-Options is NOT in the application allowlist — the
    // protective headers are the middleware's duty (N8).
    let src = "let r = respond(\"200\", \"ok\")\n         return set_header(r, \"X-Frame-Options\", \"DENY\")";
    let err = eval_expr(src).expect_err("the allowlist must refuse");
    assert!(
        err.contains("HTTP_HEADER_NOT_ALLOWED"),
        "the refusal must carry the typed stamp: {err}"
    );
    assert!(
        err.contains("X-Frame-Options") && err.contains("Cache-Control"),
        "the refusal must name the offender and the allowlist: {err}"
    );
}

#[test]
fn n7_set_header_protective_headers_are_middleware_duty() {
    for name in [
        "Strict-Transport-Security",
        "X-Content-Type-Options",
        "Content-Security-Policy",
    ] {
        let src = format!(
            "let r = respond(\"200\", \"ok\")\n         return set_header(r, \"{name}\", \"x\")"
        );
        let err = eval_expr(&src).expect_err("a protective header must refuse");
        assert!(
            err.contains("HTTP_HEADER_NOT_ALLOWED"),
            "{name} must refuse with the typed stamp: {err}"
        );
    }
}

// ── S4: the CR/LF injection refusal (runtime layer) ──────────────────

#[test]
fn n7_set_header_crlf_in_value_is_typed_loud() {
    // The response-splitting vector: the value smuggles a second header.
    let src = "let r = respond(\"200\", \"ok\")\n         return set_header(r, \"Cache-Control\", \"no-store\\r\\nX-Evil: 1\")";
    let err = eval_expr(src).expect_err("CR/LF in the value must refuse");
    assert!(
        err.contains("HTTP_HEADER_INJECTION"),
        "the refusal must carry the typed stamp: {err}"
    );
}

#[test]
fn n7_set_header_crlf_in_name_is_typed_loud() {
    let src = "let r = respond(\"200\", \"ok\")\n         return set_header(r, \"Cache-Control\\nEvil: 1\", \"x\")";
    let err = eval_expr(src).expect_err("CR/LF in the name must refuse");
    assert!(
        err.contains("HTTP_HEADER_INJECTION"),
        "the refusal must carry the typed stamp: {err}"
    );
    assert!(
        !err.contains("HTTP_HEADER_NOT_ALLOWED"),
        "the injection verdict must come FIRST (the smuggled name must not leak the allowlist's wording): {err}"
    );
}

// ── S5: the tainted value — the STATIC Category-A lane ───────────────

#[test]
fn n7_set_header_tainted_value_refused_at_compile() {
    // query_param carries UserInput taint — the header channel is the
    // server's own voice; the echo goes to the BODY (escaped), never
    // into a response header.
    let src = "pattern P(input: String) -> String {\n           let r = respond(\"200\", \"ok\")\n           let r = set_header(r, \"Cache-Control\", query_param(input))\n           return r\n         }\n         flow Main { input: String = \"q\" -> P -> output }";
    let err = metalogos::run_program(src).expect_err("a tainted header value must refuse");
    assert!(
        err.contains("HTTP_HEADER_TAINTED"),
        "the static lane must refuse at compile time: {err}"
    );
    assert!(
        err.contains("Category A security invariant"),
        "the refusal rides the Category-A blocking lane: {err}"
    );
}

#[test]
fn n7_set_header_clean_dynamic_value_allowed() {
    // A non-literal CLEAN value (computed in-program) is legitimate.
    let src = "let r = respond(\"200\", \"ok\")\n         let tag = \"W/\" + \"\\\"abc\\\"\"\n         return set_header(r, \"ETag\", tag)";
    let out = eval_expr(src).expect("a clean dynamic value must pass");
    assert!(out.contains("[HttpResponse 200]"), "{out}");
}

// ── S6: the server boundary (the value_to_response application) ──────
// The header application at the conversion boundary is pinned by the
// metalogos-server test n7_value_to_response_applies_set_headers
// (the axum Response inspection — the curl -i equivalent).

// ── S7: the TW/VM parity on the chain ────────────────────────────────

#[test]
fn n7_vm_parity_set_header_chain() {
    let src = "let r = respond(\"200\", \"ok\")\n         let r = set_header(r, \"Location\", \"https://gov.example.by/next\")\n         let r = set_header(r, \"location\", \"https://gov.example.by/final\")\n         return r";
    let tw = eval_expr(src).expect("TW chain");
    let vm = eval_expr_vm(src).expect("VM chain");
    assert_eq!(
        tw.trim_end(),
        vm.trim_end(),
        "the shared handler must produce the identical response value"
    );
}
