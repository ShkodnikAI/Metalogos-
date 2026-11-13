// ── №955 (issue #1922, the audit 12.11): error texts never quote values ─
//
// The audit's H-3 (probe p14 + 6_serve/e.mlog): `to_int(env("KEY"))`
// produced `[TYPE_MISMATCH] to_int("TOPSECRET123"): …` and `serve`
// returned the FULL text to the client in the HTTP 500 body.
//
// The fix (this PR):
//   1. the builtin-level masking — the inventoried error messages quote
//      `string(len=N)` instead of the string VALUE (the conservative
//      pre-№952 form: the runtime cannot tell a public string from a
//      labeled one, so it quotes nothing);
//   2. the serve-level generic 500 — the client receives
//      `Internal error (id: err-N)`; the full (redact-masked) text is
//      the server LOG's, keyed by the same id;
//      `METALOGOS_SERVE_VERBOSE_ERRORS=1` is the LOUD development
//      opt-out (the startup banner fires).
//
// The inventory (the sites masked by this PR): to_float/to_int
// (math.rs), date_parse_iso (calendar_utc.rs), the smtp/imap address,
// folder, host, uid sites (email.rs), the smfs virtual-path sites
// (smfs.rs). The mutation: scripts/mutation_verify_955.sh restores the
// raw `{s:?}` in math.rs → the pin below goes RED.
#![allow(clippy::disallowed_methods)]

use serial_test::serial;

fn run_with_secret(source: &str) -> String {
    let dir = std::env::temp_dir().join(format!("n955_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::env::set_var("SP", "TOPSECRET123");
    let original_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&dir).expect("set cwd");
    let ran = metalogos::run_program_on_vm(source);
    let _ = std::env::set_current_dir(original_cwd);
    match ran {
        Err(e) => e,
        Ok(_) => panic!("the probe program must refuse"),
    }
}

/// THE probe (p14): the to_int error text carries the masked form —
/// never the value.
#[test]
#[serial]
fn n955_to_int_error_text_masks_the_value() {
    let err = run_with_secret(
        "pattern T(x: String) -> String { let n = to_int(env(\"SP\")) return \"ok\" }\nflow Main { input: String = \"x\" -> T -> output }",
    );
    assert!(
        err.contains("to_int(string(len=12))"),
        "the error text must carry the masked form, got: {}",
        err
    );
    assert!(
        !err.contains("TOPSECRET123"),
        "THE SECRET LEAKED INTO THE ERROR TEXT: {}",
        err
    );
}

/// The to_float twin (the same inventoried family).
#[test]
#[serial]
fn n955_to_float_error_text_masks_the_value() {
    let err = run_with_secret(
        "pattern T(x: String) -> String { let n = to_float(env(\"SP\")) return \"ok\" }\nflow Main { input: String = \"x\" -> T -> output }",
    );
    assert!(
        err.contains("to_float(string(len=12))") && !err.contains("TOPSECRET123"),
        "the masked form expected, got: {}",
        err
    );
}
