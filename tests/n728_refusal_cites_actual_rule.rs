// Naryad #728 (issue #1355): the sink-clearance refusal cites the ACTUAL
// failed rule (gh#1350 Ask 3, the owner verbatim (a2)).
//
// Pins BOTH branches of the message:
// - an ACTION-sink refusal (http_post) keeps the №391 bridge text
//   byte-identical (the decision/data-argument wording);
// - a non-ACTION sink refusal (cron_remove — Sink/Internal/Irreversible,
//   the #1350 consumer report) names the generic sink rule + the №316
//   classification row and does NOT cite the bridge as if the call were
//   in the table (the consumer was sent to a table without the call).
//
// The failed threshold stays named in both branches (the
// explainable-refusal contract №392).

fn refusal_messages(source: &str) -> Vec<String> {
    // The compile path = the audit_category_a promotion (№98) — the
    // same messages `mlog check` prints (the #1350 report channel).
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

fn clearance_line(errs: &[String]) -> String {
    errs.iter()
        .find(|m| m.contains("sink clearance violated"))
        .cloned()
        .unwrap_or_else(|| panic!("the sink-clearance refusal must fire: {errs:?}"))
}

/// The #1350 minimal repro, verbatim: both forms pass on ≤ 0.33.0 and
/// both fire from 0.33.1 (the gh#1336 shared lowering made `each`
/// bodies visible to the sink-clearance pass).
const CRON_READBACK: &str = r#"
    pattern T() -> String {
        each cj in cron_list() {
            let _ = cron_remove(to_string(cj.prompt))
        }
        return "ok"
    }
"#;

#[test]
fn n728_action_sink_keeps_the_bridge_text() {
    let src = r#"
        entity record: String = "Иванов Иван, снилс 123-456-789, диагноз конфиденциален"
        pattern Send(data: String) -> String {
            let _ = http_post("https://analytics.example/collect", record)
            return "sent"
        }
    "#;
    let errs = refusal_messages(src);
    let hit = clearance_line(&errs);
    assert!(
        hit.contains(
            "bridge (№391): the decision/data argument of http_post must satisfy \
             conf ⊑ public AND integrity ≥ trusted"
        ),
        "the ACTION-sink refusal keeps the №391 bridge text byte-identical: {hit}"
    );
    assert!(
        !hit.contains("the six ACTION sinks only"),
        "the ACTION branch must not carry the non-ACTION wording: {hit}"
    );
    assert!(
        hit.contains("failed threshold: private-egress"),
        "the failed threshold stays named (№392): {hit}"
    );
}

#[test]
fn n728_non_action_sink_names_the_classification_row() {
    let errs = refusal_messages(CRON_READBACK);
    let hit = clearance_line(&errs);
    assert!(
        hit.contains(
            "cron_remove is classified sink/internal/irreversible \
             (the builtins classification, №316-class)"
        ),
        "the refusal names the №316 classification row (the SSOT strings): {hit}"
    );
    assert!(
        hit.contains("the bridge (№391) table covers the six ACTION sinks only"),
        "the refusal says where the bridge does NOT apply: {hit}"
    );
    assert!(
        !hit.contains("the decision/data argument of cron_remove"),
        "the refusal no longer cites the bridge as if cron_remove were an ACTION sink: {hit}"
    );
    assert!(
        hit.contains("carries label 'private, trusted'"),
        "the argument label stays named (№392): {hit}"
    );
    assert!(
        hit.contains("failed threshold: private-egress"),
        "the failed threshold stays named (№392): {hit}"
    );
}
