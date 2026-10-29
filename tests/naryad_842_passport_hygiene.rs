// ── Naryad №842 (issue #1644, wave 71 — W-A, P1, [security][core]):
// the passport hygiene — freshness/revocation, the fail-closed
// «passport digest mismatch», the re-pin by explicit update only
// (the G5 pattern of №828 brought to the passport).
//
// Contracts (the naryad):
//   (1) the negative (а): a tampered digest → the consumption REFUSES
//       (not a warning) with the «passport digest mismatch» class named;
//   (2) the negative (б): a revoked passport → the consumption refuses;
//       the un-revocation rides ONLY the explicit re-pin transaction;
//   (3) the positive: the fresh, unmodified passport consumes штатно —
//       the whole in-tree table consumes clean against today;
//   (4) the re-pin: the new digest + the new verification date land in
//       ONE transaction (all-or-nothing; a bad digest or a bad date
//       leaves the passport untouched); the re-pin re-anchors the
//       integrity and clears the revocation;
//   (5) the freshness rule: the age within PASSPORT_FRESHNESS_DAYS is
//       fresh; the boundary is exact; the wave-71 staleness mode is
//       Flag (the flag text renders, the verdict stays Ok);
//   (6) the compile path: the selection sites carry the consumption
//       check (the in-tree passports ARE the pinned table — the check
//       holds by construction; the refusal class would surface as
//       [PASSPORT_REFUSED]).
//
// Boundaries honored: the №334 runtime gates (SSRF-guard/SHA-pin) not
// duplicated — the hygiene is the passport-level trust decision BEFORE
// loading; no auto-refresh, no automatic re-validation — every state
// change is an explicit transaction.

use metalogos::backends::{LicenseClass, BACKEND_REGISTRY};
use metalogos::model_passport::{
    all_passports, passport_for, today_utc, ModelPassport, NewPassport, PassportError,
    PassportPinKind, PassportRefusal, StalenessMode, PASSPORT_FRESHNESS_DAYS,
    PASSPORT_STALENESS_MODE,
};

fn fresh_fields() -> NewPassport {
    fields_verified_at(today_utc())
}

/// The fields builder with an explicit verification date — the
/// staleness tests construct through the VALIDATING constructor (the
/// private anchors never need poking).
fn fields_verified_at(verified_at: String) -> NewPassport {
    NewPassport {
        identity: "hygiene-model".to_string(),
        digest: "c".repeat(64),
        pin_kind: PassportPinKind::Artifact,
        license: LicenseClass::Osi,
        license_note: "hygiene test model — MIT (osi)".to_string(),
        version: "v1".to_string(),
        source: "test/hygiene-model".to_string(),
        verified_at,
        origin: "test-origin".to_string(),
    }
}

// ── (1) The negative (а): the tampered digest refuses ────────────────

#[test]
fn substituted_digest_refuses_consumption() {
    let passport = ModelPassport::new(fresh_fields()).expect("constructs");
    let pinned_digest = passport.digest.clone();
    let mut tampered = passport.clone();
    tampered.digest = "d".repeat(64);
    let err = tampered
        .consume(&pinned_digest, &today_utc())
        .expect_err("the substituted digest must REFUSE, not warn");
    assert!(
        matches!(&err, PassportRefusal::DigestMismatch { actual, .. } if actual == &"d".repeat(64)),
        "expected the DigestMismatch class, got: {err:?}"
    );
    let _ = pinned_digest;
    assert!(
        err.to_string().contains("passport digest mismatch"),
        "the class text must be named explicitly, got: {err}"
    );
}

#[test]
fn tampered_record_fields_refuse_consumption() {
    // The detempo-guard rides the consumption: any post-construction
    // mutation of the record fields fails the integrity check → the
    // refusal (the «passport digest mismatch» family text).
    let passport = ModelPassport::new(fresh_fields()).expect("constructs");
    let pinned_digest = passport.digest.clone();
    let mut tampered = passport.clone();
    tampered.license_note = "tampered — MIT (osi)".to_string();
    let err = tampered
        .consume(&pinned_digest, &today_utc())
        .expect_err("the record tamper must refuse");
    assert!(
        matches!(err, PassportRefusal::Tampered(_)),
        "expected Tampered, got: {err:?}"
    );
    assert!(
        err.to_string().contains("passport digest mismatch"),
        "got: {err}"
    );
}

