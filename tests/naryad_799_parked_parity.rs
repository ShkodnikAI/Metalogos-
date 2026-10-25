// ── Naryad №799 (issue #1546, wave 65, P1 — the C3 (a) T2 decision) ────
//
// №475 fs_gate: the source-scan lock below exercises the REAL filesystem
// by design (the registry.rs SOURCE is the lock's subject — the same
// contract gen_reference.py uses); the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]
//
// The anti-drift lock for the parked surface (the №467 two-locks
// discipline, the №744 drift class): the SOURCE scan of registry.rs (the
// same contract gen_reference.py and type_signature_share.py use — the
// feature-gated rows ride the source metric only) pins SET-EQUALITY
// between the tier-"parked" spec! rows and the always-compiled
// `builtins::parked::PARKED_REGISTRY` table.
//
// A parked row added without its parked-table entry fails the lock (the
// default-build check would read the lie "undefined"); a parked-table
// entry without a row is drift the same way. The enum-kind and the
// origin-stamp pins ride this file (they compile only with the fix —
// the table IS the fix; the behavioral RED lives in
// naryad_799_builtin_parked.rs, which ran against the pre-№799 main).

use metalogos::builtins::parked::{parked_spec, refusal_message, PARKED_REGISTRY};
use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::values::{split_origin_stamp, CODE_BUILTIN_PARKED};
use metalogos::semantic::SemanticErrorKind;
use metalogos::{check_program, compile_program};

const REGISTRY_RS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/builtins/registry.rs");

