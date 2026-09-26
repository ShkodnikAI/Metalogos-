// ── Naryad #458 (P1, security/audit): HARDCODED_SECRET — the Category-A
//    half of the secrets check ──────────────────────────────────────────
//
// Contract (issue #677, audit 25.09 finding 3.6 Medium):
// 1. The audit finding: `check_secrets` ran ONLY in `mlog audit`
//    (audit_program) — a pattern with a real provider token format as a
//    string literal COMPILED and RAN. The №102 pattern vocabulary already
//    covers the provider formats; №458 moves the high-precision half into
//    `audit_category_a` as the `HARDCODED_SECRET` compile error — blocking
//    on run/compile/serve/mcp-serve (the four enforcement points call
//    `audit_category_a` already).
// 2. The split: high-precision provider prefixes (GitHub, Anthropic, AWS,
//    Slack, GitLab, Google, PEM) block; the name heuristics (`api_key`,
//    `token=`) and the generic patterns stay WARNING-level (`SECRETS`,
//    `mlog audit`) — NOT tightened (the naryad boundary).
// 3. The escape crane: secrets come from `env()` or bind to a
//    `Secret`-typed entity; EXAMPLE-marked placeholders (the AWS docs key
//    convention and the №459 synthetic-fixture format) stay clean.
// 4. The false-positive line: the repository's own example corpus
//    (`examples/*.mlog`) carries ZERO `HARDCODED_SECRET` findings — the
//    scan runs right here, on every CI run (a new fixture with a
//    real-format literal must either use the EXAMPLE convention or be a
//    documented exclusion — silence is not an option).
//
// NOTE: the probes build their token-shaped strings programmatically
// (prefix + a cycled body) — a test file full of real-looking token
// literals would itself be the anti-pattern this naryad polices.

use metalogos::audit::{audit_category_a, audit_program, Severity};

/// Build a token-shaped probe literal: prefix + a body of `n` chars.
fn tok(prefix: &str, n: usize) -> String {
    let body: String = "AbCdEf0123456789".chars().cycle().take(n).collect();
    format!("{prefix}{body}")
}

// ── 1. The blocking line: a provider-format literal refuses to run ────

#[test]
fn n458_provider_format_literal_refuses_to_compile() {
    let src = format!(
        r#"
pattern P() -> String {{
  let t = "{}"
  return t
}}
"#,
        tok("ghp_", 36)
    );
    let findings = audit_category_a(&metalogos::parser::parse(&src).expect("parse"), "");
    let hit = findings
        .iter()
        .find(|f| f.check_id == "HARDCODED_SECRET")
        .expect("a ghp_-format literal must be a Category-A finding");
    assert_eq!(
        hit.severity,
        Severity::Error,
        "the finding must be an Error"
    );
    assert!(
        hit.message.contains("read secrets from env()"),
        "the message must name the escape crane: {}",
        hit.message
    );

    // The full run path refuses (the lib.rs enforcement point — the same
    // audit_category_a pass `cmd_run` calls).
    let err = metalogos::run_program(&src).expect_err("the program must refuse to run");
    assert!(
        err.contains("HARDCODED_SECRET"),
        "the run refusal must carry the stable code: {}",
        err
    );
}

#[test]
fn n458_each_provider_format_blocks() {
    let cases: Vec<(&str, String)> = vec![
        ("github-classic", tok("ghp_", 36)),
        ("github-fine-grained", tok("github_pat_", 40)),
        ("anthropic", tok("sk-ant-api03-", 42)),
        ("aws", tok("AKIA", 16)),
        ("aws-temp", tok("ASIA", 16)),
        ("slack", tok("xoxb-0123456789-", 24)),
        ("gitlab", tok("glpat-", 34)),
        ("google", tok("AIza", 33)),
        ("pem", "-----BEGIN RSA PRIVATE KEY-----".to_string()),
    ];
    for (name, token) in cases {
        let src = format!("pattern P() -> String {{\n  let t = \"{token}\"\n  return t\n}}\n");
        let findings = audit_category_a(&metalogos::parser::parse(&src).expect("parse"), "");
        assert!(
            findings
                .iter()
                .any(|f| f.check_id == "HARDCODED_SECRET" && f.severity == Severity::Error),
            "{name}: the provider-format literal must refuse at compile time"
        );
    }
}

// ── 2. The escape crane: env() and EXAMPLE fixtures stay clean ────────