// ── (2) The negative (б): the revocation refuses ─────────────────────

#[test]
fn revoked_passport_refuses_consumption() {
    let mut passport = ModelPassport::new(fresh_fields()).expect("constructs");
    let pinned_digest = passport.digest.clone();
    passport.revoke("the license record landed — the model is pulled".to_string());
    let err = passport
        .consume(&pinned_digest, &today_utc())
        .expect_err("the revoked passport must refuse");
    assert!(
        matches!(err, PassportRefusal::Revoked(_)),
        "expected Revoked, got: {err:?}"
    );
    assert!(err.to_string().contains("passport revoked"), "got: {err}");
}

#[test]
fn un_revocation_rides_only_the_repin_transaction() {
    // No API clears the revocation except the explicit re-pin — the
    // struct field is public, but the CONTRACT is the transaction: the
    // test pins that re_pin is the only sanctioned path and it clears.
    let mut passport = ModelPassport::new(fresh_fields()).expect("constructs");
    let pinned_digest = passport.digest.clone();
    passport.revoke("pulled".to_string());
    assert!(passport.consume(&pinned_digest, &today_utc()).is_err());
    passport
        .re_pin(pinned_digest.clone(), today_utc())
        .expect("the explicit re-pin transaction succeeds");
    assert!(
        !passport.revoked,
        "the re-pin cleared the revocation (the explicit un-revocation)"
    );
    passport
        .consume(&pinned_digest, &today_utc())
        .expect("the passport consumes again after the explicit re-pin");
}

// ── (3) The positive: the fresh passport consumes штатно ─────────────

#[test]
fn fresh_unmodified_passport_consumes() {
    let passport = ModelPassport::new(fresh_fields()).expect("constructs");
    let pinned_digest = passport.digest.clone();
    passport
        .consume(&pinned_digest, &today_utc())
        .expect("the fresh unmodified passport consumes cleanly");
}

#[test]
fn the_whole_in_tree_table_consumes_clean() {
    // The in-tree passports ARE the pinned context (the digest compares
    // against itself) — the compile-path check holds by construction;
    // the test pins the invariant over every row.
    let today = today_utc();
    assert_eq!(today.len(), 10, "the UTC date shape");
    for entry in BACKEND_REGISTRY {
        let p = passport_for(entry.weights_id)
            .unwrap_or_else(|| panic!("no passport for {}", entry.weights_id));
        p.consume(p.digest.as_str(), &today)
            .unwrap_or_else(|e| panic!("{} must consume clean, got: {e}", entry.weights_id));
    }
}

// ── (4) The re-pin transaction: one act, all-or-nothing ──────────────

#[test]
fn repin_lands_digest_and_date_in_one_transaction() {
    let mut passport = ModelPassport::new(fresh_fields()).expect("constructs");
    let old_digest = passport.digest.clone();
    let new_digest = "e".repeat(64);
    passport
        .re_pin(new_digest.clone(), "2026-11-15".to_string())
        .expect("the re-pin succeeds");
    assert_eq!(passport.digest, new_digest);
    assert_eq!(passport.verified_at, "2026-11-15");
    // The pin context moved WITH the explicit update: the new digest
    // verifies, the OLD one now mismatches (the pin only moves by the
    // explicit act — the №828 posture).
    passport
        .consume(&new_digest, &today_utc())
        .expect("the new pin verifies");
    let err = passport
        .consume(&old_digest, &today_utc())
        .expect_err("the old pin must mismatch after the re-pin");
    assert!(matches!(err, PassportRefusal::DigestMismatch { .. }));
    // The integrity re-anchored: the re-pinned passport verifies.
    assert!(passport.verify_integrity().is_ok());
}

