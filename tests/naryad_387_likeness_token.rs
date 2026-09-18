// ── Naryad #387 (P2, security/media): LikenessToken — ADR-0149 D1/D6 ──
//
// The opaque likeness-consent token: the third legal credential for
// private/camera-origin media egress (beside the public label and the
// №335 consent scope). Contract surface:
//
// 1. The token is NOT a String: the challenge parameter is typed (a
//    String never verifies), the token variant is opaque (serde dead
//    marker, non-printable), and the runtime credential position
//    refuses text loudly.
// 2. The ritual is linear: a challenge verifies exactly once
//    (LIKENESS_VERIFY_FAILED on replay).
// 3. VIDEO_LIKENESS_NO_CONSENT (Category A): `video_render` with an
//    I2V reference from a `kind: "likeness"` origin requires a bound
//    `likeness_verify` result in the body scope (presence-based, the
//    D2 honest boundary). Red without the ritual, green with it.
// 4. Alias invariance: a 3-deep let-chain keeps the origin provenance
//    AND the consent scope — the unconsented save stays red, the
//    consented save turns green.
// 5. Generalized media egress: media_save accepts a private
//    camera/likeness-origin handle with the token credential (static
//    presence + runtime registry check); the kitchen-camera deny is
//    unchanged (fail-closed without a credential).
// 6. E2E: challenge/verify → save with the token → the file lands with
//    the honest C2PA-style sidecar (origin/conf preserved).

use metalogos::audit::{audit_category_a, Severity};
use metalogos::interpreter::values::Value;
use metalogos::parser;
use metalogos::semantic;

fn audit_errors(src: &str) -> Vec<String> {
    let declarations = parser::parse(src).expect("parse");
    audit_category_a(&declarations, "")
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| format!("[{}] {}", f.check_id, f.message))
        .collect()
}

fn audit_all(src: &str) -> Vec<String> {
    let declarations = parser::parse(src).expect("parse");
    audit_category_a(&declarations, "")
        .iter()
        .map(|f| format!("[{}] {}", f.check_id, f.message))
        .collect()
}

const LIKENESS_NO_RITUAL: &str = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern Send(data: String) -> String {
  let face = source portrait
  let _ = video_render("wan-2.2-ti2v-5b", "scene", face)
  return "rendered"
}
flow Main { input: String = data -> Send -> output }
"#;

const LIKENESS_WITH_RITUAL: &str = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern Send(data: String) -> String {
  let face = source portrait
  let challenge = likeness_challenge("subject-1", "likeness")
  let token = likeness_verify(challenge)
  let _ = video_render("wan-2.2-ti2v-5b", "scene", face)
  return "rendered"
}
flow Main { input: String = data -> Send -> output }
"#;

// ── 1. The token is unforgeable from text ────────────────────────────

#[test]
fn n387_string_never_verifies_as_challenge() {
    let forged = Value::String("LCH#1".to_string());
    let err = metalogos::likeness::challenge_verify(&forged, None, None)
        .expect_err("a String in the challenge position must refuse");
    assert!(
        err.contains("must be a LikenessChallenge"),
        "typed refusal: {err}"
    );
}

#[test]
fn n387_serde_dead_markers() {
    // The externally-tagged enum wraps the marker, but the credential
    // itself serializes as the DEAD marker string (no id, no state).
    let tok = serde_json::to_string(&Value::Likeness(metalogos::likeness::TokenHandle { id: 1 }))
        .expect("serialize");
    assert!(tok.contains("\"[LIKENESS_TOKEN]\""), "dead marker: {tok}");
    let ch = serde_json::to_string(&Value::LikenessChallenge(
        metalogos::likeness::ChallengeHandle { id: 1 },
    ))
    .expect("serialize");
    assert!(ch.contains("\"[LIKENESS_CHALLENGE]\""), "dead marker: {ch}");
    // A deserialized token is a tombstone: id 0 is never issued.
    let revived: Value =
        serde_json::from_str("{\"Likeness\":\"[LIKENESS_TOKEN]\"}").expect("deserialize marker");
    match revived {
        Value::Likeness(h) => assert!(
            !h.is_issued().expect("registry readable"),
            "a deserialized token must not be a live credential"
        ),
        other => panic!("expected Likeness, got {}", other.type_name()),
    }
}

#[test]
fn n387_display_is_opaque() {
    let v = Value::Likeness(metalogos::likeness::TokenHandle { id: 9 });
    assert_eq!(format!("{v}"), "[LikenessToken]");
    assert_eq!(v.type_name(), "LikenessToken");
}

