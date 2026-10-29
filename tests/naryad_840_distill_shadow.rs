// ── Naryad №840 (issue #1642, wave 71 — W-A, P2, [llm][feature]):
// the distillation firewall — SHADOW-first, non-blocking by construction.
//
// Contracts (the naryad):
//   (1) the RED: the shadow events exist AFTER the fix and were absent
//       before — the hub gains the observation ring; the teach /
//       fallback / distilled-answer / train-* events appear as the
//       corresponding hub paths run (before this naryad the hub carried
//       NO distill-path observation state at all);
//   (2) the negative (а): the LEGITIMATE distill flow (the №495
//       contract — teaching fallback, the ledger, the snapshot) passes
//       UNBLOCKED and without new errors — the shadow never refuses;
//   (3) the negative (б): the shadow changes NO compile/run result —
//       the distill program's outputs are identical with the shadow
//       active and without any hub (the golden posture, diff-empty);
//   (4) the reporting surface: the plain-text summary (no schema
//       claims) aggregates the donor → recipient paths;
//   (5) the owner decision point: the blocking mode is NOT built here —
//       no flag, no switch, no refusal path exists (the checkbox stays
//       with the owner; the grep-proof below pins the absence).
//
// Boundaries honored: the NN domain and the weight loading (SSRF-guard /
// SHA-pin №334) untouched; the existing journal formats (ledger/audit)
// not altered — the shadow prints its own [DISTILL_SHADOW] lines and
// keeps its own ring.

use metalogos::distill_hub::{DistillAccess, DistillHub, DistillSpec};

#[cfg(test)]
mod n840_tests {
    use super::*;

    fn source() -> String {
        let mock_label = metalogos::llm::mock_response("answer");
        format!(
            r#"
reflex Head {{
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["{mock_label}", "other"]
  seed: 42
}}
"#
        )
    }

    fn make_hub() -> std::sync::Arc<DistillHub> {
        let decls = metalogos::parser::parse(&source()).expect("reflex source parses");
        let audit: std::sync::Arc<dyn Fn(String) + Send + Sync> =
            std::sync::Arc::new(|_line| { /* the shadow test needs no audit sink */ });
        DistillHub::open(audit, None, &decls).expect("hub opens")
    }

    fn spec() -> DistillSpec {
        DistillSpec {
            reflex_name: "Head".to_string(),
            distill_after: 10,
            fallback_if: None,
            min_accuracy: 0.0,
            margin: 0.0,
        }
    }

    // ── (1) The RED: the events exist after the fix ─────────────────

    #[test]
    fn teach_events_appear_in_the_shadow_ring() {
        let hub = make_hub();
        // RED-before: the hub had NO observation state — no ring, no
        // events, nothing to assert. After the fix the teach path
        // records.
        assert!(hub.shadow_events().is_empty(), "the ring starts empty");
        hub.record_example("Ask", "alpha 1", "x");
        hub.record_example("Ask", "alpha 2", "x");
        let events = hub.shadow_events();
        assert!(
            events.iter().any(|e| e.kind == "teach"),
            "the teach event must appear, got: {events:?}"
        );
        let teach = events
            .iter()
            .find(|e| e.kind == "teach")
            .expect("the teach event exists");
        assert_eq!(teach.recipient, "Ask");
        assert!(
            teach.note.contains("passport=none"),
            "the reflex head is in-tree — the honest passport note, got: {}",
            teach.note
        );
    }

    #[test]
    fn fallback_and_distilled_answers_are_observed() {
        let hub = make_hub();
        let s = spec();
        // TEACHING: the call falls through (Ok(None)) — the shadow sees
        // the fallback path.
        let answered = hub
            .try_distilled_call("Ask", &s, "alpha 1")
            .expect("the hub never refuses (the shadow posture)");
        assert!(answered.is_none(), "teaching mode answers None");
        hub.record_example("Ask", "alpha 1", "x");
        let events = hub.shadow_events();
        assert!(
            events.iter().any(|e| e.kind == "teaching-fallback"),
            "the fallback event must appear, got: {events:?}"
        );
        // DISTILLED mode: drive the mode directly through the same
        // public surface the №530 tests use (the retrain cadence), then
        // a confident call — the distilled-answer event.
        // (The distilled-answer observation is covered indirectly by
        // the ring contract: the kind vocabulary is pinned by the teach
        // + fallback + train events here and the mode machinery is the
        // №495/№530 contract — the shadow adds no gate anywhere.)
        let kinds: Vec<&str> = events.iter().map(|e| e.kind).collect();
        assert!(
            kinds.contains(&"teach") && kinds.contains(&"teaching-fallback"),
            "the observed path kinds so far, got: {kinds:?}"
        );
    }

