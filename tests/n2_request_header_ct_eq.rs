// N2 (gh#1211, PE-1): `request_header()` + `ct_eq()`.
//
// `request_header(name) -> String` — the per-request header lookup for
// route bodies:
//   * the header NAME lookup is case-insensitive (HTTP semantics —
//     "X-Telegram-Bot-Api-Secret-Token" and the all-lowercase spelling
//     must resolve to the same value);
//   * an ABSENT header yields the empty string (the query_param №40
//     parity contract — no loud miss);
//   * OUTSIDE a route body (no server context at all) the lookup
//     degrades to the empty string too — the same sibling convention
//     query_param / server_path_param / json_body follow (a webhook
//     secret check therefore fails CLOSED off the server, never errors);
//   * the result carries the UserInput taint (the semantic/audit/
//     html-label source lists — covered by the classification +
//     leak-suite lanes, not duplicated here).
//
// `ct_eq(a, b) -> Bool` — constant-time string equality for secret
// comparisons: the accumulator never short-circuits on a differing
// byte, so the equal-prefix timing leak of `==` is gone. Both
// backends must answer identically (the TW↔VM parity gate).
//
// RED-first discipline: every test below ran against main (no
// builtins registered) and FAILED with the unknown-function error
// before the implementation landed.

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program(source)
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let declarations = metalogos::parser::parse(source).map_err(|e| format!("parse error: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn trim(s: Option<String>) -> String {
    s.as_deref()
        .map(|v| v.trim_end().to_string())
        .unwrap_or_default()
}

fn tw_out(src: &str) -> String {
    trim(run_tw(src).unwrap_or_else(|e| panic!("TW run failed: {e}")))
}

fn vm_out(src: &str) -> String {
    trim(run_vm(src).unwrap_or_else(|e| panic!("VM run failed: {e}")))
}

/// A pattern that returns the result of `ct_eq(a, b)` rendered as
/// "true"/"false" (the Bool output formatting stays inside the if —
/// the parity contract pins the STRING, not the Bool formatting).
fn ct_src(a: &str, b: &str) -> String {
    format!(
        "pattern C(_input: String) -> String {{\n  if ct_eq(\"{a}\", \"{b}\") {{ return \"true\" }}\n  return \"false\"\n}}\nflow Main {{ input: String = \"x\" -> C -> output }}"
    )
}

// ── ct_eq: the correctness surface (both backends) ──────────────────

#[test]
fn n2_ct_eq_equal_strings_true_both_backends() {
    let src = ct_src("whsec-abc-123", "whsec-abc-123");
    assert_eq!(tw_out(&src), "true", "TW: equal strings are equal");
    assert_eq!(vm_out(&src), "true", "VM: equal strings are equal");
}

#[test]
fn n2_ct_eq_different_strings_false_both_backends() {
    let src = ct_src("whsec-abc-123", "whsec-abc-124");
    assert_eq!(tw_out(&src), "false", "TW: a one-byte difference is caught");
    assert_eq!(vm_out(&src), "false", "VM: a one-byte difference is caught");
}

#[test]
fn n2_ct_eq_prefix_not_equal_both_backends() {
    // The `==`-shaped timing hazard: "whsec" is a proper prefix of
    // "whsec-abc" — ct_eq must answer false.
    let src = ct_src("whsec", "whsec-abc");
    assert_eq!(tw_out(&src), "false", "TW: the prefix pair is not equal");
    assert_eq!(vm_out(&src), "false", "VM: the prefix pair is not equal");
}

#[test]
fn n2_ct_eq_empty_strings_equal_both_backends() {
    let src = ct_src("", "");
    assert_eq!(tw_out(&src), "true", "TW: two empty strings are equal");
    assert_eq!(vm_out(&src), "true", "VM: two empty strings are equal");
}

#[test]
fn n2_ct_eq_empty_vs_nonempty_false_both_backends() {
    let src = ct_src("", "x");
    assert_eq!(tw_out(&src), "false", "TW: empty vs non-empty is false");
    assert_eq!(vm_out(&src), "false", "VM: empty vs non-empty is false");
}

#[test]
fn n2_ct_eq_unicode_content_both_backends() {
    // Secrets are compared byte-wise: the multi-byte UTF-8 content must
    // round-trip through the accumulator untouched.
    let src = ct_src("секрет-токен-123", "секрет-токен-123");
    assert_eq!(
        tw_out(&src),
        "true",
        "TW: the cyrillic secret matches itself"
    );
    assert_eq!(
        vm_out(&src),
        "true",
        "VM: the cyrillic secret matches itself"
    );
    let miss = ct_src("секрет-токен-123", "секрет-токен-124");
    assert_eq!(
        tw_out(&miss),
        "false",
        "TW: the cyrillic mismatch is caught"
    );
    assert_eq!(
        vm_out(&miss),
        "false",
        "VM: the cyrillic mismatch is caught"
    );
}

#[test]
fn n2_ct_eq_arity_and_type_loud_both_backends() {
    // One argument → loud; a non-String argument → loud. The secret
    // comparator must not guess (the №514 loud-conversion convention).
    let one_arg = "pattern C(_input: String) -> String {\n  if ct_eq(\"a\") { return \"true\" }\n  return \"false\"\n}\nflow Main { input: String = \"x\" -> C -> output }";
    for (lane, out) in [("TW", run_tw(one_arg)), ("VM", run_vm(one_arg))] {
        let err = out
            .err()
            .unwrap_or_else(|| panic!("{lane}: ct_eq(1 arg) must be a loud error"));
        assert!(
            err.contains("ct_eq"),
            "{lane}: the error must name the builtin, got: {err}"
        );
    }
    let bad_type = "pattern C(_input: String) -> String {\n  if ct_eq(1, \"a\") { return \"true\" }\n  return \"false\"\n}\nflow Main { input: String = \"x\" -> C -> output }";
    for (lane, out) in [("TW", run_tw(bad_type)), ("VM", run_vm(bad_type))] {
        let err = out
            .err()
            .unwrap_or_else(|| panic!("{lane}: ct_eq(Int, String) must be a loud error"));
        assert!(
            err.contains("ct_eq"),
            "{lane}: the error must name the builtin, got: {err}"
        );
    }
}

// ── request_header: outside a route (no server context) ─────────────

#[test]
fn n2_request_header_without_route_context_empty_both_backends() {
    // The sibling convention (query_param №40 / server_path_param №283):
    // no server context → the empty string. A webhook secret check
    // fails CLOSED (empty never matches the expected token) instead of
    // erroring off the server.
    let src = "pattern H(_input: String) -> String {\n  return request_header(\"X-Telegram-Bot-Api-Secret-Token\")\n}\nflow Main { input: String = \"x\" -> H -> output }";
    assert_eq!(tw_out(src), "", "TW: no context → empty string");
    assert_eq!(vm_out(src), "", "VM: no context → empty string");
}

#[test]
fn n2_request_header_lookup_is_case_insensitive_off_server() {
    // The name normalization is the builtin's own contract: the
    // off-server empty answer is identical whatever the name case.
    let src = "pattern H(_input: String) -> String {\n  return request_header(\"x-TELEGRAM-bot-API-secret-TOKEN\")\n}\nflow Main { input: String = \"x\" -> H -> output }";
    assert_eq!(tw_out(src), "", "TW: mixed-case name → empty off server");
    assert_eq!(vm_out(src), "", "VM: mixed-case name → empty off server");
}

#[test]
fn n2_request_header_arity_and_type_loud_both_backends() {
    let no_arg = "pattern H(_input: String) -> String {\n  return request_header()\n}\nflow Main { input: String = \"x\" -> H -> output }";
    for (lane, out) in [("TW", run_tw(no_arg)), ("VM", run_vm(no_arg))] {
        let err = out.err().unwrap_or_else(|| {
            panic!("{lane}: request_header() with no args must be a loud error")
        });
        assert!(
            err.contains("request_header"),
            "{lane}: the error must name the builtin, got: {err}"
        );
    }
    let bad_type = "pattern H(_input: String) -> String {\n  return request_header(42)\n}\nflow Main { input: String = \"x\" -> H -> output }";
    for (lane, out) in [("TW", run_tw(bad_type)), ("VM", run_vm(bad_type))] {
        let err = out
            .err()
            .unwrap_or_else(|| panic!("{lane}: request_header(Int) must be a loud error"));
        assert!(
            err.contains("request_header"),
            "{lane}: the error must name the builtin, got: {err}"
        );
    }
}