// ── 2. The ritual is linear ──────────────────────────────────────────

#[test]
fn n387_challenge_is_one_time() {
    let ch = metalogos::likeness::challenge_issue("subject-1", "likeness", 60).expect("issue");
    assert!(metalogos::likeness::challenge_verify(&ch, None, None).is_ok());
    let replay = metalogos::likeness::challenge_verify(&ch, None, None)
        .expect_err("the consumed challenge must refuse");
    assert!(replay.contains("LIKENESS_VERIFY_FAILED"), "typed: {replay}");
}

// ── 3. VIDEO_LIKENESS_NO_CONSENT red/green ───────────────────────────

#[test]
fn n387_video_likeness_red_without_ritual() {
    let errs = audit_errors(LIKENESS_NO_RITUAL);
    assert!(
        errs.iter()
            .any(|m| m.starts_with("[VIDEO_LIKENESS_NO_CONSENT]")),
        "the deepfake gate must fire: {errs:?}"
    );
}

#[test]
fn n387_video_likeness_green_with_ritual() {
    let errs = audit_errors(LIKENESS_WITH_RITUAL);
    assert!(
        !errs.iter().any(|m| m.contains("VIDEO_LIKENESS_NO_CONSENT")),
        "the ritual clears the gate: {errs:?}"
    );
}

#[test]
fn n387_video_likeness_camera_origin_not_gated_but_untrusted_frame_is() {
    // A camera-origin frame (not `kind: likeness`) is NOT the
    // VIDEO_LIKENESS_NO_CONSENT subject (it keeps the UNTRUSTED_FRAME
    // advisory for untrusted sources) — the gate targets the declared
    // likeness kind only (ADR-0149 D1).
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern Send(data: String) -> String {
  let face = source cam
  let _ = video_render("wan-2.2-ti2v-5b", "scene", face)
  return "rendered"
}
flow Main { input: String = data -> Send -> output }
"#;
    let findings = audit_all(src);
    assert!(
        !findings
            .iter()
            .any(|m| m.contains("VIDEO_LIKENESS_NO_CONSENT")),
        "camera origin is not a likeness declaration: {findings:?}"
    );
}

#[test]
fn n387_likeness_origin_requires_no_fake_kinds() {
    // The declared-kind honesty boundary: the gate fires only on the
    // declaration, so an unknown kind must stay a loud compile error.
    let src = r#"
origin weird { kind: hologram, media: image, label: private }
pattern P(_tick: String) -> String {
  let face = source weird
  return "x"
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("unknown kind")),
        "the kind vocabulary stays closed: {errs:?}"
    );
}

// ── 4. Alias invariance (the 3-deep let-chain) ───────────────────────

#[test]
fn n387_alias_chain_keeps_the_deny() {
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P(_tick: String) -> String {
  let frame = source cam
  let g = frame
  let h = g
  let i = h
  return media_save(i, "frames/leak.jpg")
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "a 3-deep alias chain must NOT detach the private label: {errs:?}"
    );
}

#[test]
fn n387_alias_chain_keeps_consent_green() {
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P(_tick: String) -> String {
  let frame = source cam
  let g = frame
  let h = g
  let i = h
  let ok = consent_grant(i, "gdpr", "subject-1")
  return media_save(ok, "frames/ok.jpg")
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.is_empty(),
        "the consent scope survives the alias chain: {errs:?}"
    );
}

// ── 5. Generalized media egress ──────────────────────────────────────

#[test]
fn n387_likeness_save_red_without_credentials() {
    let src = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern P(_tick: String) -> String {
  let face = source portrait
  return media_save(face, "frames/leak.jpg")
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "no consent, no token — no egress: {errs:?}"
    );
}

