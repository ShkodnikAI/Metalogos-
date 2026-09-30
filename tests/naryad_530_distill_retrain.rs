//! №530 (issue #839; audit 30.09 N-4): the out-of-lock training, the
//! DISTILLED retraining, and the loud degradation → TEACHING revert.
//!
//! INTEGRATION tests on purpose: the hub's unit tests would add a
//! distill_hub → parser source edge, and the C4 acyclicity ratchet
//! (№382) fails on the new module cycle. The public surface
//! (DistillHub::open + the DistillAccess trait + raw_registry_for_tests)
//! is enough to drive the whole machine.

use metalogos::distill_hub::{DistillAccess, DistillHub, DistillSpec};
use metalogos::interpreter::types::DistillMode;
// ── №530 (issue #839; audit 30.09 N-4): the out-of-lock training, the
// DISTILLED retraining, and the loud degradation → TEACHING revert ──
#[cfg(test)]
mod n530_tests {
    use super::*;

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

    struct Fixture {
        hub: std::sync::Arc<DistillHub>,
        audit_lines: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        pattern: &'static str,
        label: String,
    }

    fn make_hub() -> Fixture {
        let decls = metalogos::parser::parse(&source()).expect("reflex source parses");
        let audit_lines: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&audit_lines);
        let audit: std::sync::Arc<dyn Fn(String) + Send + Sync> =
            std::sync::Arc::new(move |line| {
                sink.lock().unwrap_or_else(|e| e.into_inner()).push(line)
            });
        let hub = DistillHub::open(audit, None, &decls).expect("hub opens");
        let label = metalogos::llm::mock_response("answer");
        Fixture {
            hub,
            audit_lines,
            pattern: "Ask",
            label,
        }
    }

    fn spec(distill_after: usize) -> DistillSpec {
        DistillSpec {
            reflex_name: "Head".to_string(),
            distill_after,
            fallback_if: Some((metalogos::ast::CompareOp::Lt, 0.7)),
            min_accuracy: 0.8,
            margin: 0.05,
        }
    }

    fn wait_for(fixture: &Fixture, marker: &str, timeout_ms: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        while std::time::Instant::now() < deadline {
            if fixture
                .audit_lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .any(|l| l.contains(marker))
            {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        false
    }

    fn fingerprint(fixture: &Fixture) -> u64 {
        let id = fixture
            .hub
            .reflex_id_for_tests("Head")
            .expect("the reflex handle");
        let reg_arc = fixture.hub.raw_registry_for_tests();
        let reg = reg_arc.lock().unwrap_or_else(|e| e.into_inner());
        match reg.get(id) {
            Some(metalogos::nn::ModelKind::Dense(m)) => m.weights_fingerprint(),
            _ => panic!("a Dense model exists"),
        }
    }

    fn mode(fixture: &Fixture) -> DistillMode {
        fixture
            .hub
            .snapshot(fixture.pattern)
            .map(|(_, m)| m)
            .unwrap_or(DistillMode::Teaching)
    }

    #[test]
    fn n530_failed_gate_leaves_weights_untouched() {
        // The initial training passes (single-label data: the №496 honest
        // trick — the all-one-label holdout accuracy is 1.0 and the
        // single-class carve-out applies min_accuracy raw).
        let f = make_hub();
        for i in 0..25 {
            f.hub
                .record_example(f.pattern, &format!("alpha {i}"), &f.label);
        }
        let s = spec(20);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        assert!(
            wait_for(&f, "distill.training-finished", 15000),
            "the initial training finishes"
        );
        assert_eq!(mode(&f), DistillMode::Distilled);

        // The retrain: 30 NEW examples on the SAME input with the
        // OPPOSITE label — the feature vectors collapse to one point
        // (dim 4 hashed TF-IDF), the majority-baseline gate (№485)
        // refuses the degenerate majority model: the gate REJECTS.
        let other = "other";
        for i in 0..30 {
            let _ = i;
            f.hub.record_example(f.pattern, "alpha conflict", other);
        }
        let before = fingerprint(&f);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        assert!(
            wait_for(&f, "distill.degraded", 15000),
            "the loud degradation audit lands"
        );
        assert_eq!(mode(&f), DistillMode::Teaching, "the loud TEACHING revert");
        let after = fingerprint(&f);
        assert_eq!(
            before, after,
            "the live weights are untouched by the failed-gate retrain"
        );
    }

    #[test]
    fn n530_retrain_succeeds_and_stays_distilled() {
        let f = make_hub();
        for i in 0..25 {
            f.hub
                .record_example(f.pattern, &format!("alpha {i}"), &f.label);
        }
        let s = spec(20);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        assert!(wait_for(&f, "distill.training-finished", 15000));
        assert_eq!(mode(&f), DistillMode::Distilled);

        // Fresh CONSISTENT examples — the retrain passes the gate and the
        // pattern stays DISTILLED (the weights swap in atomically).
        for i in 0..25 {
            f.hub
                .record_example(f.pattern, &format!("gamma {i}"), &f.label);
        }
        f.hub.try_distilled_call(f.pattern, &s, "gamma 1").unwrap();
        assert!(
            wait_for(&f, "distill.retraining-started", 15000),
            "the retrain trigger fires on the accumulated buffer"
        );
        assert!(
            wait_for(&f, "distill.training-finished", 15000),
            "the retrain finishes"
        );
        assert!(
            !f.audit_lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .any(|l| l.contains("distill.degraded")),
            "no degradation on consistent data"
        );
        assert_eq!(mode(&f), DistillMode::Distilled);
    }

    #[test]
    fn n530_training_does_not_hold_the_registry_lock() {
        // The large buffer makes the 30-epoch run long enough (hundreds of
        // ms) that the pre-№530 behavior — holding the registry lock for
        // the WHOLE run — is distinguishable deterministically: the test
        // acquires the lock repeatedly and every acquisition BEFORE the
        // finished line must be sub-100ms (the old code blocked for the
        // whole training).
        let f = make_hub();
        for i in 0..4000 {
            f.hub
                .record_example(f.pattern, &format!("alpha token{i} word{i} {i}"), &f.label);
        }
        let s = spec(20);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        // Wait for the STARTED line — the trainer is now inside the run.
        assert!(wait_for(&f, "distill.training-started", 15000));
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut acquired_before_finish = 0usize;
        let mut worst = std::time::Duration::ZERO;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let finished = |f: &Fixture| {
            f.audit_lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .any(|l| l.contains("distill.training-finished"))
        };
        while std::time::Instant::now() < deadline && !finished(&f) {
            let t0 = std::time::Instant::now();
            let reg_arc = f.hub.raw_registry_for_tests();
            {
                let _reg = reg_arc.lock().unwrap_or_else(|e| e.into_inner());
            }
            drop(reg_arc);
            let elapsed = t0.elapsed();
            worst = worst.max(elapsed);
            acquired_before_finish += 1;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(finished(&f), "the training finishes within the deadline");
        assert!(
            acquired_before_finish >= 3,
            "the lock was acquirable DURING training (acquired {acquired_before_finish})"
        );
        assert!(
            worst < std::time::Duration::from_millis(100),
            "every in-training acquisition is fast (worst {worst:?}) — the lock is never held by an epoch loop"
        );
    }
}