/// One-line scan of the spec! source rows: the name and the tier (the
/// LAST quoted word INSIDE the balanced spec!(...) call — the optional
/// type string sits before it). The spec! calls are single-line in
/// registry.rs (the house form the generators rely on), but the
/// trailing `//` comments may carry parens — the call's close is found
/// by PAREN BALANCING from the `spec!(` origin (quote-aware), never by
/// rfind on the line.
fn source_parked_rows() -> Vec<(String, String)> {
    let src = std::fs::read_to_string(REGISTRY_RS).expect("registry.rs readable");
    let mut rows = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim();
        let Some(call_at) = trimmed.find("spec!(") else {
            continue;
        };
        let name = trimmed
            .split('"')
            .nth(1)
            .expect("spec! carries a quoted name")
            .to_string();
        // Paren balance from the `(` after `spec!`, quote-aware.
        let bytes = trimmed.as_bytes();
        let mut depth = 0usize;
        let mut in_str = false;
        let mut close = None;
        for (i, &b) in bytes.iter().enumerate().skip(call_at + 5) {
            match b {
                b'"' if i == 0 || bytes[i - 1] != b'\\' => in_str = !in_str,
                b'(' if !in_str => depth += 1,
                b')' if !in_str => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close.expect("spec! call balances on its own line");
        let head = &trimmed[..close];
        let open = head.rfind('"').expect("tier is the last quoted word");
        let before = &head[..open];
        let open2 = before.rfind('"').expect("tier opens with a quote");
        rows.push((name, head[open2 + 1..open].to_string()));
    }
    rows
}

#[test]
fn n799_parked_table_covers_every_parked_tier_row() {
    let rows = source_parked_rows();
    assert!(
        rows.len() >= 500,
        "the source scan must see the whole registry, got {} rows",
        rows.len()
    );
    let mut parked_from_source: Vec<String> = rows
        .iter()
        .filter(|(_, tier)| tier == "parked")
        .map(|(name, _)| name.clone())
        .collect();
    parked_from_source.sort();

    let mut parked_from_table: Vec<String> =
        PARKED_REGISTRY.iter().map(|p| p.name.to_string()).collect();
    parked_from_table.sort();

    assert_eq!(
        parked_from_source, parked_from_table,
        "№799 parity lock: every tier-\"parked\" spec! row must have a \
         builtins::parked entry, and the table must name nothing else \
         (the №744 drift class — a surface without its lock)"
    );
}

#[test]
fn n799_parked_table_features_are_real_cargo_features() {
    // The feature names the refusals advertise must be real [features]
    // of Cargo.toml — a typo would send the user to a flag that does
    // nothing (the honest-refusal discipline).
    let cargo = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml readable");
    for p in PARKED_REGISTRY {
        if let Some(f) = p.feature {
            assert!(
                cargo.contains(&format!("\n{} = [", f)) || cargo.contains(&format!("\n{} =", f)),
                "№799: the parked feature `{}` (advertised by {}) must be a real \
                 Cargo.toml feature",
                f,
                p.name
            );
        }
    }
}

#[test]
fn n799_parked_code_is_origin_stamped() {
    // The coded-error contract (№413 mechanics): [BUILTIN_PARKED] is
    // branchable through `try r.error.code` — the ONE classification
    // point (`split_origin_stamp`, the whitelist the TW/VM sewing points
    // share) must stamp the refusal's class at position 0. The runtime
    // surface for the branchable read is the TW eval/REPL lane (the
    // check gate on run/serve refuses EARLIER — its position-0 stamp is
    // pinned by the behavior tests); the VM lane refuses at compile
    // (the coded №479 origin, pinned above).
    let msg = refusal_message(parked_spec("voice_enroll").expect("parked row"));
    let (code, _rest) =
        split_origin_stamp(&msg).expect("№799: the refusal must carry the origin stamp");
    assert_eq!(
        code, CODE_BUILTIN_PARKED,
        "№799: BUILTIN_PARKED must be origin-stamped (the try-branching \
         mechanics; the №413 contract)"
    );
}

#[test]
fn n799_check_finding_carries_the_structured_kind() {
    // The №523 discipline: the gates classify by the ENUM, never by the
    // message text — the parked finding must carry BuiltinParked.
    let result = check_program(
        r#"
pattern Gen(p: String) -> String {
  let _ = vision_generate(p, "a cat")
  return "done"
}
"#,
    )
    .expect("parse ok");
    let parked = result
        .errors
        .iter()
        .filter(|e| e.kind == SemanticErrorKind::BuiltinParked)
        .count();
    assert!(
        parked >= 1,
        "№799: the parked finding must carry the BuiltinParked kind; got: {:?}",
        result.errors
    );
    // And the stable code the run gate stamps (position 0 of the №523
    // refusal) is BUILTIN_PARKED.
    assert_eq!(
        SemanticErrorKind::BuiltinParked.stable_code(),
        Some(CODE_BUILTIN_PARKED),
        "№799: the parked kind stamps the shared constant"
    );
}

#[test]
fn n799_compiled_default_registry_carries_no_parked_row() {
    // The T2 acceptance: the default build compiles NO feature-gated
    // parked row — every compiled row's tier differs from "parked"
    // EXCEPT the environment-parked exception (reflex_bpe_load: no
    // feature to gate it with, the handler never returns Ok in this
    // environment — №717/№195; the row stays registered, the runtime
    // refusal is its documented contract).
    let parked_compiled: Vec<&str> = BUILTIN_REGISTRY
        .iter()
        .filter(|s| s.tier == "parked")
        .map(|s| s.name)
        .collect();
    assert_eq!(
        parked_compiled,
        vec!["reflex_bpe_load"],
        "№799: the default build must compile no FEATURE-GATED parked row — \
         only the environment-parked exception rides (reflex_bpe_load)"
    );
    // The names remain KNOWN: the parked table answers for them.
    assert!(parked_spec("vision_generate").is_some());
    assert!(parked_spec("voice_enroll").is_some());
    assert!(parked_spec("reflex_generate").is_some());
    assert!(parked_spec("video_render").is_some());
    assert!(parked_spec("not_a_builtin_at_all").is_none());
}

#[test]
fn n799_vm_compile_refusal_is_a_coded_error_with_the_parked_code() {
    // The №479 origin form: the coded error carries the stable code the
    // diff-fuzzer's class signature reads.
    let err = compile_program(
        r#"
pattern Enroll(d: String) -> String {
  let _ = voice_enroll(d, "audio.wav")
  return "done"
}
"#,
    )
    .expect_err("the parked call refuses at VM compile in the default build");
    assert!(
        err.contains("[BUILTIN_PARKED]") && err.contains("--features voice"),
        "№799: the VM compile origin must carry [BUILTIN_PARKED] naming the \
         feature; got: {err}"
    );
}