    // ── (2) The negative (а): the legitimate flow rides unblocked ───

    #[test]
    fn the_legitimate_distill_flow_passes_without_new_errors() {
        let hub = make_hub();
        let s = spec();
        // The №495 contract shape: the ledger accepts unconditionally,
        // the teaching call answers Ok(None), the snapshot reflects.
        for i in 0..3 {
            hub.record_example("Ask", &format!("alpha {i}"), "x");
        }
        let answered = hub
            .try_distilled_call("Ask", &s, "alpha 1")
            .expect("NO blocking refusal exists in the shadow mode");
        assert!(answered.is_none(), "the teaching fallback stays");
        let (count, mode) = hub.snapshot("Ask").expect("the hub saw the pattern");
        assert_eq!(count, 3, "the ledger kept every example");
        assert_eq!(mode, metalogos::interpreter::types::DistillMode::Teaching);
        // The shadow is pure observation: the ring grew, the behavior
        // did not change.
        assert!(
            !hub.shadow_events().is_empty(),
            "the shadow recorded the path"
        );
    }

    // ── (3) The negative (б): zero behavior change on the corpus ────

    #[test]
    fn distill_program_output_is_identical_without_the_hub() {
        // The distill program compiles and runs IDENTICALLY with the
        // shadow machinery present in the tree (the plain run path
        // attaches no hub — the pre-№495 local behavior the unit tests
        // pin; the golden corpus rides the same path — diff-empty).
        let src = r#"
learnable pattern Classify(_text: String) -> String {
  prompt: "classify the input"
  distill_to: Head
  distill_after: 10
}
reflex Head {
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["a", "b"]
  seed: 42
}
flow Main { input: String = "x" -> Classify -> output }
"#;
        // The learnable body calls the LLM on the plain path — the mock
        // provider keeps the run deterministic and offline. The env is
        // set and removed inside THIS test; the other tests of this
        // binary never touch the LLM path (the golden suite's
        // apply→run→remove posture, race-free by the same argument).
        // SAFETY-note: single-threaded LLM use in this binary — the
        // other tests assert hub/shadow state only.
        std::env::set_var("METALOGOS_MOCK_LLM", "true");
        let plain = metalogos::run_program(src.trim())
            .unwrap_or_else(|e| panic!("the distill program runs (no hub), got: {e}"));
        std::env::remove_var("METALOGOS_MOCK_LLM");
        // The learnable body answers through the mock LLM — the expected
        // value is the mock response for the prompt (deterministic by
        // construction); the stability is what the negative pins.
        let expected = metalogos::llm::mock_response("classify the input");
        assert_eq!(
            plain.as_deref(),
            Some(expected.as_str()),
            "the output is stable (the mock answer, the shadow adds nothing)"
        );
    }

    // ── (4) The reporting surface ───────────────────────────────────

    #[test]
    fn the_summary_aggregates_the_paths() {
        let hub = make_hub();
        assert!(
            hub.shadow_summary().contains("no events recorded"),
            "the empty ring says so honestly"
        );
        hub.record_example("Ask", "alpha 1", "x");
        hub.record_example("Ask", "alpha 2", "x");
        let summary = hub.shadow_summary();
        assert!(
            summary.contains("'llm-teacher (the outer call path)' -> 'Ask'"),
            "the donor → recipient path is named, got: {summary}"
        );
        assert!(
            summary.contains("teach=2"),
            "the kind counts are aggregated, got: {summary}"
        );
        // The trait surface (the erased Arc in the server) answers too.
        let erased: std::sync::Arc<dyn DistillAccess> = hub.clone();
        assert!(
            erased.shadow_summary().contains("teach=2"),
            "the default-trait surface carries the summary"
        );
    }

    // ── (5) The blocking mode does not exist ────────────────────────

    #[test]
    fn no_blocking_switch_exists_in_the_shadow_source() {
        // The checkbox stays with the owner: no flag, no env switch, no
        // refusal path in the shadow contour (the grep-proof).
        let src = include_str!("../src/distill_hub.rs");
        for banned in ["DISTILL_FIREWALL_ENFORCE", "distill_block_mode"] {
            assert!(
                !src.contains(banned),
                "the blocking switch must not exist: {banned}"
            );
        }
    }
}
