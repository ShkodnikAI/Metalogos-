// ── tests/naryad_449_effects.rs ─────────────────────────────────────
// Наряд №449 (Волна 15, issue #657): the static effects module (REALITY
// §2.4 was NO — `grep 'effect' src/audit.rs` empty).
//
// Contract groups pinned here:
//   (а) SSOT derivation: effect attributes {read, write, network,
//       irreversible} are derived SOLELY from the №316 classification
//       (Role × default Label × Reversibility) — every registry builtin
//       yields a defined effect set (classification drift is caught),
//       canonical mappings are pinned.
//   (б) Effect-trace: taint findings carry the effect context of the
//       involved builtins in the audit output.
//   (в) Severity escalation: a tainted argument reaching a
//       network-effect builtin (№316 `Label::Network` — the egress axis,
//       incl. the №316 DUAL prompt-egress of call_llm) is a Category-A
//       TAINT_INTERP error in the EXISTING category (no new check_id,
//       no new lattice).
//   (г) Zero false positives: sanitized flows and the write-only lanes
//       (memorize/db — TAINT_PERSISTENCE/clearance territory) stay out.

use metalogos::audit::builtin_effects;

fn has_finding(source: &str, check_id: &str) -> bool {
    match metalogos::audit_program(source) {
        Ok(result) => result.findings.iter().any(|f| f.check_id == check_id),
        Err(_) => false,
    }
}

fn has_no_findings_with_id(source: &str, check_id: &str) -> bool {
    !has_finding(source, check_id)
}

fn finding_messages_with(source: &str, check_id: &str) -> Vec<String> {
    match metalogos::audit_program(source) {
        Ok(result) => result
            .findings
            .iter()
            .filter(|f| f.check_id == check_id)
            .map(|f| f.message.clone())
            .collect(),
        Err(_) => Vec::new(),
    }
}

// ── (а) SSOT derivation ──────────────────────────────────────────────

/// Source-level registry names — the same feature-independent contract
/// the №316 tests use (include_str! parse of `spec!("name"`).
fn registry_source_names() -> Vec<String> {
    let src = include_str!("../src/builtins/registry.rs");
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("spec!(\"") {
        let after = &rest[i + 7..];
        if let Some(j) = after.find('"') {
            out.push(after[..j].to_string());
        }
        rest = &rest[i + 7..];
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn a1_every_registered_builtin_has_effect_output() {
    // №449 «Сделано, когда»: SSOT-покрытие 100% реестра — the effect
    // derivation is TOTAL over the classification; a builtin added to the
    // registry without a №316 row (classification drift) fails HERE.
    let missing: Vec<String> = registry_source_names()
        .into_iter()
        .filter(|n| builtin_effects(n).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "registry builtins without effect derivation ({}): {:?}",
        missing.len(),
        missing
    );
}

#[test]
fn a2_canonical_effect_mappings_are_pinned() {
    // From the №316 rows: db_insert (Sink, Internal, Irreversible),
    // print (Sink, Public, Irreversible), http_post (Sink, Network,
    // Irreversible), call_llm (Source, Network, Pure), memorize
    // (Sink, Internal, Reversible), upper (Pure).
    let e = |n: &str| builtin_effects(n).unwrap();

    let db = e("db_insert");
    assert!(db.write && db.irreversible && !db.network && !db.read);

    let pr = e("print");
    assert!(pr.write && pr.irreversible && !pr.network);

    let hp = e("http_post");
    assert!(hp.network && hp.write && hp.irreversible && !hp.read);

    let llm = e("call_llm");
    assert!(llm.read && llm.network && !llm.write && !llm.irreversible);

    // The anti-collision pin: memorize is write-only — the memory lane
    // stays TAINT_PERSISTENCE's territory, NOT an effects escalation sink.
    let mem = e("memorize");
    assert!(mem.write && !mem.network && !mem.irreversible && !mem.read);

    // Pure builtins claim nothing.
    let up = e("upper");
    assert!(!up.read && !up.write && !up.network && !up.irreversible);
}

// ── (б) Effect-trace in the audit output ────────────────────────────

#[test]
fn b1_taint_interp_message_carries_effect_trace() {
    // The sink argument's builtins (call_llm → read, network) appear in
    // the finding message — the effect context of the taint event.
    let source = "pattern Wrap(x: String) -> String {\n  return upper(x)\n}\npattern Send(data: String) -> String {\n  let _ = respond(\"200\", Wrap(call_llm(\"summarize the document\")))\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    let msgs = finding_messages_with(source, "TAINT_INTERP");
    assert!(
        !msgs.is_empty(),
        "the straight-line chain must fire TAINT_INTERP"
    );
    assert!(
        msgs.iter().any(|m| m.contains("effects:") && m.contains("network") && m.contains("read")),
        "the TAINT_INTERP message must carry the effect trace (read, network from call_llm); got: {:?}",
        msgs
    );
}

// ── (в) Severity escalation on the network axis ─────────────────────

#[test]
fn c1_prompt_egress_chain_is_taint_interp() {
    // LLM output fed as the PROMPT of the next LLM call — the №316 DUAL
    // note («the prompt is transmitted to an external provider») — was
    // clean before №449; the network-effect escalation catches it.
    let source = "pattern Send(data: String) -> String {\n  let poisoned = call_llm(\"first prompt\")\n  let _ = call_llm(poisoned)\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "call_llm(poisoned) — prompt egress over the network axis — must fire TAINT_INTERP"
    );
}

#[test]
fn c2_network_sink_chain_is_taint_interp() {
    // http_post carries the network effect (№316): the interprocedural
    // chain into it is a TAINT_INTERP event (alongside the clearance gate).
    let source = "pattern Wrap(x: String) -> String {\n  return upper(x)\n}\npattern Send(data: String) -> String {\n  let _ = http_post(\"https://hook.example/ingest\", Wrap(call_llm(\"summarize the document\")))\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "http_post(Wrap(call_llm(...))) must fire TAINT_INTERP (network-effect sink)"
    );
    let msgs = finding_messages_with(source, "TAINT_INTERP");
    assert!(
        msgs.iter().any(|m| m.contains("effects:")),
        "the network escalation must carry the effect trace"
    );
}

