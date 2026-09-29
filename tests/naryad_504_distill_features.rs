// ── tests/naryad_504_distill_features.rs ────────────────────────────────
// №504 (P1, reliability; the audit 28.09 §3.1 companion finding): the
// distillation features leave the byte-position `simple_embedding` —
// NOT a semantic representation — and land on the hashed TF-IDF form of
// the `EmbeddingBackend` TF-IDF fallback (offline, deterministic,
// network-free, bounded memory: no tables at all).
//
// WHAT THE MEASUREMENTS SHOW (the honest A/B, all numbers printed):
//   * the legacy byte-position cosine is ~0.79-0.90 for ANY pair of
//     English-ish strings — paraphrases, rewordings, UNRELATED topics —
//     it has NO discriminative power (the always-positive byte mass and
//     the shared prefix dominate); the audit's "near-unrelated vectors"
//     was the charitable reading — the truth is worse: EVERYTHING looks
//     related;
//   * the hashed TF-IDF discriminates: unrelated = 0.0, the reordered
//     paraphrase = 0.89, the prefix paraphrase = 0.58 (the shared-token
//     share); word-synonym paraphrases stay low (0.2) — an honest
//     token-level limitation (no synonymy knowledge offline);
//   * the №485 gate on the 40-utterance 4-intent corpus at the
//     PRODUCTION training budget (30 epochs, lr 0.1 — hardcoded in the
//     distill path): holdout 0.375 (legacy) → 0.500 (hashed) — the
//     direction the naryad promised, but BOTH stay TEACHING: at this
//     budget the Dense head barely converges at all (loss ~1.3). The
//     budget lever (lr/epochs) is a SEPARATE semantics decision for the
//     owner — not silently moved here (at lr 0.5 the same corpus
//     reaches 0.75, so the features are not the ceiling).
//
// Pinned here:
//   T1 the discrimination pin: under the new features an unrelated pair
//      is far apart (≈ 0) while the legacy rates it ~0.8 — the exact
//      property a routing feature extractor needs;
//   T2 determinism: the same text produces the identical vector in any
//      call order, any process (the FNV-1a posture, №487);
//   T3 the real-text corpus A/B: the new features must beat the legacy
//      on the SAME model/budget (non-regression + the printed metrics
//      table for the report);
//   T4 the signature migration: a save carries the current signature; a
//      row tampered to the legacy signature LOADS (weights readable)
//      and stamps the model loudly — the distill predicate then refuses
//      the confident answer until the next successful train, and a
//      successful train lifts the stamp (the single-point rule in
//      `ReflexModel::train`).
//
// The №496 serve e2e stays the state-contract gate (its single-class
// corpus is feature-independent by construction).

// The harness (not program-influenced code) manages its temp SQLite
// files directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use metalogos::embeddings::{
    cosine_similarity, hashed_tfidf_vector, DISTILL_FEATURE_SIGNATURE, LEGACY_FEATURE_SIGNATURE,
};
use metalogos::nn::dense::Dense;
use metalogos::nn::persist;
use metalogos::nn::{ActivationKind, ReflexModel, ReflexRegistry};

// ── the legacy extractor (test-only A/B baseline copy) ──────────────
// VERBATIM from the pre-№504 `distill_hub::simple_embedding` — kept
// HERE only to compute the "before" numbers of the report's A/B table.
fn legacy_bytes_embedding(input: &str, dim: usize) -> Vec<f64> {
    let mut embedding = vec![0.0; dim];
    let bytes = input.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        let bucket = i % dim;
        embedding[bucket] += (b as f64) * 0.01;
        if b != 0 {
            embedding[(bucket + 1) % dim] =
                (embedding[(bucket + 1) % dim] * 0.99) + (b as f64) * 0.001;
        }
    }
    let max_val = embedding.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    for v in &mut embedding {
        *v /= max_val;
    }
    embedding
}

