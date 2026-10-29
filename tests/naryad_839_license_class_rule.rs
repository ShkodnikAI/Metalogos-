// ── Naryad №839 (issue #1641, wave 71 — W-A, P1, [security][compiler]):
// the license-class × profile permit rule — non-osi/restrictive backend
// selections only in a profile that EXPLICITLY permits the class; the
// refusal is loud, BEFORE run, with the dedicated
// LICENSE_CLASS_NOT_ALLOWED code.
//
// Contracts:
//   (1) the negatives (the leak-suite RED pair, mirrored here): a
//       literal ladder to a NonOsi rung / a Restrictive rung without a
//       permit does not compile — the error names the dedicated code;
//   (2) the positives: Osi rungs run permit-free (the default posture);
//       NonOsi with the named `allow_license_class: "non-osi"` permit
//       compiles; Restrictive with `"restrictive"` compiles;
//   (3) the permit is per-CLASS: a non-osi permit does NOT unlock
//       restrictive, and vice versa; a bogus class word is a loud
//       profile error; an empty licensing declaration is loud;
//   (4) the backward compat: the advisory bridge
//       (`backends: permissive_with_audit`) still unlocks both classes
//       (the ADR-0163 surface unchanged) and stays audited (Info events);
//   (5) the vision-decl model field is a selection position: a NonOsi
//       weights id without a permit refuses; the Osi one compiles;
//   (6) the named-permit program rides the audit lane as events (never
//       silent — the №333 posture holds under the named unlock too).
//
// RED discipline: the leak pair (n450/n451) EXPECTs the dedicated code —
// on unpatched main the ladder program COMPILES (the rung names never
// reached the №333 weights-id scan; the w1_degrade golden rode that
// gap) — "not caught" on the blocking runner. The w1_degrade golden now
// carries the named permits (the corpus migration this rule requires).

use metalogos::audit::{audit_category_a, Severity};

fn compile(src: &str) -> Result<(), String> {
    metalogos::compile_program(src.trim()).map(|_| ())
}

fn audit_findings(src: &str) -> Vec<(Severity, String)> {
    let decls = metalogos::parser::parse(src.trim()).expect("parses");
    audit_category_a(&decls, "")
        .into_iter()
        .map(|f| (f.severity, f.check_id.to_string()))
        .collect()
}

const LADDER_NONOSI: &str = r#"
pattern Pick(_tick: String) -> String {
  let sel = backend_select("omni", ["nemotron-omni"])
  if sel.ok {
    return "selected:" + sel.backend
  }
  return "degraded"
}
flow Main { input: String = "x" -> Pick -> output }
"#;

const LADDER_RESTRICTIVE: &str = r#"
pattern Pick(_tick: String) -> String {
  let sel = backend_select("omni", ["wall-oss"])
  if sel.ok {
    return "selected:" + sel.backend
  }
  return "degraded"
}
flow Main { input: String = "x" -> Pick -> output }
"#;

const LADDER_OSI: &str = r#"
pattern Pick(_tick: String) -> String {
  let sel = backend_select("tts", ["chatterbox"])
  if sel.ok {
    return "selected:" + sel.backend
  }
  return "degraded"
}
flow Main { input: String = "x" -> Pick -> output }
"#;

// ── (1) The negatives: no permit → loud refusal with the code ────────

#[test]
fn nonosi_ladder_without_permit_refuses_with_dedicated_code() {
    let err = compile(LADDER_NONOSI).expect_err("the NonOsi ladder must refuse without a permit");
    assert!(
        err.contains("LICENSE_CLASS_NOT_ALLOWED"),
        "expected the dedicated code, got: {err}"
    );
    assert!(
        err.contains("nemotron-omni") && err.contains("non-osi"),
        "the refusal must name the rung and the class, got: {err}"
    );
}

