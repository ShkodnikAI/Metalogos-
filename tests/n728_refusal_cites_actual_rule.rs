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

/// A foreign key into a non-ACTION sink (the n325 private-entity
/// shape): no paired-source provenance — the gate fires and the refusal
/// must name the classification row (№728).
///
/// NOTE (the №729 interaction): the #1350 flat repro (the cron_list →
/// cron_remove read-back) used to be this pin's program — the SAME
/// train's №729 (the sanctioned read-back pair recognition, the owner
/// verbatim (a)) makes that exact shape compile GREEN by design, so the
/// non-ACTION refusal pin rides a foreign key now (the №729 test file
/// carries the read-back GREEN pins).
const FOREIGN_KEY: &str = r#"
    entity stale_id: String = "Иванов Иван, снилс 123-456-789"
    pattern T() -> String {
        let _ = cron_remove(stale_id)
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
    let errs = refusal_messages(FOREIGN_KEY);
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
