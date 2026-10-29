// ── Naryad №838 (issue #1640, wave 71 — W-A, P1, [core][security][llm]):
// ModelPassport — the type + the 1:1 migration of the registry/manifest.
//
// Contracts (the naryad):
//   (1) RED-negative (а): a passport WITHOUT a mandatory field (digest /
//       class note / verification date) does NOT construct — loud Result
//       error, never a silent default. RED discipline: the validation
//       arms are the fix — on an unvalidating constructor every assert
//       here would fail (Ok instead of Err).
//   (2) RED-negative (б): the detempo-guard — a digest substituted AFTER
//       construction is caught by the integrity check (the «passport
//       digest mismatch» class), same for any record-field tamper.
//   (3) The positive migration 1:1: EVERY registry entry — including
//       TimesFM (the PINNED NEVER posture untouched) — passes through
//       the passport with the SAME license class, the SAME note, and
//       the SAME pin state (artifact digest == the registry pin hash;
//       record digest self-consistent for the pending lane).
//   (4) The arity 1:1 both directions (registry ↔ passports) — the
//       migration is total, no half-migrated table.
//   (5) The weights-manifest link reads the SAME SSOT: every
//       WEIGHTS_SOURCES repo equals its passport's source (the manifest
//       side of the 1:1), and the associated lookup resolves by key.
//
// Boundaries honored: no behavior growth (no compile/audit/load path
// switched to the passport here — №839/№841/№842 arrive later); the VM
// lane untouched; serialization out of the 8D-1 slice.

use metalogos::backends::{BackendClass, LicenseClass, ShaPin, BACKEND_REGISTRY, WEIGHTS_SOURCES};
use metalogos::model_passport::{
    all_passports, passport_for, ModelPassport, NewPassport, PassportError, PassportPinKind,
};

// The vision weights-manifest link is feature-gated (the `vision` module
// is off-by-default); the link test rides the feature the same way the
// vision tests do.
#[cfg(feature = "vision")]
use metalogos::vision::weights::WeightsManifest;

fn valid_fields() -> NewPassport {
    NewPassport {
        identity: "test-model".to_string(),
        digest: "a".repeat(64),
        pin_kind: PassportPinKind::Artifact,
        license: LicenseClass::Osi,
        license_note: "test model — MIT (osi)".to_string(),
        version: "v1".to_string(),
        source: "test/test-model".to_string(),
        verified_at: "2026-10-29".to_string(),
        origin: "test-origin".to_string(),
    }
}

// ── (1) The constructor validation: missing mandatory fields refuse ──

#[test]
fn passport_without_digest_does_not_construct() {
    let mut f = valid_fields();
    f.digest = String::new();
    let err = ModelPassport::new(f).expect_err("an empty digest must refuse construction");
    assert!(
        matches!(err, PassportError::BadDigest(_)),
        "expected the BadDigest class, got: {err:?}"
    );
}

#[test]
fn passport_with_malformed_digest_does_not_construct() {
    // Not hex (the fabricated-hash class), not lowercase, wrong length —
    // all refuse loudly (the ADR-0163 "never a fabricated hash" rule).
    for bad in ["z".repeat(64), "A".repeat(64), "abcd".to_string()] {
        let mut f = valid_fields();
        f.digest = bad;
        let err = ModelPassport::new(f).expect_err("a malformed digest must refuse");
        assert!(
            matches!(err, PassportError::BadDigest(_)),
            "expected BadDigest, got: {err:?}"
        );
    }
}

#[test]
fn passport_without_license_note_does_not_construct() {
    let mut f = valid_fields();
    f.license_note = "  ".to_string();
    let err = ModelPassport::new(f).expect_err("an empty license_note must refuse");
    assert!(
        matches!(err, PassportError::MissingField("license_note")),
        "expected MissingField(license_note), got: {err:?}"
    );
}

#[test]
fn passport_without_verification_date_does_not_construct() {
    let mut f = valid_fields();
    f.verified_at = String::new();
    let err = ModelPassport::new(f).expect_err("an empty verification date must refuse");
    assert!(
        matches!(err, PassportError::MissingField("verified_at")),
        "expected MissingField(verified_at), got: {err:?}"
    );
}

