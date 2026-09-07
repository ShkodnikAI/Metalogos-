// ── tests/naryad_201_reflex_taint.rs ───────────────────────────────
// Наряд №201, Contracts 1-6: Reflex and learnable pattern taint model.
//
// Verifies the security audit catches:
// 1. respond(reflex_generate(...)) → HTML_INJECTION (direct + via variable)
// 2. env() in reflex_train data → SECRET_LEAK (positive + negative)
// 3. json_body() in reflex_train data → UNTRUSTED_TRAINING_DATA
// 4. Learnable pattern output in respond() → HTML_INJECTION
// 5. Error contract examples (golden)
// 6. Regression: no false positives on existing patterns

use metalogos::check_program;

fn has_finding(source: &str, check_id: &str) -> bool {
    match check_program(source) {
        Ok(result) => result.errors.iter().any(|e| e.message.contains(check_id)),
        Err(_) => false,
    }
}

fn has_no_findings(source: &str) -> bool {
    match check_program(source) {
        Ok(result) => result.errors.is_empty(),
        Err(_) => false,
    }
}

// ── Contract 1: respond(reflex_generate(...)) → HTML_INJECTION ────────

#[test]
fn c1_reflex_generate_direct_to_respond_is_html_injection() {
    let source = r#"
        reflex_gen StoryModel {
            input: embedding(16)
            vocab_size: 100
            layers: [transformer_block(4, 16, 64)]
            seed: 42
        }

        pattern Gen(x: String) -> String {
            respond("200 OK", reflex_generate(StoryModel, x, 10.0, 0.5))
            return ""
        }
    "#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "respond(reflex_generate(...)) must trigger HTML_INJECTION"
    );
}

#[test]
fn c1_reflex_generate_via_variable_is_html_injection() {
    let source = r#"
        reflex_gen StoryModel {
            input: embedding(16)
            vocab_size: 100
            layers: [transformer_block(4, 16, 64)]
            seed: 42
        }

        pattern Gen(x: String) -> String {
            let g = reflex_generate(StoryModel, x, 10.0, 0.5)
            respond("200 OK", g)
            return ""
        }
    "#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "reflex_generate output via variable → respond() must trigger HTML_INJECTION"
    );
}

// ── Contract 2: env() in reflex_train data → SECRET_LEAK ────────────

#[test]
fn c2_env_in_reflex_train_data_is_secret_leak() {
    let source = r#"
        reflex Classifier {
            input: embedding(2)
            layers: [dense(8, relu), dense(2, softmax)]
            labels: ["a", "b"]
            seed: 42
        }

        pattern Train(x: String) -> String {
            let s = env("SECRET_KEY")
            let data = [[s, 0.0]]
            reflex_train(Classifier, data, 10.0, "accuracy", 0.5)
            return "ok"
        }
    "#;
    assert!(
        has_finding(source, "SECRET_LEAK"),
        "env() value in reflex_train data must trigger SECRET_LEAK"
    );
}

#[test]
fn c2_literal_data_in_reflex_train_no_finding() {
    // Positive control: literal data arrays → no findings.
    let source = r#"
        reflex Classifier {
            input: embedding(2)
            layers: [dense(8, relu), dense(2, softmax)]
            labels: ["a", "b"]
            seed: 42
        }

        pattern Train(x: String) -> String {
            let data = [[0.1, 0.2, 0.0], [0.3, 0.4, 1.0]]
            reflex_train(Classifier, data, 10.0, "accuracy", 0.5)
            return "ok"
        }
    "#;
    assert!(
        !has_finding(source, "SECRET_LEAK"),
        "literal data in reflex_train must NOT trigger SECRET_LEAK"
    );
    assert!(
        !has_finding(source, "UNTRUSTED_TRAINING_DATA"),
        "literal data in reflex_train must NOT trigger UNTRUSTED_TRAINING_DATA"
    );
}

// ── Contract 3: json_body() in reflex_train → UNTRUSTED_TRAINING_DATA ─

