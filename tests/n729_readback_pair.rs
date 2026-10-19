// Naryad #729 (issue #1356): the sanctioned read-back pair recognition
// (gh#1350 Ask 2, the owner verbatim (a)).
//
// Pins GREEN: the #1350 consumer shapes (the gh#1336 coverage extension
// made them compile-red on 0.33.1) — the flat repro verbatim, the
// let-chain variant, the each-over-a-let-bound-source variant, and the
// mtree twin (mtree_retrieve → mtree_forget).
//
// Pins RED (the conservative boundary, fail-closed №523/№531): a
// foreign key (no paired-source provenance), a foreign sink (the pair
// does not extend to other sinks), and the payload position
// (mtree_store has NO key position — its arg 0 is the payload text;
// the #1350 issue's «mtree reads → mtree_store» conjecture corrected
// by the source: memory.rs:478).

fn compile_errors(source: &str) -> Vec<String> {
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

#[test]
fn n729_the_flat_readback_repro_compiles_green() {
    // The #1350 minimal repro, verbatim: green on ≤ 0.33.0, red from
    // 0.33.1 — the sanctioned recipe restores the green.
    let src = r#"
        pattern T() -> String {
            each cj in cron_list() {
                let _ = cron_remove(to_string(cj.prompt))
            }
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "the sanctioned read-back round-trip must compile: {errs:?}"
    );
}

#[test]
fn n729_the_let_chain_keeps_the_domain() {
    let src = r#"
        pattern T() -> String {
            each cj in cron_list() {
                let key = to_string(cj.prompt)
                let _ = cron_remove(key)
            }
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "the let-chain read-back key must keep the domain: {errs:?}"
    );
}

#[test]
fn n729_each_over_a_let_bound_source_keeps_the_domain() {
    let src = r#"
        pattern T() -> String {
            let rows = cron_list()
            each cj in rows {
                let _ = cron_remove(to_string(cj.prompt))
            }
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "the each-over-a-let-bound-source variant must compile: {errs:?}"
    );
}

#[test]
fn n729_the_mtree_twin_forget_compiles_green() {
    let src = r#"
        pattern T() -> String {
            each e in mtree_retrieve("stale") {
                let _ = mtree_forget(e.id)
            }
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.is_empty(),
        "the mtree twin (mtree_retrieve → mtree_forget) must compile: {errs:?}"
    );
}

#[test]
fn n729_a_foreign_key_stays_refused() {
    // A private constant as the key — no paired-source provenance, the
    // gate stays (the n325 private-entity shape).
    let src = r#"
        entity stale_id: String = "Иванов Иван, снилс 123-456-789"
        pattern T() -> String {
            let _ = cron_remove(stale_id)
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_NETWORK]")),
        "a key without the paired-source provenance stays refused: {errs:?}"
    );
}

#[test]
fn n729_the_pair_does_not_extend_to_foreign_sinks() {
    // The same read-back value into a NON-paired sink — full gate.
    let src = r#"
        pattern T() -> String {
            each cj in cron_list() {
                let _ = http_post("https://analytics.example/collect", to_string(cj.prompt))
            }
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_NETWORK]")),
        "the read-back exemption must not extend to a foreign sink: {errs:?}"
    );
}

#[test]
fn n729_mtree_store_has_no_key_position_and_stays_gated() {
    // mtree_store's arg 0 is the PAYLOAD text (auto-assigned id,
    // admission-scored) — not a lookup key; the store stays gated in
    // full (the TAINT_PERSISTENCE vocabulary).
    let src = r#"
        entity note: String = "снилс 123-456-789, диагноз конфиденциален"
        pattern T() -> String {
            let _ = mtree_store(note)
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[TAINT_PERSISTENCE]")),
        "mtree_store (no key position) stays gated: {errs:?}"
    );
}