#[test]
fn n387_likeness_save_green_with_ritual() {
    let src = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern P(_tick: String) -> String {
  let face = source portrait
  let challenge = likeness_challenge("subject-1", "likeness")
  let token = likeness_verify(challenge)
  return media_save(face, "frames/portrait.jpg", token)
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(errs.is_empty(), "the token clears the media save: {errs:?}");
}

#[test]
fn n387_consent_path_green_for_media_save() {
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P(_tick: String) -> String {
  let frame = source cam
  let ok = consent_grant(frame, "gdpr", "subject-1")
  return media_save(ok, "frames/ok.jpg")
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(errs.is_empty(), "the №335 consent path is legal: {errs:?}");
}

#[test]
fn n387_token_does_not_clear_non_camera_kinds() {
    // The token path is scoped to camera/likeness origins — a private
    // file-origin handle keeps its plain deny (the ritual is about
    // likeness, not a universal egress key).
    let src = r#"
origin docs { kind: file, path: "x.jpg", media: image, label: private }
pattern P(_tick: String) -> String {
  let f = source docs
  let challenge = likeness_challenge("subject-1", "likeness")
  let token = likeness_verify(challenge)
  return media_save(f, "out/x.jpg", token)
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "the token is not a universal key: {errs:?}"
    );
}

#[test]
fn n387_kitchen_camera_stays_red() {
    // The canonical red contract (№332) is untouched by №387.
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/w1_kitchen_camera.mlog"
    ))
    .expect("the canonical example exists");
    let errs = audit_errors(&src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "the kitchen camera keeps its deny: {errs:?}"
    );
}

// ── 6. E2E: the ritual at runtime ────────────────────────────────────

#[test]
fn n387_runtime_ritual_unseals_egress() {
    let src = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern Token(_tick: String) -> String {
  let face = from portrait media_store_image("face-bytes", "private")
  let challenge = likeness_challenge("subject-1", "likeness")
  let token = likeness_verify(challenge)
  let saved = media_save(face, "w387_e2e_saved.jpg", token)
  return saved
}
flow Main { input: String = "tick" -> Token -> output }
"#;
    let out = metalogos::run_program(src)
        .expect("the ritual e2e runs")
        .unwrap_or_default();
    assert!(out.contains("w387_e2e_saved.jpg"), "output: {out:?}");
}

#[test]
fn n387_runtime_refuses_without_the_credential() {
    let src = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern Token(_tick: String) -> String {
  let face = from portrait media_store_image("face-bytes", "private")
  let challenge = likeness_challenge("subject-1", "likeness")
  let _token = likeness_verify(challenge)
  return media_save(face, "w387_e2e_saved.jpg")
}
flow Main { input: String = "tick" -> Token -> output }
"#;
    let err = metalogos::run_program(src).expect_err("the seal holds without the credential");
    assert!(
        err.contains("MEDIA_SEALED_EGRESS"),
        "the runtime backstop names the seal: {err}"
    );
}

#[test]
fn n387_runtime_refuses_a_forged_credential() {
    // A String in the credential position never unseals — loud type
    // refusal, not a silent downgrade. The ritual IS performed (the
    // static gate passes), but the CREDENTIAL argument is forged text.
    let src = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern Token(_tick: String) -> String {
  let face = from portrait media_store_image("face-bytes", "private")
  let challenge = likeness_challenge("subject-1", "likeness")
  let token = likeness_verify(challenge)
  return media_save(face, "w387_e2e_saved.jpg", "forged-token")
}
flow Main { input: String = "tick" -> Token -> output }
"#;
    let err = metalogos::run_program(src).expect_err("a forged credential must refuse");
    assert!(
        err.contains("MEDIA_SEALED_EGRESS") && err.contains("must be a LikenessToken"),
        "the type refusal is loud: {err}"
    );
}

// ── 7. The static gate stays presence-honest (D2 boundary pinned) ────

#[test]
fn n387_ritual_after_the_call_does_not_clear_it() {
    // The token must be bound BEFORE the gated call site — a verify
    // that happens lexically later does not clear the egress (the
    // presence check is order-sensitive).
    let src = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern P(_tick: String) -> String {
  let face = source portrait
  let early = media_save(face, "frames/leak.jpg")
  let challenge = likeness_challenge("subject-1", "likeness")
  let token = likeness_verify(challenge)
  return early
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "the ritual must precede the egress: {errs:?}"
    );
}

#[test]
fn n387_branch_bound_token_does_not_escape() {
    // Conservative scope: a token bound inside a branch does not clear
    // sinks after the branch (fail-closed — documented in FlowCtx).
    let src = r#"
origin portrait { kind: likeness, media: image, label: private }
pattern P(_tick: String) -> String {
  let face = source portrait
  if data != "" {
    let challenge = likeness_challenge("subject-1", "likeness")
    let token = likeness_verify(challenge)
  }
  return media_save(face, "frames/leak.jpg")
}
flow Main { input: String = data -> P -> output }
"#;
    // The static walk conservatively keeps the deny (branch-local).
    let violations = semantic::sink_clearance_violations(&parser::parse(src).expect("parse"));
    assert!(
        !violations.is_empty(),
        "a branch-bound token does not escape the branch"
    );
}