#[test]
fn n458_env_sourced_secret_stays_clean() {
    // The SAME token shape as the blocking probe, but READ, not pasted.
    let src = r#"
pattern P() -> String {
  let t = env("N458_TOKEN_FROM_ENV")
  return t
}
"#;
    let findings = audit_category_a(&metalogos::parser::parse(src).expect("parse"), "");
    assert!(
        !findings.iter().any(|f| f.check_id == "HARDCODED_SECRET"),
        "env() is the escape crane — no finding allowed: {:?}",
        findings
    );
}

#[test]
fn n458_example_marked_fixtures_stay_clean() {
    // The AWS docs key convention (AKIA + 16 chars ending in EXAMPLE) and
    // the synthetic-fixture format of №459 must not trip the gate.
    let aws_docs_key = format!("AKIA{}EXAMPLE", "IOSFODNN7");
    let ghp_fixture = format!("ghp_EXAMPLE_{}", "0123456789_".repeat(3));
    let src = format!(
        r#"
pattern P() -> String {{
  let aws = "{aws}"
  let ghp = "{ghp}"
  return aws + ghp
}}
"#,
        aws = aws_docs_key,
        ghp = ghp_fixture
    );
    let findings = audit_category_a(&metalogos::parser::parse(&src).expect("parse"), "");
    assert!(
        !findings.iter().any(|f| f.check_id == "HARDCODED_SECRET"),
        "EXAMPLE-marked fixtures stay clean: {:?}",
        findings
    );
}

// ── 3. The split: name heuristics stay WARNING-level (not tightened) ───

#[test]
fn n458_name_heuristics_stay_warning_level() {
    // A generic name-heuristic literal ≥30 chars: the warning-level
    // SECRETS check fires in `mlog audit`, but compilation is NOT blocked
    // (the №458 boundary: the heuristics are not promoted).
    let src = format!(
        r#"
pattern P() -> String {{
  let t = "{}"
  return t
}}
"#,
        tok("api_key_", 30)
    );
    let decls = metalogos::parser::parse(&src).expect("parse");

    // Compiles: no Category-A finding.
    let cat_a = audit_category_a(&decls, "");
    assert!(
        !cat_a.iter().any(|f| f.check_id == "HARDCODED_SECRET"),
        "name heuristics must NOT block compilation: {:?}",
        cat_a
    );

    // Still warned: the warning-level check sees it.
    let result = audit_program(&src).expect("audit");
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.check_id == "SECRETS" && f.severity == Severity::Warning),
        "the name-heuristic warning must survive in mlog audit"
    );
}

#[test]
fn n458_short_prefix_strings_stay_clean() {
    // The precision guard parity: a SHORT string with a provider prefix
    // (a demo placeholder, not a real token) does not block.
    let src = r#"
pattern P() -> String {
  return "ghp_x"
}
"#;
    let findings = audit_category_a(&metalogos::parser::parse(src).expect("parse"), "");
    assert!(
        !findings.iter().any(|f| f.check_id == "HARDCODED_SECRET"),
        "below the length threshold the literal is not a secret: {:?}",
        findings
    );
}

// ── 4. The false-positive line: the repository's own corpus ───────────

#[test]
fn n458_no_false_positives_on_the_repo_corpus() {
    // Every example program in examples/**.mlog audits CLEAN of
    // HARDCODED_SECRET. The intentional audit-bait fixture
    // (examples/p102_secret_patterns.mlog) carries EXAMPLE-marked
    // placeholders, so the blocking check must agree with it.
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let examples = repo.join("examples");
    let mut scanned = 0usize;
    let mut hits: Vec<String> = Vec::new();
    let mut stack = vec![examples.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read_dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "mlog").unwrap_or(false) {
                let source = std::fs::read_to_string(&path).expect("read example");
                if let Ok(decls) = metalogos::parser::parse(&source) {
                    scanned += 1;
                    for f in audit_category_a(&decls, "") {
                        if f.check_id == "HARDCODED_SECRET" {
                            hits.push(format!(
                                "{}: {}",
                                path.strip_prefix(&repo).unwrap_or(&path).display(),
                                f.message
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        scanned > 200,
        "the corpus scan must actually run (scanned {})",
        scanned
    );
    assert!(
        hits.is_empty(),
        "HARDCODED_SECRET false positives on the repo corpus:\n{}",
        hits.join("\n")
    );
}
