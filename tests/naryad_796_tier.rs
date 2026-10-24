// ── Naryad №796 (issue #1534, Wave 64 P1 — the C3 (a) decision) ──────
//
// The maturity tier SSOT: every `spec!` registration carries an
// EXPLICIT tier ("stable" | "experimental" | "parked" — the owner's
// rule table, the C3 decision 23.10). The gate is the GRAMMAR itself
// (a spec! without the trailing tier literal is a compile error — the
// fallthrough arm in the macro), and this battery pins the vocabulary,
// the coverage and the owner-rule spot checks on the compiled registry.
//
// RED-before: new-surface pins (the field did not exist before №796 —
// the pre-№796 tree does not compile this battery; the grammar gate is
// the compile-time RED for any future tier-less row).

use metalogos::builtins::BUILTIN_REGISTRY;

const VOCABULARY: [&str; 3] = ["stable", "experimental", "parked"];

/// The coverage pin: EVERY compiled row carries a tier inside the
/// vocabulary. (The source-level 526 count rides gen_reference --check —
/// the file reader sees all 526 rows including the 69 feature-gated ones;
/// the compiled default registry carries 509 — the feature rows compile
/// in their own CI jobs.)
#[test]
fn n796_every_registered_builtin_carries_explicit_tier() {
    assert!(
        !BUILTIN_REGISTRY.is_empty(),
        "the registry must not be empty"
    );
    for spec in BUILTIN_REGISTRY {
        assert!(
            VOCABULARY.contains(&spec.tier),
            "`{}` carries tier `{}` outside the vocabulary {VOCABULARY:?} (№796)",
            spec.name,
            spec.tier
        );
    }
}

/// The owner-rule spot checks (the C3 (a) table):
/// - the working core (len) is `stable`;
/// - the ADR-0178 generative contour (reflex_generate, vision_generate,
///   tts_speak, video_render) is `parked`;
/// - the embodied/forecast young APIs (device_open, pose_make,
///   forecast_next) are `experimental`.
#[test]
fn n796_owner_rule_spot_checks() {
    // IF-PRESENT semantics: the feature-gated rows compile in their own CI
    // jobs (the voice/vision/video features) — here every PRESENT name must
    // match the owner's C3 (a) rule; the default build pins the core.
    let tier_of = |name: &str| -> Option<&'static str> {
        BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.tier)
    };
    // stable — the working core (always compiled)
    for n in [
        "len",
        "print",
        "query",
        "respond",
        "secret",
        "mcp_call",
        "grant_issue",
        "ledger_verify",
        "memory_read",
    ] {
        assert_eq!(
            tier_of(n),
            Some("stable"),
            "`{n}` must be registered and stable"
        );
    }
    // parked — the ADR-0178 generative contour
    for n in [
        "reflex_generate",
        "vision_generate",
        "vision_edit",
        "vision_lora_generate",
        "voice_design",
        "tts_speak",
        "voice_enroll",
        "video_render",
    ] {
        if let Some(tier) = tier_of(n) {
            assert_eq!(tier, "parked", "`{n}` must be parked when compiled");
        }
    }
    // experimental — the embodied/forecast young APIs
    for n in [
        "device_open",
        "pose_make",
        "proof_seal",
        "series_make",
        "forecast_next",
    ] {
        if let Some(tier) = tier_of(n) {
            assert_eq!(
                tier, "experimental",
                "`{n}` must be experimental when compiled"
            );
        }
    }
}

/// The tier census (the report's table basis — printed, not asserted
/// beyond the vocabulary; the owner re-tiers at the review).
#[test]
fn n796_tier_census() {
    let mut counts = std::collections::BTreeMap::new();
    for spec in BUILTIN_REGISTRY {
        *counts.entry(spec.tier).or_insert(0usize) += 1;
    }
    println!("№796 tier census (the compiled default build): {counts:?}");
    assert_eq!(
        counts.values().sum::<usize>(),
        BUILTIN_REGISTRY.len(),
        "the census covers every compiled row"
    );
}
