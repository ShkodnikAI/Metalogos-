// ── Naryad №799 (issue #1546, wave 65, P1 — the C3 (a) T2 decision) ────
//
// The parked-tier registry rows (№796's classification) are the opt-in
// build surface: the row and its handler compile ONLY under the owning
// feature (`#[cfg(feature = ...)]` in registry.rs), so the DEFAULT build
// neither registers nor compiles them. The name, however, stays KNOWN to
// the language: this table is the always-compiled SSOT of the parked
// surface — the check lane reads it to refuse a parked call with
//
//     [BUILTIN_PARKED] <name> requires --features <f> (parked: <basis>)
//
// instead of the lie `undefined` (the name IS defined — it is parked),
// and the compiler/runtime backstops read it to stamp the same contract
// at their origins (the №479 coded-error form).
//
// The anti-drift lock: `tests/naryad_799_parked_parity.rs` scans the
// registry SOURCE (the same contract `gen_reference.py` and
// `type_signature_share.py` use — the feature-gated rows ride the source
// metric only) and pins set-equality: every tier-"parked" spec! row has
// an entry here, and this table names nothing else. A parked row added
// without its parked-table entry fails the lock; a parked-table entry
// without a row is drift the same way.
//
// The tier stays owned by the registry rows (№796); this table carries
// the FEATURE mapping and the parking BASIS (the honest "why"), which
// the row's tier alone cannot express.

/// One parked builtin: the name, the feature that unlocks the row
/// (`None` — parked unconditionally: the row compiles, the handler never
/// answers Ok in this environment), and the parking basis.
pub struct ParkedSpec {
    pub name: &'static str,
    pub feature: Option<&'static str>,
    pub basis: &'static str,
}

/// The parked surface. 21 entries — the exact set of tier-"parked" rows
/// in `BUILTIN_REGISTRY` (the parity lock above pins the equality).
pub const PARKED_REGISTRY: &[ParkedSpec] = &[
    // ── Reflex (the sequence layer; the candle build) ──
    ParkedSpec {
        name: "reflex_generate",
        feature: Some("candle"),
        basis: "the live Ok arm is candle-gated (No. 748)",
    },
    ParkedSpec {
        name: "reflex_bpe_load",
        feature: None,
        basis: "the handler never returns Ok in this environment (No. 717, No. 195)",
    },
    // ── Vision (the opt-in `vision` build; No. 210, ADR-0124) ──
    ParkedSpec {
        name: "vision_generate",
        feature: Some("vision"),
        basis: "the vision pillar is the opt-in `vision` build (No. 210, ADR-0124)",
    },
    ParkedSpec {
        name: "vision_edit",
        feature: Some("vision"),
        basis: "the vision pillar is the opt-in `vision` build (No. 210, ADR-0124)",
    },
    ParkedSpec {
        name: "vision_export",
        feature: Some("vision"),
        basis: "the vision pillar is the opt-in `vision` build (No. 210, ADR-0124)",
    },
    ParkedSpec {
        name: "vision_export_raw",
        feature: Some("vision"),
        basis: "the vision pillar is the opt-in `vision` build (No. 210, ADR-0124)",
    },
    ParkedSpec {
        name: "vision_fetch_weights",
        feature: Some("vision"),
        basis: "the vision pillar is the opt-in `vision` build (No. 210, ADR-0124)",
    },
    ParkedSpec {
        name: "vision_lora_load",
        feature: Some("vision"),
        basis: "the vision pillar is the opt-in `vision` build (No. 210, ADR-0124)",
    },
    ParkedSpec {
        name: "vision_lora_generate",
        feature: Some("vision"),
        basis: "the vision pillar is the opt-in `vision` build (No. 210, ADR-0124)",
    },
    // ── Voice (the opt-in `voice` build; No. 302, ADR-0143). The No. 526
    //    erasure fns (voice_delete/voice_list) stay compiled in EVERY
    //    build — the GDPR Art. 17 path cannot depend on a build flag —
    //    only the USER-PROGRAM rows gate. ──
    ParkedSpec {
        name: "voice_enroll",
        feature: Some("voice"),
        basis: "the voice pillar skeleton is the opt-in `voice` build (No. 302, ADR-0143)",
    },
    ParkedSpec {
        name: "tts_speak",
        feature: Some("voice"),
        basis: "the voice pillar skeleton is the opt-in `voice` build (No. 302, ADR-0143)",
    },
    ParkedSpec {
        name: "audio_export",
        feature: Some("voice"),
        basis: "the voice pillar skeleton is the opt-in `voice` build (No. 302, ADR-0143)",
    },
    ParkedSpec {
        name: "voice_design",
        feature: Some("voice"),
        basis: "the voice pillar skeleton is the opt-in `voice` build (No. 302, ADR-0143)",
    },
    ParkedSpec {
        name: "voice_save",
        feature: Some("voice"),
        basis: "the voice pillar skeleton is the opt-in `voice` build (No. 302, ADR-0143)",
    },
    ParkedSpec {
        name: "voice_load",
        feature: Some("voice"),
        basis: "the voice pillar skeleton is the opt-in `voice` build (No. 302, ADR-0143)",
    },
    ParkedSpec {
        name: "voice_delete",
        feature: Some("voice"),
        basis: "the user-program row is voice-gated; the No. 526 erasure fns stay compiled in every build",
    },
    ParkedSpec {
        name: "voice_list",
        feature: Some("voice"),
        basis: "the user-program row is voice-gated; the No. 526 erasure fns stay compiled in every build",
    },
    // ── Video (the opt-in `video` build; No. 307/No. 309, ADR-0151) ──
    ParkedSpec {
        name: "video_render",
        feature: Some("video"),
        basis: "the video pillar is the opt-in `video` build (No. 307, ADR-0151)",
    },
    ParkedSpec {
        name: "frame_interp",
        feature: Some("video"),
        basis: "the video pillar is the opt-in `video` build (No. 307, ADR-0151)",
    },
    ParkedSpec {
        name: "video_extend",
        feature: Some("video"),
        basis: "the video pillar is the opt-in `video` build (No. 307, ADR-0151)",
    },
    ParkedSpec {
        name: "video_fetch_weights",
        feature: Some("video"),
        basis: "the video pillar is the opt-in `video` build (No. 307, ADR-0151)",
    },
];

/// Name → the parked spec, if the name is on the parked surface.
pub fn parked_spec(name: &str) -> Option<&'static ParkedSpec> {
    PARKED_REGISTRY.iter().find(|p| p.name == name)
}

/// The refusal message — the class prefix rides the TEXT (the audit
/// finding form: machine consumers extract the first `[CHECK_ID]`), the
/// structured kind rides `SpannedError::kind` / the coded error.
pub fn refusal_message(spec: &ParkedSpec) -> String {
    match spec.feature {
        Some(f) => format!(
            "[BUILTIN_PARKED] {} requires --features {} (parked: {})",
            spec.name, f, spec.basis
        ),
        None => format!(
            "[BUILTIN_PARKED] {} is not callable in this build (parked: {})",
            spec.name, spec.basis
        ),
    }
}