#[test]
fn passport_with_impossible_date_does_not_construct() {
    let mut f = valid_fields();
    f.verified_at = "2026-13-45".to_string();
    let err = ModelPassport::new(f).expect_err("an impossible date must refuse");
    assert!(
        matches!(err, PassportError::BadVerificationDate(_)),
        "expected BadVerificationDate, got: {err:?}"
    );
}

#[test]
fn passport_record_kind_digest_must_be_self_consistent() {
    let mut f = valid_fields();
    f.pin_kind = PassportPinKind::Record;
    // A Record-kind passport whose digest does not equal its own record
    // digest refuses (a foreign hash under the Record kind is exactly
    // the fabricated-hash class); the self-consistent positive rides
    // `every_registry_entry_passes_through_the_passport_unchanged` via
    // the pending lane of the migration.
    let err = ModelPassport::new(f).expect_err("a foreign Record digest must refuse");
    assert!(matches!(err, PassportError::BadDigest(_)), "got: {err:?}");
}

// ── (2) The detempo-guard: post-construction tamper is caught ────────

#[test]
fn substituted_digest_is_caught_by_integrity_check() {
    let passport = ModelPassport::new(valid_fields()).expect("valid fields construct");
    let mut tampered = passport.clone();
    tampered.digest = "b".repeat(64);
    let err = tampered
        .verify_integrity()
        .expect_err("a substituted digest must fail the integrity check");
    assert!(
        matches!(&err, PassportError::Tampered(why) if why.contains("digest was modified")),
        "the message must name the digest tamper, got: {err:?}"
    );
    assert!(
        err.to_string().contains("passport digest mismatch"),
        "the «passport digest mismatch» class must be named, got: {err}"
    );
}

#[test]
fn tampered_record_fields_are_caught_by_integrity_check() {
    let passport = ModelPassport::new(valid_fields()).expect("valid fields construct");
    let mut tampered = passport.clone();
    tampered.license_note = "tampered note — MIT (osi)".to_string();
    let err = tampered
        .verify_integrity()
        .expect_err("a record-field tamper must fail the integrity check");
    assert!(
        matches!(err, PassportError::Tampered(_)),
        "expected Tampered, got: {err:?}"
    );
    assert!(
        err.to_string().contains("passport digest mismatch"),
        "the class text must be present, got: {err}"
    );
}

// ── (3) The positive migration: every entry passes through 1:1 ───────

#[test]
fn every_registry_entry_passes_through_the_passport_unchanged() {
    assert_eq!(
        BACKEND_REGISTRY.len(),
        all_passports().len(),
        "the passport table must mirror the registry 1:1"
    );
    for entry in BACKEND_REGISTRY {
        let p = passport_for(entry.weights_id)
            .unwrap_or_else(|| panic!("no passport for {}", entry.weights_id));
        assert_eq!(p.identity, entry.weights_id);
        assert_eq!(
            p.license, entry.license,
            "{}: the class must be unchanged",
            entry.name
        );
        assert_eq!(
            p.license_note, entry.license_note,
            "{}: the note must be unchanged",
            entry.name
        );
        match entry.pin {
            ShaPin::Pinned(h) => {
                assert_eq!(
                    p.pin_kind,
                    PassportPinKind::Artifact,
                    "{}: a pinned entry must carry the artifact kind",
                    entry.name
                );
                assert_eq!(
                    p.digest, h,
                    "{}: the artifact digest must equal the registry pin",
                    entry.name
                );
            }
            ShaPin::PendingNo334 => {
                assert_eq!(
                    p.pin_kind,
                    PassportPinKind::Record,
                    "{}: a pending entry must carry the record kind",
                    entry.name
                );
                assert_eq!(
                    p.digest,
                    p.record_digest(),
                    "{}: the record digest must be self-consistent",
                    entry.name
                );
            }
        }
        assert!(
            p.verify_integrity().is_ok(),
            "{}: the migrated passport must pass its own integrity check",
            entry.name
        );
    }
}