#[test]
fn c3_sanitized_network_sink_is_not_flagged() {
    // render() before the network sink lifts the taint — zero false
    // positives on the escalation path.
    let source = "pattern Send(data: String) -> String {\n  let raw = call_llm(\"summarize the document\")\n  let shown = render(raw)\n  let _ = http_post(\"https://hook.example/ingest\", shown)\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "sanitized data into a network-effect builtin must stay clean"
    );
}

// ── (г) Boundaries: no class collisions ─────────────────────────────

#[test]
fn d1_memorize_write_lane_is_not_an_effects_sink() {
    // The memory write lane keeps its own TAINT_PERSISTENCE contract —
    // the effects module must NOT add a TAINT_INTERP finding on
    // memorize(<llm>) (write-only, Internal label in №316).
    let source = "pattern Send(data: String) -> String {\n  let _ = memorize(\"k449\", call_llm(\"summarize the document\"))\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "memorize is write-only/Internal — no effects escalation (TAINT_PERSISTENCE's territory)"
    );
}

#[test]
fn d2_db_write_lane_is_not_an_effects_sink() {
    // Same boundary for the db write lane (Internal label): the effects
    // module escalates the NETWORK axis only; db destructive-write gates
    // (IRREVERSIBLE_NO_GRANT / clearance) stay the specialized checks.
    let source = "pattern Send(data: String) -> String {\n  let raw = call_llm(\"summarize the document\")\n  let _ = db_execute(\"INSERT INTO notes VALUES ('\" + raw + \"')\")\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "db_execute is write-only/Internal — no effects escalation on the network axis"
    );
}

#[test]
fn d3_legacy_sinks_keep_their_messages_and_severity() {
    // The №292/№448 straight-line contract re-pinned: the four legacy
    // sinks still fire TAINT_INTERP as Errors.
    let source = "pattern Wrap(x: String) -> String {\n  return upper(x)\n}\npattern Send(data: String) -> String {\n  let _ = print(Wrap(call_llm(\"summarize the document\")))\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(has_finding(source, "TAINT_INTERP"));
}