#[test]
fn restrictive_ladder_without_permit_refuses_with_dedicated_code() {
    let err = compile(LADDER_RESTRICTIVE)
        .expect_err("the Restrictive ladder must refuse without a permit");
    assert!(
        err.contains("LICENSE_CLASS_NOT_ALLOWED"),
        "expected the dedicated code, got: {err}"
    );
    assert!(
        err.contains("wall-oss") && err.contains("restrictive"),
        "the refusal must name the rung and the class, got: {err}"
    );
}

#[test]
fn refusal_fires_before_run_on_the_compile_path() {
    // The compile→bytecode path (the VM lane) rides the SAME compile
    // gates — the refusal is backend-independent by construction (the
    // №673 family shape).
    let err =
        metalogos::compile_program(LADDER_NONOSI.trim()).expect_err("the compile path must refuse");
    assert!(err.contains("LICENSE_CLASS_NOT_ALLOWED"), "got: {err}");
}

// ── (2) The positives ────────────────────────────────────────────────

#[test]
fn osi_ladder_needs_no_permit() {
    compile(LADDER_OSI).unwrap_or_else(|e| panic!("osi rungs run permit-free, got: {e}"));
}

#[test]
fn nonosi_with_named_permit_compiles() {
    let src = format!(
        "profile licensing {{ allow_license_class: \"non-osi\" }}\n{}",
        LADDER_NONOSI
    );
    compile(&src).unwrap_or_else(|e| panic!("the named non-osi permit unlocks, got: {e}"));
}

#[test]
fn restrictive_with_named_permit_compiles() {
    let src = format!(
        "profile licensing {{ allow_license_class: \"restrictive\" }}\n{}",
        LADDER_RESTRICTIVE
    );
    compile(&src).unwrap_or_else(|e| panic!("the named restrictive permit unlocks, got: {e}"));
}

#[test]
fn multi_class_permit_compiles_both() {
    let src = format!(
        "profile licensing {{ allow_license_class: \"non-osi\" allow_license_class: \"restrictive\" }}\n{}",
        LADDER_NONOSI
    );
    compile(&src).unwrap_or_else(|e| panic!("the repeatable permit form unlocks, got: {e}"));
}

// ── (3) The permit is per-class; the shapes stay loud ────────────────

#[test]
fn nonosi_permit_does_not_unlock_restrictive() {
    let src = format!(
        "profile licensing {{ allow_license_class: \"non-osi\" }}\n{}",
        LADDER_RESTRICTIVE
    );
    let err = compile(&src).expect_err("a non-osi permit must not unlock restrictive");
    assert!(err.contains("LICENSE_CLASS_NOT_ALLOWED"), "got: {err}");
}

#[test]
fn restrictive_permit_does_not_unlock_nonosi() {
    let src = format!(
        "profile licensing {{ allow_license_class: \"restrictive\" }}\n{}",
        LADDER_NONOSI
    );
    let err = compile(&src).expect_err("a restrictive permit must not unlock non-osi");
    assert!(err.contains("LICENSE_CLASS_NOT_ALLOWED"), "got: {err}");
}

#[test]
fn bogus_class_word_is_loud_on_every_lane() {
    // The bogus word never unlocks anything: on the compile path the
    // license-class rule refuses the non-permitted selection (the bogus
    // word resolved to NO permit — fail-closed); on the `mlog check`
    // lane the profile validation names the unknown class loudly.
    let src = format!(
        "profile licensing {{ allow_license_class: \"permissive\" }}\n{}",
        LADDER_NONOSI
    );
    let err = compile(&src).expect_err("a bogus class word must never unlock");
    assert!(
        err.contains("LICENSE_CLASS_NOT_ALLOWED"),
        "the compile lane refuses fail-closed, got: {err}"
    );
    let analysis = metalogos::check_program(&src).expect("check_program parses");
    let check_text: String = analysis
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        check_text.contains("unknown license class"),
        "the check lane names the unknown class, got: {check_text}"
    );
}

