//! Наряд №376: configurable interprocedural taint depth + cross-module
//! summaries cache.
//!
//! Contracts (issue #443):
//!   1. Depth is configurable via `METALOGOS_TAINT_DEPTH` (1..=16; unset or
//!      invalid → the measured default 4). The `INTERP_DEPTH_LIMIT` warning
//!      stays and reports the CONFIGURED depth.
//!   2. The red/green examples (office dept/chain shapes of depth 3 and 4)
//!      are caught by the audit on the default configuration.
//!   3. The summaries cache recomputes only when the module (source content)
//!      changes — keyed by (source hash, depth).
//!   4. No stubs.
//!
//! Env-var and cache-counter tests are serialized through a process-local
//! mutex (the env and the cache are process-global; Rust runs tests in one
//! binary on parallel threads).

use metalogos::audit_program;

use std::sync::{Mutex, MutexGuard, OnceLock};

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn findings_for(source: &str) -> Vec<String> {
    match audit_program(source) {
        Ok(result) => result
            .findings
            .iter()
            .map(|f| f.check_id.to_string())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn read_example(name: &str) -> String {
    std::fs::read_to_string(std::path::Path::new("examples").join(name))
        .unwrap_or_else(|e| panic!("{} must exist: {}", name, e))
}

/// The red/green examples (office dept/chain depth 3 and 4) are CAUGHT on
/// the default configuration — TAINT_INTERP findings present; the sanitized
/// variant inside d4 does not add a TAINT_INTERP error for itself beyond the
/// unsanitized one.
#[test]
fn naryad_376_red_green_examples_caught_at_default() {
    let d3 = read_example("taint_chain_d3.mlog");
    let d4 = read_example("taint_chain_d4.mlog");

    let f3 = findings_for(&d3);
    assert!(
        f3.iter().any(|c| c == "TAINT_INTERP"),
        "depth-3 dept/chain must be flagged (pre-№376 hole): {:?}",
        f3
    );
    let f4 = findings_for(&d4);
    assert!(
        f4.iter().any(|c| c == "TAINT_INTERP"),
        "depth-4 dept/chain must be flagged: {:?}",
        f4
    );
    // Exactly one TAINT_INTERP error in d4 — the sanitized variant (render)
    // does not produce one.
    assert_eq!(
        f4.iter().filter(|c| c.as_str() == "TAINT_INTERP").count(),
        1,
        "the render() sanitizer path must not add a second TAINT_INTERP finding"
    );
}

/// The depth is configurable: the INTERP_DEPTH_LIMIT warning message reports
/// the configured depth (2 vs 4 vs invalid-fallback), and the configured
/// value survives an audit run.
#[test]
fn naryad_376_env_switch_changes_reported_depth() {
    let _g = env_lock();
    let cyclic = r#"
pattern Rec(x: String) -> String {
    return Rec(x)
}

pattern Handler() -> String {
    let r = respond("200 OK", Rec(call_llm("hi")))
    return r
}
"#;

    let depth_in_message = |depth: Option<&str>| -> usize {
        match depth {
            Some(d) => std::env::set_var("METALOGOS_TAINT_DEPTH", d),
            None => std::env::remove_var("METALOGOS_TAINT_DEPTH"),
        }
        let findings = match audit_program(cyclic) {
            Ok(r) => r.findings,
            Err(e) => panic!("audit failed: {}", e),
        };
        let msg = findings
            .iter()
            .find(|f| f.check_id == "INTERP_DEPTH_LIMIT")
            .unwrap_or_else(|| panic!("cyclic pattern must emit INTERP_DEPTH_LIMIT"))
            .message
            .clone();
        let anchor = "bounded at depth ";
        let pos = msg.find(anchor).expect("message shape");
        let tail = &msg[pos + anchor.len()..];
        let num: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
        num.parse::<usize>().expect("depth digits")
    };

    // Unset → the measured default (4).
    assert_eq!(depth_in_message(None), 4, "default depth must be 4");
    // Explicit 2 → 2 (the pre-№376 default is still selectable).
    assert_eq!(depth_in_message(Some("2")), 2);
    // Explicit 4 → 4.
    assert_eq!(depth_in_message(Some("4")), 4);
    // Invalid → falls back to the default (4).
    assert_eq!(depth_in_message(Some("banana")), 4);
    assert_eq!(depth_in_message(Some("0")), 4, "0 is outside 1..=16");
    assert_eq!(depth_in_message(Some("17")), 4, "17 is outside 1..=16");

    std::env::remove_var("METALOGOS_TAINT_DEPTH");
}

/// Cross-module summaries cache: auditing the SAME unchanged source twice
/// hits the cache (1 insert, ≥1 hit); auditing a CHANGED module inserts.
#[test]
fn naryad_376_summaries_cache_recompute_only_on_change() {
    let _g = env_lock();
    metalogos::audit::summaries_cache_clear();

    let module = r#"
pattern Wrap(x: String) -> String {
    return upper(x)
}

pattern Handler() -> String {
    let r = respond("200 OK", Wrap(call_llm("hi")))
    return r
}
"#;

    // First run: cache miss → insert.
    let _ = audit_program(module);
    let (ins1, hits1) = metalogos::audit::summaries_cache_stats();
    assert_eq!(ins1, 1, "first audit of a module must insert");
    assert_eq!(hits1, 0);

    // Second run of the SAME (unchanged) module: cache hit.
    let _ = audit_program(module);
    let (ins2, hits2) = metalogos::audit::summaries_cache_stats();
    assert_eq!(ins2, 1, "unchanged module must NOT recompute");
    assert!(hits2 >= 1, "unchanged module must hit the cache");

    // A CHANGED module (different content) → a new insert.
    let changed = module.replace("upper(x)", "trim(x)");
    let _ = audit_program(&changed);
    let (ins3, _) = metalogos::audit::summaries_cache_stats();
    assert_eq!(ins3, 2, "changed module must recompute (new insert)");

    // The findings stay CORRECT through the cache (contract: the cached
    // summaries are behaviorally identical).
    assert!(
        findings_for(module).iter().any(|c| c == "TAINT_INTERP"),
        "cached path must keep catching the chain"
    );
}

/// No stubs (№16.0-D) — markers assembled from parts to avoid self-matching.
#[test]
fn naryad_376_no_stubs() {
    let markers = [
        concat!("todo", "!"),
        concat!("unimplemented", "!"),
        concat!("SKELE", "TON"),
    ];
    let src = std::fs::read_to_string(file!()).unwrap_or_default();
    for m in markers {
        assert!(
            !src.contains(m),
            "naryad_376 test contains stub marker {}",
            m
        );
    }
}