#[test]
fn c3_json_body_in_reflex_train_is_untrusted_training_data() {
    let source = r#"
        reflex Classifier {
            input: embedding(2)
            layers: [dense(8, relu), dense(2, softmax)]
            labels: ["a", "b"]
            seed: 42
        }

        pattern Train(x: String) -> String {
            let body = json_body()
            let data = [[body, 0.0]]
            reflex_train(Classifier, data, 10.0, "accuracy", 0.5)
            return "ok"
        }
    "#;
    assert!(
        has_finding(source, "UNTRUSTED_TRAINING_DATA"),
        "json_body() in reflex_train data must trigger UNTRUSTED_TRAINING_DATA"
    );
}

#[test]
fn c3_query_param_in_reflex_train_is_untrusted_training_data() {
    let source = r#"
        reflex Classifier {
            input: embedding(2)
            layers: [dense(8, relu), dense(2, softmax)]
            labels: ["a", "b"]
            seed: 42
        }

        pattern Train(x: String) -> String {
            let q = query_param("text")
            let data = [[q, 0.0]]
            reflex_train(Classifier, data, 10.0, "accuracy", 0.5)
            return "ok"
        }
    "#;
    assert!(
        has_finding(source, "UNTRUSTED_TRAINING_DATA"),
        "query_param() in reflex_train data must trigger UNTRUSTED_TRAINING_DATA"
    );
}

// ── Contract 4: Learnable pattern output in respond() → HTML_INJECTION ─

#[test]
fn c4_learnable_pattern_output_in_respond_is_html_injection() {
    let source = r#"
        learnable pattern Classify(text: String) -> String {
            prompt: "Classify this text"
        }

        pattern Run(x: String) -> String {
            respond("200 OK", Classify(x))
            return ""
        }
    "#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "Learnable pattern output → respond() must trigger HTML_INJECTION (ADR-0117: learnable output is untrusted)"
    );
}

#[test]
fn c4_learnable_pattern_via_variable_in_respond_is_html_injection() {
    let source = r#"
        learnable pattern Classify(text: String) -> String {
            prompt: "Classify this text"
        }

        pattern Run(x: String) -> String {
            let result = Classify(x)
            respond("200 OK", result)
            return ""
        }
    "#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "Learnable pattern output via variable → respond() must trigger HTML_INJECTION"
    );
}

// ── Contract 6: Regression — no false positives on existing patterns ─

#[test]
fn c6_clean_program_no_false_positives() {
    let source = r#"
        entity greeting: String = "Hello"

        pattern Shout(s: String) -> String {
            return upper(s) + "!"
        }
    "#;
    assert!(
        has_no_findings(source),
        "Clean program with no taint sources must produce 0 findings"
    );
}

#[test]
fn c6_program_without_reflex_train_no_secret_leak() {
    // Program without reflex_train must not produce SECRET_LEAK or
    // UNTRUSTED_TRAINING_DATA for reflex_train.
    let source = r#"
        entity x: Float = 3.0

        pattern Double(n: Float) -> Float {
            let doubled = n + n
            return doubled
        }
    "#;
    assert!(
        !has_finding(source, "SECRET_LEAK"),
        "Program without reflex_train must not produce SECRET_LEAK"
    );
    assert!(
        !has_finding(source, "UNTRUSTED_TRAINING_DATA"),
        "Program without reflex_train must not produce UNTRUSTED_TRAINING_DATA"
    );
}

#[test]
fn c6_render_sanitizes_reflex_generate() {
    // render() should clear the LlmOutput taint from reflex_generate,
    // so respond(render(reflex_generate(...))) is safe.
    let source = r#"
        reflex_gen StoryModel {
            input: embedding(16)
            vocab_size: 100
            layers: [transformer_block(4, 16, 64)]
            seed: 42
        }

        pattern Gen(x: String) -> String {
            respond("200 OK", render(reflex_generate(StoryModel, x, 10.0, 0.5)))
            return ""
        }
    "#;
    assert!(
        !has_finding(source, "HTML_INJECTION"),
        "render(reflex_generate(...)) should NOT trigger HTML_INJECTION (render sanitizes)"
    );
}