#[test]
fn empty_licensing_declaration_is_loud_on_the_check_lane() {
    let src = format!("profile licensing {{ }}\n{}", LADDER_OSI);
    let analysis = metalogos::check_program(&src).expect("check_program parses");
    let check_text: String = analysis
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        check_text.contains("requires options"),
        "expected the no-op-mistake refusal, got: {check_text}"
    );
}

// ── (4) The bridge stays the blanket unlock (backward compat) ────────

#[test]
fn advisory_bridge_still_unlocks_both_classes() {
    let src = format!(
        "profile licensing {{ backends: permissive_with_audit }}\n{}",
        LADDER_NONOSI
    );
    compile(&src).unwrap_or_else(|e| panic!("the bridge unlocks (ADR-0163 surface), got: {e}"));
    let src = format!(
        "profile licensing {{ backends: permissive_with_audit }}\n{}",
        LADDER_RESTRICTIVE
    );
    compile(&src).unwrap_or_else(|e| panic!("the bridge unlocks restrictive too, got: {e}"));
}

#[test]
fn bridge_unlocked_weights_id_mention_stays_audited() {
    // The №333 contract under the bridge: Info events, never silence.
    let src = r#"
profile licensing { backends: permissive_with_audit }

pattern Serve() -> String {
  let omni_model = "nemotron-3-nano-omni-30b-a3b"
  return "ok"
}
flow Main { input: String = "x" -> Serve -> output }
"#;
    compile(src).unwrap_or_else(|e| panic!("the bridge compiles, got: {e}"));
    let sevs = audit_findings(src);
    assert!(
        sevs.iter()
            .any(|(s, c)| c == "BACKEND_LICENSE" && *s == Severity::Info),
        "the audited-usage event must remain, got: {sevs:?}"
    );
}

#[test]
fn named_permit_unlocked_weights_id_mention_is_audited_too() {
    // The named unlock does not silence the audit lane — the №333
    // "allowed, but never silent" posture holds under the named permit.
    let src = r#"
profile licensing { allow_license_class: "non-osi" }

pattern Serve() -> String {
  let omni_model = "nemotron-3-nano-omni-30b-a3b"
  return "ok"
}
flow Main { input: String = "x" -> Serve -> output }
"#;
    compile(src).unwrap_or_else(|e| panic!("the named permit compiles, got: {e}"));
    let sevs = audit_findings(src);
    assert!(
        sevs.iter()
            .any(|(s, c)| c == "BACKEND_LICENSE" && *s == Severity::Info),
        "the named permit must keep the usage audited, got: {sevs:?}"
    );
}

#[test]
fn default_posture_weights_id_refusal_unchanged() {
    // The №333 distribution refusal is byte-stable (the w1_license_gate
    // error contract depends on the substring).
    let err = compile(
        r#"
pattern Serve() -> String {
  let omni_model = "nemotron-3-nano-omni-30b-a3b"
  return "ok"
}
flow Main { input: String = "x" -> Serve -> output }
"#,
    )
    .expect_err("the default posture refuses weights-id mentions");
    assert!(
        err.contains("BACKEND_LICENSE_DISTRIBUTION")
            && err.contains("are non-osi and FORBIDDEN in the distribution profile"),
        "the №333 text must stay verbatim, got: {err}"
    );
}

// ── (5) The vision-decl model field is a selection position ──────────

#[test]
fn vision_nonosi_model_without_permit_refuses() {
    let src = r#"
vision "poster" { model: "nemotron-3-nano-omni-30b-a3b" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
flow Main { input: String = "x" -> output }
"#;
    let err = compile(src).expect_err("a NonOsi vision model must refuse without a permit");
    assert!(
        err.contains("LICENSE_CLASS_NOT_ALLOWED"),
        "expected the dedicated code on the vision position, got: {err}"
    );
}

#[test]
fn vision_osi_model_compiles_permit_free() {
    let src = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
flow Main { input: String = "x" -> output }
"#;
    compile(src).unwrap_or_else(|e| panic!("the osi vision model compiles, got: {e}"));
}