#[test]
fn repin_is_all_or_nothing() {
    let mut passport = ModelPassport::new(fresh_fields()).expect("constructs");
    let old_digest = passport.digest.clone();
    let old_date = passport.verified_at.clone();
    // A bad digest → the whole transaction refuses.
    let err = passport
        .re_pin("NOT-A-HASH".to_string(), "2026-11-15".to_string())
        .expect_err("a bad re-pin digest must refuse");
    assert!(matches!(err, PassportError::BadDigest(_)));
    assert_eq!(passport.digest, old_digest, "the digest untouched");
    assert_eq!(passport.verified_at, old_date, "the date untouched");
    // A bad date → the whole transaction refuses.
    let err = passport
        .re_pin("f".repeat(64), "2026-13-99".to_string())
        .expect_err("a bad re-pin date must refuse");
    assert!(matches!(err, PassportError::BadVerificationDate(_)));
    assert_eq!(passport.digest, old_digest, "still untouched");
    assert_eq!(passport.verified_at, old_date, "still untouched");
}

// ── (5) The freshness rule and the wave-71 mode ──────────────────────

#[test]
fn the_freshness_boundary_is_exact() {
    // The boundary: exactly the window = fresh; one day more = stale.
    // Constructed through the validating constructor with the fixed
    // date — the anchors hold by construction.
    let passport =
        ModelPassport::new(fields_verified_at("2026-10-29".to_string())).expect("constructs");
    // The arithmetic helper pins the boundary without depending on the
    // real clock: age_days against a fixed reference.
    assert_eq!(
        passport.age_days("2026-10-29"),
        Some(0),
        "the same day is age 0"
    );
    assert_eq!(
        passport.age_days("2027-04-27"),
        Some(PASSPORT_FRESHNESS_DAYS),
        "the window edge is inclusive"
    );
    assert!(passport.is_fresh("2027-04-27"));
    assert!(
        !passport.is_fresh("2027-04-28"),
        "one day past the window is stale"
    );
}

#[test]
fn the_wave71_staleness_mode_is_flag() {
    // The mode constant: the flag-only posture (the in-tree dates must
    // not hard-refuse on day one; the escalation is the owner's act).
    assert_eq!(PASSPORT_STALENESS_MODE, StalenessMode::Flag);
    // The stale passport CONSUMES under the Flag mode (the verdict),
    // while the refusal class renders the flag text when constructed.
    let passport =
        ModelPassport::new(fields_verified_at("2020-01-01".to_string())).expect("constructs");
    let pinned_digest = passport.digest.clone();
    passport
        .consume(&pinned_digest, &today_utc())
        .expect("the Flag mode does not refuse the stale passport");
    let stale = PassportRefusal::Stale {
        verified_at: "2020-01-01".to_string(),
        window_days: PASSPORT_FRESHNESS_DAYS,
    };
    assert!(
        stale.to_string().contains("passport stale"),
        "the flag text renders: {stale}"
    );
}

// ── (6) The compile path carries the consumption check ───────────────

#[test]
fn the_selection_sites_ride_the_hygiene_check() {
    // The №839 rule calls the consumption check on every selection site;
    // the in-tree table holds by construction → the programs compile
    // (the Osi ladder permit-free; the NonOsi ladder with the permit).
    let osi = r#"
pattern Pick(_tick: String) -> String {
  let sel = backend_select("tts", ["chatterbox"])
  if sel.ok {
    return "selected"
  }
  return "degraded"
}
flow Main { input: String = "x" -> Pick -> output }
"#;
    metalogos::compile_program(osi.trim())
        .unwrap_or_else(|e| panic!("the hygiene check must not break the clean path, got: {e}"));
    let nonosi = r#"
profile licensing { allow_license_class: "non-osi" }

pattern Pick(_tick: String) -> String {
  let sel = backend_select("omni", ["nemotron-omni"])
  if sel.ok {
    return "selected"
  }
  return "degraded"
}
flow Main { input: String = "x" -> Pick -> output }
"#;
    metalogos::compile_program(nonosi.trim())
        .unwrap_or_else(|e| panic!("the permitted ladder compiles, got: {e}"));
}

#[test]
fn the_table_stays_intact_after_the_hygiene_landing() {
    // The №838 contracts hold: the arity 1:1 and the integrity over the
    // whole table (the hygiene fields default honestly).
    assert_eq!(all_passports().len(), BACKEND_REGISTRY.len());
    for p in all_passports() {
        assert!(!p.revoked, "{}: nothing is revoked in-tree", p.identity);
        assert!(
            p.revocation_note.is_empty(),
            "{}: the note is empty in-tree",
            p.identity
        );
        assert!(p.verify_integrity().is_ok());
    }
}
