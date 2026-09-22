// ── Naryad #352: the DIRECTED effect trails — the compile contract ────
//
// ADR-0174 §3.2: the direction words ride the №324 trail gate
// (factual ⊑ declared). Consequences pinned here:
//   T1  a pattern calling a TTS builtin and declaring only ⟨io, audit⟩
//       FAILS to compile — the excess `speak` is named;
//   T2  a listen-declared pattern that also calls a speak builtin
//       FAILS (the direction conflict is a compile error —
//       «конфликт направлений без типов — ошибка компиляции»);
//   T3  a DUPLEX pattern declares BOTH words — ⟨io, audit, listen,
//       speak⟩ — and compiles CLEAN (the signature IS the duplex type);
//   T4  a listen-only pattern (STT + listen stream) with the listen
//       word declared compiles clean; the omni surface (both) refuses
//       a single-direction declaration;
//   T5  the duplex stream surfaces carry their direction statically:
//       speak_start needs `speak`, listen_start needs `listen`.
//
// The voice builtins are feature-gated, but the direction table is
// semantic-side (ADR-0174 §3.1) — the gate fires on the NAME, so the
// contract is testable without the voice feature (and the analysis is
// backend-independent).

use metalogos::semantic;

fn check(source: &str) -> Vec<String> {
    let declarations = metalogos::parser::parse(source).expect("parses");
    semantic::check_program(&declarations)
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

fn errors_containing(errors: &[String], needle: &str) -> Vec<String> {
    errors
        .iter()
        .filter(|m| m.contains(needle))
        .cloned()
        .collect()
}

// ── T1: the undeclared speak direction is a compile error ──────────────

#[test]
fn undeclared_speak_is_a_compile_error_naming_the_word() {
    let src = r#"
pattern Answer(q: String) -> String ⟨io, audit⟩ {
  let _ = tts_speak(q, "voice-demo")
  return "ok"
}
flow Main { input: String = "" -> Answer -> output }
"#;
    let errors = check(src);
    let hits = errors_containing(&errors, "speak");
    assert!(
        !hits.is_empty(),
        "the excess 'speak' must be a compile error; got: {errors:?}"
    );
    // The №324 shape: the trail is named with the excess effect.
    assert!(
        hits.iter().any(|m| m.contains("⟨io, audit⟩")),
        "the error names the declared trail: {hits:?}"
    );
}

// ── T2: the direction CONFLICT (both flows, one word declared) ────────

#[test]
fn direction_conflict_without_the_type_is_a_compile_error() {
    // A DUPLEX body (STT + TTS) declaring only the listen word — the
    // speak half is the conflict.
    let src = r#"
pattern Half(text: String) -> String ⟨io, audit, listen⟩ {
  let heard = stt_transcribe(text)
  let _ = tts_speak(heard, "voice-demo")
  return heard
}
flow Main { input: String = "" -> Half -> output }
"#;
    let errors = check(src);
    let hits = errors_containing(&errors, "speak");
    assert!(
        !hits.is_empty(),
        "the direction conflict must be a compile error; got: {errors:?}"
    );
    assert!(
        errors_containing(&errors, "excess: listen").is_empty(),
        "the LISTEN half is declared — no EXCESS about it; got: {errors:?}"
    );
}

// ── T3: the duplex declaration compiles clean ──────────────────────────

#[test]
fn the_duplex_trail_compiles_clean() {
    // The clean-duplex shape uses NON-gated surfaces (speak_start —
    // tts_speak is voice-feature-gated and would add an unrelated
    // "undefined" note without the feature; stt_transcribe is ungated).
    let src = r#"
pattern Duplex(ch: Duplex, text: String) -> String ⟨io, audit, listen, speak⟩ {
  let heard = stt_transcribe(text)
  let _ = speak_start(ch, heard)
  return heard
}
flow Main { input: String = "" -> Duplex -> output }
"#;
    let errors = check(src);
    assert!(
        errors.is_empty(),
        "the duplex signature covers both directions; got: {errors:?}"
    );
}

// ── T4: the listen-only declaration and the omni surface ──────────────

#[test]
fn listen_only_trail_covers_stt_but_not_the_omni_surface() {
    let listen_only = r#"
pattern Ear(audio: String) -> String ⟨io, audit, listen⟩ {
  return stt_transcribe(audio)
}
flow Main { input: String = "" -> Ear -> output }
"#;
    let errors = check(listen_only);
    assert!(
        errors.is_empty(),
        "stt_transcribe is a listen flow — the listen trail covers it; got: {errors:?}"
    );

    // omni_ask is BOTH directions (№334): a single-direction trail
    // refuses — the excess speak is named.
    let omni = r#"
pattern Ear2(audio: String) -> String ⟨io, audit, listen⟩ {
  return omni_ask(audio)
}
flow Main { input: String = "" -> Ear2 -> output }
"#;
    let errors2 = check(omni);
    assert!(
        !errors_containing(&errors2, "speak").is_empty(),
        "omni_ask needs BOTH words; got: {errors2:?}"
    );
}

// ── T5: the stream surfaces carry their direction statically ──────────

#[test]
fn the_stream_surfaces_need_their_own_direction_word() {
    // speak_start in a listen-declared pattern — the speak excess.
    let src = r#"
pattern Speaker(ch: Duplex) -> String ⟨io, audit, listen⟩ {
  let _ = speak_start(ch, "text")
  return "ok"
}
flow Main { input: String = "" -> Speaker -> output }
"#;
    let errors = check(src);
    assert!(
        !errors_containing(&errors, "speak").is_empty(),
        "speak_start is a speak flow; got: {errors:?}"
    );
    // listen_start in the same pattern — covered by the declared listen.
    let src2 = r#"
pattern Listener(ch: Duplex) -> String ⟨io, audit, listen⟩ {
  let _ = listen_start(ch, "high")
  return "ok"
}
flow Main { input: String = "" -> Listener -> output }
"#;
    let errors2 = check(src2);
    assert!(
        errors2.is_empty(),
        "listen_start is a listen flow — covered; got: {errors2:?}"
    );
    // speak_stop also needs the speak word (the direction is static).
    let src3 = r#"
pattern Stopper(ch: Duplex) -> String ⟨io, audit⟩ {
  let _ = speak_stop(ch)
  return "ok"
}
flow Main { input: String = "" -> Stopper -> output }
"#;
    let errors3 = check(src3);
    assert!(
        !errors_containing(&errors3, "speak").is_empty(),
        "speak_stop carries the speak direction; got: {errors3:?}"
    );
}

// ── The trail words parse in the closed vocabulary ─────────────────────

#[test]
fn the_new_words_are_in_the_effect_vocabulary_and_unknown_words_stay_loud() {
    // The words themselves are valid now (a no-op pattern declares them).
    let src = r#"
pattern Both(x: String) -> String ⟨io, audit, listen, speak⟩ {
  return x
}
flow Main { input: String = "" -> Both -> output }
"#;
    assert!(check(src).is_empty());
    // An unknown word stays a loud error (the №324 discipline).
    let bad = r#"
pattern Bad(x: String) -> String ⟨io, teleport⟩ {
  return x
}
flow Main { input: String = "" -> Bad -> output }
"#;
    let errors = check(bad);
    assert!(
        !errors_containing(&errors, "unknown effect word").is_empty(),
        "unknown words stay loud; got: {errors:?}"
    );
}