#[test]
fn timesfm_passes_through_with_the_same_class_and_pin_state() {
    // The positive control the naryad names explicitly: the TimesFM
    // record rides the passport with its class and its PINNED NEVER
    // posture untouched (the 3.0 weights stay restrictive-by-never in
    // the note text, which is preserved verbatim).
    let entry = BACKEND_REGISTRY
        .iter()
        .find(|e| e.class == BackendClass::Timeseries)
        .expect("the timeseries rung exists");
    let p = passport_for(entry.weights_id).expect("the timeseries passport exists");
    assert_eq!(p.license, LicenseClass::Osi);
    assert!(
        p.license_note.contains("PINNED NEVER"),
        "the NEVER posture text must survive the migration verbatim: {}",
        p.license_note
    );
    assert_eq!(entry.pin, ShaPin::PendingNo334);
    assert_eq!(p.pin_kind, PassportPinKind::Record);
}

#[test]
fn nonosi_and_restrictive_records_keep_their_classes() {
    // The MDL-3 test case (NonOsi) and the default-deny record
    // (Restrictive) must survive the migration unchanged — the №839 rule
    // will key on exactly these classes.
    let has = |l: LicenseClass| all_passports().iter().any(|p| p.license == l);
    assert!(has(LicenseClass::NonOsi));
    assert!(has(LicenseClass::Restrictive));
    let nemotron = all_passports()
        .iter()
        .find(|p| p.license == LicenseClass::NonOsi)
        .expect("the nemotron passport exists");
    assert!(
        nemotron.license_note.contains("NVIDIA Open Model License"),
        "the classification basis must survive: {}",
        nemotron.license_note
    );
}

// ── (4) The arity 1:1, both directions ───────────────────────────────

#[test]
fn passport_table_is_one_to_one_with_the_registry() {
    let registry_ids: Vec<&str> = BACKEND_REGISTRY.iter().map(|e| e.weights_id).collect();
    let passport_ids: Vec<&str> = all_passports()
        .iter()
        .map(|p| p.identity.as_str())
        .collect();
    for id in &registry_ids {
        assert!(
            passport_ids.contains(id),
            "registry id {id} has no passport"
        );
    }
    for id in &passport_ids {
        assert!(
            registry_ids.contains(id),
            "passport {id} has no registry row"
        );
    }
    assert_eq!(registry_ids.len(), passport_ids.len());
}

// ── (5) The weights-manifest link reads the same SSOT ────────────────

#[test]
fn every_weights_source_repo_equals_its_passport_source() {
    for (id, source) in WEIGHTS_SOURCES {
        let p = passport_for(id).unwrap_or_else(|| panic!("no passport for the manifest {id}"));
        assert_eq!(
            p.source, source.repo,
            "the manifest side of the 1:1 drifted for {id}"
        );
        assert_eq!(p.pin_kind, PassportPinKind::Artifact);
    }
}

#[cfg(feature = "vision")]
#[test]
fn manifest_passport_link_resolves_by_key() {
    let p = WeightsManifest::passport_for_model("trocr-base-printed")
        .expect("the trocr passport resolves through the manifest link");
    assert_eq!(p.source, "microsoft/trocr-base-printed");
    assert_eq!(p.license, LicenseClass::Osi);
    assert!(WeightsManifest::passport_for_model("no-such-model").is_none());
}

#[test]
fn all_passports_carry_valid_verification_dates() {
    for p in all_passports() {
        let bytes = p.verified_at.as_bytes();
        assert_eq!(bytes.len(), 10, "{}: the date shape", p.identity);
        assert_eq!(&p.verified_at[4..5], "-", "{}: the date shape", p.identity);
        assert_eq!(&p.verified_at[7..8], "-", "{}: the date shape", p.identity);
        let month: u32 = p.verified_at[5..7].parse().expect("month digits");
        let day: u32 = p.verified_at[8..10].parse().expect("day digits");
        assert!((1..=12).contains(&month), "{}: the month range", p.identity);
        assert!((1..=31).contains(&day), "{}: the day range", p.identity);
    }
}
