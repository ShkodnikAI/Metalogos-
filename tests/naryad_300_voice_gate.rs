// ── tests/naryad_300_voice_gate.rs ─────────────────────────────────
// Наряд №300 (issue #368, P1/security): VOICE_PREREQ_GATE —
// MODEL_WEIGHTS_UNSAFE generalized from vision_fetch_weights literal
// to SSOT list + suffix convention _fetch_weights.
//
// Tests: voice_fetch_weights (suffix) caught by same check_id;
// negative: http_get with same URL NOT caught (no false positive).

use metalogos::audit::audit_category_a;
use metalogos::parser;

fn findings_of(source: &str) -> Vec<metalogos::audit::AuditFinding> {
    let decls = parser::parse(source).unwrap();
    audit_category_a(&decls, source)
}

fn by_id<'a>(
    findings: &'a [metalogos::audit::AuditFinding],
    id: &'a str,
) -> Vec<&'a metalogos::audit::AuditFinding> {
    findings.iter().filter(|f| f.check_id == id).collect()
}

// ── voice_fetch_weights (suffix convention) caught by MODEL_WEIGHTS_UNSAFE ──

#[test]
fn n300_voice_fetch_weights_bare_safetensors() {
    let source = r#"
pattern Fetch(dir: String) -> String {
    return voice_fetch_weights("https://huggingface.co/pkg/model.safetensors", dir)
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
    assert_eq!(
        hits.len(),
        1,
        "voice_fetch_weights with bare .safetensors must trigger MODEL_WEIGHTS_UNSAFE, got: {:?}",
        findings
    );
    assert_eq!(hits[0].severity, metalogos::audit::Severity::Error);
}

#[test]
fn n300_voice_fetch_weights_pickle_class() {
    let source = r#"
pattern Fetch(dir: String) -> String {
    return voice_fetch_weights("https://huggingface.co/pkg/weights.pkl", dir)
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
    assert_eq!(
        hits.len(),
        1,
        "voice_fetch_weights with .pkl must trigger MODEL_WEIGHTS_UNSAFE"
    );
}

#[test]
fn n300_voice_fetch_weights_ssrf_blocked() {
    let source = r#"
pattern Fetch(dir: String) -> String {
    return voice_fetch_weights("http://169.254.169.254/latest/manifest.json", dir)
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
    assert_eq!(
        hits.len(),
        1,
        "voice_fetch_weights with metadata IP must trigger MODEL_WEIGHTS_UNSAFE"
    );
}

// ── Negative: http_get with same URL NOT caught by MODEL_WEIGHTS_UNSAFE ──

#[test]
fn n300_http_get_not_caught_by_model_weights_gate() {
    let source = r#"
pattern Fetch(url: String) -> String {
    return http_get("https://huggingface.co/pkg/model.safetensors")
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
    assert!(
        hits.is_empty(),
        "http_get must NOT trigger MODEL_WEIGHTS_UNSAFE (no false positive), got: {:?}",
        findings
    );
}

// ── Existing vision_fetch_weights still works (no regression) ──

#[test]
fn n300_vision_fetch_weights_still_caught() {
    let source = r#"
pattern Fetch(dir: String) -> String {
    return vision_fetch_weights("https://huggingface.co/pkg/model.safetensors", dir)
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
    assert_eq!(
        hits.len(),
        1,
        "vision_fetch_weights must still be caught (no regression)"
    );
}