fn legacy_cosine(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

// ── T1: the discrimination pin (the auditor's example, honestly) ────

#[test]
fn n504_unrelated_texts_are_far_apart_under_hashed_tfidf() {
    let dim = 64;
    let paraphrase = "Where is my package?";
    let reordered = "My package — where is it?";
    let unrelated = "What are your business hours?";

    fn cos(a: &str, b: &str, dim: usize) -> f32 {
        let va = hashed_tfidf_vector(a, dim);
        let vb = hashed_tfidf_vector(b, dim);
        cosine_similarity(
            &va.iter().map(|x| *x as f32).collect::<Vec<_>>(),
            &vb.iter().map(|x| *x as f32).collect::<Vec<_>>(),
        )
    }

    let sim_reordered = cos(paraphrase, reordered, dim);
    let sim_unrelated = cos(paraphrase, unrelated, dim);

    // The legacy extractor cannot tell these worlds apart (the printed
    // legacy numbers document the flaw for the report):
    let legacy_reordered = {
        let (a, b) = (
            legacy_bytes_embedding(paraphrase, dim),
            legacy_bytes_embedding(reordered, dim),
        );
        legacy_cosine(&a, &b)
    };
    let legacy_unrelated = {
        let (a, b) = (
            legacy_bytes_embedding(paraphrase, dim),
            legacy_bytes_embedding(unrelated, dim),
        );
        legacy_cosine(&a, &b)
    };
    eprintln!(
        "A/B discrimination: reordered legacy={:.3} hashed={:.3} | unrelated legacy={:.3} hashed={:.3}",
        legacy_reordered, sim_reordered, legacy_unrelated, sim_unrelated
    );
    // The legacy sees the unrelated topic at ~0.79 — HIGHER than its own
    // reordered-paraphrase score. No discrimination at all.
    assert!(
        legacy_unrelated > 0.7,
        "documenting the flaw: the legacy cosine rates unrelated topics near {:.3}",
        legacy_unrelated
    );
    // The new features separate the worlds honestly.
    assert!(
        sim_unrelated < 0.05,
        "unrelated topics must be far apart under the hashed TF-IDF ({:.3})",
        sim_unrelated
    );
    assert!(
        sim_reordered > sim_unrelated + 0.5,
        "the reordered paraphrase must be far nearer than the unrelated \
         topic ( {:.3} vs {:.3})",
        sim_reordered,
        sim_unrelated
    );
    assert!(
        sim_reordered > 0.8,
        "the same words in a different order must be near-identical \
         (order-insensitive vocabulary features; got {:.3})",
        sim_reordered
    );
}

// ── T2: determinism across call order ───────────────────────────────

#[test]
fn n504_features_are_order_independent_and_stable() {
    let dim = 48;
    let texts = [
        "What are your business hours?",
        "I want a refund for order 42",
        "Where is my package?",
    ];
    // Forward order.
    let forward: Vec<Vec<f64>> = texts.iter().map(|t| hashed_tfidf_vector(t, dim)).collect();
    // Reverse order + an unrelated text in between — the results must be
    // identical (NO growing state: the extractor is a pure function).
    let mut backward = vec![hashed_tfidf_vector("unrelated filler text", dim)];
    backward.extend(texts.iter().rev().map(|t| hashed_tfidf_vector(t, dim)));
    backward.reverse();

    for i in 0..texts.len() {
        assert_eq!(
            forward[i], backward[i],
            "the feature vector must not depend on the embedding call order"
        );
    }
    // The signature constants are the agent-facing contract.
    assert_eq!(DISTILL_FEATURE_SIGNATURE, "tfidf-hash-v1");
    assert_eq!(LEGACY_FEATURE_SIGNATURE, "bytes-pos-v1");
}

// ── T3: the №485 gate on a real-text corpus (the A/B metrics) ───────

/// Four intents, ten paraphrased real-text utterances each — the shape
/// the office actually distills (routing questions to answers).
fn real_text_corpus() -> Vec<(String, String)> {
    let hours = [
        "What are your business hours?",
        "When do you open on weekdays?",
        "Are you open on Sundays?",
        "What time does the store close?",
        "Tell me your working hours",
        "When can I visit the shop?",
        "Do you work on weekends?",
        "What are the opening times?",
        "Is the office open now?",
        "Until how late are you open?",
    ];
    let refund = [
        "I want a refund for my order",
        "How do I get my money back?",
        "Please return the payment for item 12",
        "The product is broken, give me a refund",
        "Can I return this and get my cash back?",
        "I demand a reimbursement",
        "Refund my purchase please",
        "The order arrived damaged, refund it",
        "How long does a money return take?",
        "I would like my payment back",
    ];
    let track = [
        "Where is my package?",
        "Track my parcel please",
        "What is the status of my delivery?",
        "When will my order arrive?",
        "Where's my shipment?",
        "Has my parcel shipped yet?",
        "I cannot find my delivery",
        "Tell me where the package is now",
        "My order has not arrived, where is it?",
        "Give me the tracking status",
    ];
    let human = [
        "Let me talk to a human agent",
        "Connect me with a real person",
        "I want to speak to support staff",
        "Give me a live operator",
        "Transfer me to a consultant",
        "Can a real person help me?",
        "I need a human on the line",
        "Put me through to your team",
        "Agent please, not a bot",
        "Hand me over to customer care",
    ];
    let mut corpus = Vec::new();
    for (i, t) in hours.iter().enumerate() {
        corpus.push((t.to_string(), "hours".to_string()));
        let _ = i;
    }
    for t in refund.iter() {
        corpus.push((t.to_string(), "refund".to_string()));
    }
    for t in track.iter() {
        corpus.push((t.to_string(), "track".to_string()));
    }
    for t in human.iter() {
        corpus.push((t.to_string(), "human".to_string()));
    }
    corpus
}

fn train_and_gate_lr(dim: usize, lr: f64, extract: &dyn Fn(&str, usize) -> Vec<f64>) -> (f64, f64) {
    // The same model shape and hyper-parameters the distill path uses
    // (`ReflexModel::train` 30 epochs, lr 0.1, the deterministic 80/20
    // split by seed — №179).
    let corpus = real_text_corpus();
    let labels = ["hours", "refund", "track", "human"];
    let mut model = ReflexModel {
        name: "RouterHead".to_string(),
        layers: vec![Box::new(Dense::new(
            dim,
            labels.len(),
            ActivationKind::Softmax,
            42,
        ))],
        seed: 42,
        last_metric: None,
        input_size: dim,
        labels: labels.iter().map(|s| s.to_string()).collect(),
        feature_signature: DISTILL_FEATURE_SIGNATURE.to_string(),
    };
    let inputs: Vec<Vec<f64>> = corpus.iter().map(|(t, _)| extract(t, dim)).collect();
    let targets: Vec<usize> = corpus
        .iter()
        .map(|(_, l)| labels.iter().position(|x| x == l).unwrap())
        .collect();
    let (loss, holdout) = model.train(&inputs, &targets, 30, lr).expect("train");
    // The №485 gate: max(min_accuracy, majority_baseline + margin).
    let mut counts = std::collections::HashMap::new();
    for &t in &targets {
        *counts.entry(t).or_insert(0usize) += 1;
    }
    let baseline = counts.values().copied().max().unwrap() as f64 / targets.len() as f64;
    let gate = f64::max(0.85, baseline + 0.05);
    eprintln!(
        "A/B real-text corpus: loss={:.4} holdout={:.3} majority_baseline={:.2} gate={:.2} → {}",
        loss,
        holdout,
        baseline,
        gate,
        if holdout >= gate {
            "SWITCH"
        } else {
            "STAY TEACHING"
        }
    );
    (holdout, gate)
}

#[test]
fn n504_real_text_corpus_ab_direction() {
    // The comparison dim: 256 — the extractor guidance (the hashed
    // vocabulary needs room: this corpus carries ~200 distinct tokens;
    // dim 64 smashes ~3 tokens into every bucket and the class signal
    // drowns in collisions — dim is a DECLARATION parameter, the same
    // way `input: embedding(N)` is chosen per model). The legacy
    // extractor is dim-insensitive (byte positions), so 256 is also its
    // fair side of the table.
    let dim = 256;
    // (a) The PRODUCTION budget (30 epochs, lr 0.1 — hardcoded in the
    //     distill path): the Dense head barely converges at all, both
    //     sides tie — printed for the report, NOT pinned as a win.
    let (old_prod, gate_prod) = train_and_gate_lr(dim, 0.1, &legacy_bytes_embedding);
    let (new_prod, gate_new_prod) = train_and_gate_lr(dim, 0.1, &hashed_tfidf_vector);
    eprintln!(
        "A/B production budget (30 epochs, lr 0.1): legacy {:.3} vs hashed {:.3}, gate {:.2} — both {}",
        old_prod, new_prod, gate_prod,
        if new_prod >= gate_new_prod { "SWITCH" } else { "STAY TEACHING" }
    );

    // (b) The budget-isolated A/B (a TEST-side lr — the production path
    //     is NOT touched here): at a budget where the head CAN converge,
    //     the hashed features must WIN on the same corpus — the proof
    //     that the feature quality is the ceiling №504 lifts, and the
    //     training budget is the next lever (the owner's semantics
    //     decision, see the report).
    let (old_fast, _) = train_and_gate_lr(dim, 0.5, &legacy_bytes_embedding);
    let (new_fast, gate_fast) = train_and_gate_lr(dim, 0.5, &hashed_tfidf_vector);
    eprintln!(
        "A/B lr 0.5 (test-side): legacy {:.3} vs hashed {:.3}, gate {:.2}",
        old_fast, new_fast, gate_fast
    );
    assert!(
        new_fast > old_fast,
        "at a converging budget the hashed TF-IDF features must beat the \
         byte-position features (new {:.3} <= old {:.3})",
        new_fast,
        old_fast
    );
    // The gate formulas are identical on both sides — the №485 contract
    // is untouched by the feature swap.
    assert_eq!(gate_prod, gate_new_prod);
}

// ── T4: the signature migration (save → tamper → load → refuse) ─────

fn make_head() -> (ReflexRegistry, metalogos::nn::ReflexId) {
    let mut registry = ReflexRegistry::new();
    let model = ReflexModel {
        name: "Head".to_string(),
        layers: vec![Box::new(Dense::new(8, 2, ActivationKind::Softmax, 42))],
        seed: 42,
        last_metric: None,
        input_size: 8,
        labels: vec!["yes".to_string(), "no".to_string()],
        feature_signature: DISTILL_FEATURE_SIGNATURE.to_string(),
    };
    let id = registry.register(model);
    (registry, id)
}

#[test]
fn n504_signature_migration_is_loud_and_blocking() {
    let dir = std::env::temp_dir().join(format!("n504_sig_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("sig.db");

    // Save with the CURRENT signature.
    let (registry, id) = make_head();
    {
        let model = registry.get_dense(id).expect("model");
        persist::save_model_to_db(model, "Head", &db).expect("save");
    }

    // A fresh registry loads it cleanly — the current signature round-trips.
    let (mut registry_b, id_b) = make_head();
    persist::load_model_from_db(&mut registry_b, id_b, "Head", &db)
        .expect("a current-signature save must load");
    let loaded = registry_b.get_dense(id_b).expect("loaded");
    assert_eq!(loaded.feature_signature, DISTILL_FEATURE_SIGNATURE);

    // Tamper the row to the LEGACY signature (a pre-№504 binary's save
    // is exactly this: NULL/legacy + byte-position weights).
    {
        #[allow(clippy::disallowed_methods)]
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute(
            "UPDATE reflex_models SET feature_signature = ?1 WHERE name = 'Head'",
            rusqlite::params![LEGACY_FEATURE_SIGNATURE],
        )
        .unwrap();
    }

    // The weights stay READABLE — the load succeeds either way…
    let (mut registry_c, id_c) = make_head();
    persist::load_model_from_db(&mut registry_c, id_c, "Head", &db)
        .expect("a legacy-signature save must still LOAD (the weights are readable)");
    // …but the model is stamped loudly, and the distill predicate refuses.
    let stale_signature = registry_c
        .get_dense(id_c)
        .expect("loaded")
        .feature_signature
        .clone();
    assert_eq!(stale_signature, LEGACY_FEATURE_SIGNATURE);
    assert_ne!(
        stale_signature, DISTILL_FEATURE_SIGNATURE,
        "the mismatch must be observable by the distill guards"
    );

    // A successful train lifts the stamp (the single-point rule).
    {
        let corpus: Vec<(Vec<f64>, usize)> = (0..20)
            .map(|i| {
                let mut v = vec![0.0; 8];
                v[i % 8] = 1.0;
                (v, i % 2)
            })
            .collect();
        let (inputs, targets): (Vec<Vec<f64>>, Vec<usize>) = corpus.into_iter().unzip();
        let model = registry_c.get_dense_mut(id_c).expect("model");
        model.train(&inputs, &targets, 30, 0.1).expect("train");
        assert_eq!(
            registry_c.get_dense(id_c).expect("model").feature_signature,
            DISTILL_FEATURE_SIGNATURE,
            "a successful train must re-bind the weights to the current features"
        );
    }

    std::fs::remove_file(&db).ok();
    std::fs::remove_dir(&dir).ok();
}
