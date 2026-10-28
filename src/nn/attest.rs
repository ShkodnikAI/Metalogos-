//! Reflex attestation (Наряд №825, ADR-0200 — the provable tier-lowering line, W1).
//!
//! A certificate carries the **provenance and consistency** facts of a
//! trained Reflex model: which pattern produced it, which spec shape, which
//! training data, which embedding signature, which weight bytes — anchored
//! into the action-ledger chain (`reflex.attest` records carrying
//! `cert_sha256`; the ledger's Ed25519 chain makes the anchor
//! tamper-EVIDENT, not tamper-PROOF — ADR-0167's boundary, verbatim).
//!
//! ## What the certificate is NOT (ADR-0200 §2, honest limits)
//!
//! It is not a quality proof. `ok: true` means provenance consistency,
//! never "the model is good". Every eval value is `uncalibrated` until E3
//! measures calibration. Nothing here claims "calibrated" or
//! "proven-correct".
//!
//! ## Three hashes, three refusal codes (ADR-0200 §4)
//!
//! - `spec_hash` over the pattern signature (normalized prompt, labels,
//!   distill config, metric, threshold; the declaration SHAPE — comments,
//!   formatting and `cache*` keys never enter: they are not part of the
//!   compiled pattern) → `[ATTESTATION_STALE]`, refusal, the changed
//!   field is named;
//! - `data_hash` over the training-set manifest →
//!   `[ATTESTATION_DATA_CHANGED]`, informational, NOT a refusal;
//! - `embed_signature` over the feature-extractor signature →
//!   `[ATTESTATION_EMBED_MISMATCH]`, hard refusal.
//!
//! A weights-byte mismatch (`weights_sha256`) is a fourth, always-loud
//! refusal: `[ATTESTATION_WEIGHTS_CHANGED]`.
//!
//! ## The changed-field name
//!
//! The `[ATTESTATION_STALE]` reason names the changed field — which
//! requires the OLD canonical spec at verify time. The history table
//! therefore stores one extra column, `spec_json` (the exact canonical
//! string the hash covers, per row). The certificate JSON itself keeps
//! the ADR-0200 §4 key set verbatim — the snapshot lives in the table,
//! not in the cert.
//!
//! ## Transitional window (owner decision 4, variant б)
//!
//! Weights saved before W1 carry no certificate: they load with a loud
//! warning and `cert: none`; an `unsigned` certificate (no configured
//! ledger key) loads with a loud warning. `METALOGOS_ATTEST_STRICT=1`
//! turns both cases into refusals. The default flips to strict one release
//! window after W1 ships (the CHANGELOG Migration entry is part of the
//! contract).
//!
//! ## Ledger failure at save
//!
//! A ledger write failure at save does NOT silently cancel or degrade the
//! save — it is a loud error (ADR-0200 §4: the attestation is an artifact,
//! not a side action; the ADR-0167 best-effort asymmetry for ACTIONS does
//! not apply here). The save transaction rolls back.

use crate::nn::ReflexModel;
use crate::vision::provenance::sha256_hex;

/// Certificate schema version (bump on any shape change).
pub const ATTESTATION_VERSION: i64 = 1;

/// Refusal / informational codes (ADR-0200 §4 verbatim forms).
pub const CODE_STALE: &str = "[ATTESTATION_STALE]";
pub const CODE_DATA_CHANGED: &str = "[ATTESTATION_DATA_CHANGED]";
pub const CODE_EMBED_MISMATCH: &str = "[ATTESTATION_EMBED_MISMATCH]";
pub const CODE_WEIGHTS_CHANGED: &str = "[ATTESTATION_WEIGHTS_CHANGED]";
pub const CODE_NONE: &str = "[ATTESTATION_NONE]";
pub const CODE_UNSIGNED: &str = "[ATTESTATION_UNSIGNED]";

/// `METALOGOS_ATTEST_STRICT=1` — the absence of a certificate and an
/// `unsigned` certificate become load refusals (owner decision 4).
pub const ENV_STRICT: &str = "METALOGOS_ATTEST_STRICT";
/// `METALOGOS_ATTEST_ALLOW_STALE=1` — the serve-path stale bypass (a
/// loud audit line + a ledger record every time it is used).
pub const ENV_ALLOW_STALE: &str = "METALOGOS_ATTEST_ALLOW_STALE";

/// The pattern-side facts the caller (TW interpreter or VM) extracts from
/// its own compiled learnable pattern. `None` fields are `null` WITH a
/// reason in the certificate — never invented (ADR-0200 §4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttestSpecInput {
    /// The learnable pattern that distills into this model, if any.
    pub pattern_name: Option<String>,
    /// The pattern's prompt (comments and formatting are not part of the
    /// compiled pattern node — they cannot enter the hash by construction).
    pub prompt: Option<String>,
    pub distill_after: Option<usize>,
    /// `fallback_if: confidence OP value` — the operator as the canonical
    /// lowercase word ("lt" | "le" | "gt" | "ge" | "eq" | "ne").
    pub fallback_if: Option<(String, f64)>,
    /// №456: the holdout gate minimum (the RESOLVED value — the caller
    /// applies the 0.85 default before building the input).
    pub min_accuracy: Option<f64>,
    /// №485: the majority-baseline margin (resolved, 0.05 default).
    pub margin: Option<f64>,
}

/// Everything the save/verify path needs from the caller: the spec input
/// plus the pattern's current teaching examples (the training-set manifest
/// source in the hub-less lanes; the hub lane's persistent
/// `distill_samples` rows are read from the same DB when the caller has
/// none).
#[derive(Debug, Clone, Default)]
pub struct AttestInput {
    pub spec: AttestSpecInput,
    pub examples: Vec<(String, String)>,
}

/// Load/save verification policy. Read from the environment at the
/// dispatch boundary; tests pass explicit values (no env races).
#[derive(Debug, Clone, Copy)]
pub struct AttestPolicy {
    pub strict: bool,
    pub allow_stale: bool,
}

impl AttestPolicy {
    /// The lenient transitional default (owner decision 4, variant б).
    pub fn lenient() -> Self {
        AttestPolicy {
            strict: false,
            allow_stale: false,
        }
    }

    pub fn from_env() -> Self {
        AttestPolicy {
            strict: std::env::var(ENV_STRICT)
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
            allow_stale: std::env::var(ENV_ALLOW_STALE)
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
        }
    }
}

/// The verify verdict — the same shape feeds `mlog attest verify` (CLI),
/// the `attest_verify` builtin (Struct) and the `reflex_load` gate.
#[derive(Debug, Clone)]
pub struct AttestVerdict {
    pub ok: bool,
    /// Output level (ADR-0200 §4 + owner decision 5): the DATA level
    /// dominates while every W1 dataset is unfiltered — `unfiltered`;
    /// `signed` / `unsigned` are the certificate's own anchor levels and
    /// surface once W4 introduces filtered datasets. `none` — no
    /// certificate row exists.
    pub level: String,
    /// The CURRENT spec hash (recomputed from the live declarations).
    pub spec_hash: String,
    /// Human-readable reason: a refusal code + detail, an informational
    /// code, or "" when everything matched.
    pub reason: String,
    /// The certificate JSON itself ("" when absent).
    pub cert_json: String,
}

// ── The table (history, append-only) ─────────────────────────────────

pub const REFLEX_ATTESTATION_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS reflex_attestation (
  seq             INTEGER PRIMARY KEY AUTOINCREMENT,
  model           TEXT NOT NULL,
  spec_hash       TEXT NOT NULL,
  data_hash       TEXT NOT NULL,
  embed_signature TEXT NOT NULL,
  weights_sha256  TEXT NOT NULL,
  cert_json       TEXT NOT NULL,
  ledger_seq      INTEGER,
  created_at      INTEGER NOT NULL,
  superseded      INTEGER NOT NULL DEFAULT 0,
  spec_json       TEXT NOT NULL DEFAULT ''
);
";

pub fn ensure_attestation_table(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(REFLEX_ATTESTATION_SCHEMA).map_err(|e| {
        format!(
            "reflex attest: failed to ensure reflex_attestation table: {}",
            e
        )
    })
}

// ── Canonical spec string + spec_hash ────────────────────────────────

/// The canonical layer descriptor — the declaration SHAPE (type name +
/// dims). Activation kind is deliberately excluded: it is recoverable at
/// deserialize and `persist.rs` (Block 3) documents an activation swap as
/// "the architecture didn't change". Weight BYTES are the
/// `weights_sha256` lane, never the spec lane.
fn layer_descriptors(model: &ReflexModel) -> Vec<String> {
    model
        .layers
        .iter()
        .map(|l| format!("{}({},{})", l.name(), l.input_size(), l.output_size()))
        .collect()
}

#[derive(serde::Serialize)]
struct SpecSer<'a> {
    pattern: &'a Option<String>,
    prompt: &'a Option<String>,
    distill_after: &'a Option<usize>,
    fallback_if: &'a Option<(String, f64)>,
    min_accuracy: &'a Option<f64>,
    margin: &'a Option<f64>,
    reflex: &'a str,
    input_size: usize,
    labels: &'a [String],
    seed: u64,
    layers: Vec<String>,
    metric: &'static str,
}

/// The only registered gating metric today (№826 widens the registry;
/// the literal is the honest current value, not a guess).
const SPEC_METRIC: &str = "accuracy";

fn spec_ser<'a>(
    reflex_name: &'a str,
    input_size: usize,
    labels: &'a [String],
    seed: u64,
    layers: Vec<String>,
    spec: &'a AttestSpecInput,
) -> SpecSer<'a> {
    SpecSer {
        pattern: &spec.pattern_name,
        prompt: &spec.prompt,
        distill_after: &spec.distill_after,
        fallback_if: &spec.fallback_if,
        min_accuracy: &spec.min_accuracy,
        margin: &spec.margin,
        reflex: reflex_name,
        input_size,
        labels,
        seed,
        layers,
        metric: SPEC_METRIC,
    }
}

/// The canonical spec representation: JSON with a fixed field order
/// (machine-built — serde serializes struct fields in declaration order).
pub fn canonical_spec_json(model: &ReflexModel, spec: &AttestSpecInput) -> String {
    serde_json::to_string(&spec_ser(
        &model.name,
        model.input_size,
        &model.labels,
        model.seed,
        layer_descriptors(model),
        spec,
    ))
    .unwrap_or_else(|_| "{}".to_string())
}

/// Canonical spec JSON from raw declaration parts (the CLI lane — no
/// runtime model exists).
pub fn canonical_spec_from_parts(
    reflex_name: &str,
    input_size: usize,
    labels: &[String],
    seed: u64,
    layers: &[String],
    spec: &AttestSpecInput,
) -> String {
    serde_json::to_string(&spec_ser(
        reflex_name,
        input_size,
        labels,
        seed,
        layers.to_vec(),
        spec,
    ))
    .unwrap_or_else(|_| "{}".to_string())
}

pub fn spec_hash(model: &ReflexModel, spec: &AttestSpecInput) -> String {
    sha256_hex(canonical_spec_json(model, spec).as_bytes())
}

// ── Data manifest + data_hash ────────────────────────────────────────

/// The training-set manifest. The caller's examples (the live teaching set
/// the training consumed) take precedence; without them the persistent
/// `distill_samples` rows for the pattern are the manifest (the hub lane's
/// persistent ledger — the test-7 route); with neither, the manifest is
/// empty and honest about it (`provenance = "none"`).
pub struct DataManifest {
    pub rows: usize,
    /// "runtime-teaching" | "distill_samples" | "none".
    pub provenance: String,
    pub hash: String,
}

pub fn data_manifest(
    conn: &rusqlite::Connection,
    pattern_name: Option<&str>,
    examples: &[(String, String)],
) -> DataManifest {
    let mut lines: Vec<String> = Vec::new();
    let provenance: String;
    if !examples.is_empty() {
        provenance = "runtime-teaching".to_string();
        for (input, output) in examples {
            lines.push(
                serde_json::to_string(&serde_json::json!({
                    "input": input,
                    "output": output
                }))
                .unwrap_or_default(),
            );
        }
    } else if let Some(pattern) = pattern_name {
        provenance = "distill_samples".to_string();
        for (input, output) in read_distill_samples(conn, pattern) {
            lines.push(
                serde_json::to_string(&serde_json::json!({
                    "input": input,
                    "output": output
                }))
                .unwrap_or_default(),
            );
        }
    } else {
        provenance = "none".to_string();
    }
    let hash = sha256_hex(lines.join("\n").as_bytes());
    DataManifest {
        rows: lines.len(),
        provenance,
        hash,
    }
}

/// Read the persisted teaching rows for one pattern. A missing table is
/// an empty set (the hub-less lane never creates one) — NOT an error.
fn read_distill_samples(conn: &rusqlite::Connection, pattern: &str) -> Vec<(String, String)> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT input, output FROM distill_samples WHERE pattern_name = ?1 ORDER BY rowid",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return Vec::new();
    };
    rows.filter_map(|r| r.ok()).collect()
}

// ── Certificate construction ─────────────────────────────────────────

fn cert_level_word(ledger_seq: Option<i64>) -> (&'static str, bool) {
    match ledger_seq {
        Some(_) => {
            let signed = std::env::var(crate::ledger::LEDGER_KEY_ENV).is_ok();
            (if signed { "signed" } else { "unsigned" }, signed)
        }
        None => ("unsigned", false),
    }
}

/// Build the canonical certificate JSON (keys sorted — a serde_json Map
/// is a BTreeMap by default, so insertion order does not matter).
/// Unknown facts are `null` WITH a sibling `*_reason` key — never
/// invented (ADR-0200 §4).
pub fn build_certificate(
    model: &ReflexModel,
    spec: &AttestSpecInput,
    manifest: &DataManifest,
    s_hash: &str,
    weights_sha: &str,
    ledger_seq: Option<i64>,
) -> serde_json::Value {
    let (level, signed) = cert_level_word(ledger_seq);

    let mut cert = serde_json::Map::new();
    cert.insert(
        "attestation_version".into(),
        serde_json::json!(ATTESTATION_VERSION),
    );
    cert.insert("model".into(), serde_json::json!(model.name));
    match &spec.pattern_name {
        Some(p) => {
            cert.insert("pattern".into(), serde_json::json!(p));
        }
        None => {
            cert.insert("pattern".into(), serde_json::Value::Null);
            cert.insert(
                "pattern_reason".into(),
                serde_json::json!(
                    "no learnable pattern distills into this model (direct reflex_train)"
                ),
            );
        }
    }
    cert.insert("spec_hash".into(), serde_json::json!(s_hash));
    cert.insert("data_hash".into(), serde_json::json!(manifest.hash));
    cert.insert(
        "embed_signature".into(),
        serde_json::json!(model.feature_signature),
    );
    cert.insert("weights_sha256".into(), serde_json::json!(weights_sha));
    // Teacher facts land with the teacher registry (W3b) — null with the
    // reason; decision 3 (open models with a permissive license) becomes
    // machine-checkable there.
    cert.insert("teacher".into(), serde_json::Value::Null);
    cert.insert(
        "teacher_reason".into(),
        serde_json::json!(
            "the teacher registry (W3b) does not exist yet; decision 3 applies from W3b"
        ),
    );
    // Dataset: filter is "none" until W4 (verbatim ADR-0200 §4).
    let provenance: Vec<String> = if manifest.provenance == "none" {
        vec![]
    } else {
        vec![manifest.provenance.clone()]
    };
    cert.insert(
        "dataset".into(),
        serde_json::json!({
            "rows": manifest.rows,
            "filter": "none",
            "provenance": provenance,
        }),
    );
    // eval: last_metric is the last TRAIN metric (the holdout accuracy
    // when the distill gate passed); None for a fresh model — an obvious
    // null, no reason key needed.
    cert.insert(
        "eval".into(),
        serde_json::json!({
            "held_out_accuracy": model.last_metric,
            "metric": SPEC_METRIC,
            "calibration": "uncalibrated",
        }),
    );
    // rollback_if: the mutate_rollback rules are session-executed
    // statements (MutateDecl), not stored model state — there is nothing
    // machine-true to write at save time. Null with the reason.
    cert.insert("rollback_if".into(), serde_json::Value::Null);
    cert.insert(
        "rollback_if_reason".into(),
        serde_json::json!(
            "mutate_rollback rules are session-executed (MutateDecl), not stored model state; the capture lands with the W5 state machine"
        ),
    );
    cert.insert(
        "toolchain".into(),
        serde_json::json!({
            "reflex_version": crate::nn::serde_weights::REFLEX_VERSION,
            "mlog_version": env!("CARGO_PKG_VERSION"),
        }),
    );
    cert.insert("ledger_seq".into(), serde_json::json!(ledger_seq));
    cert.insert("level".into(), serde_json::json!(level));
    if ledger_seq.is_none() {
        cert.insert(
            "level_reason".into(),
            serde_json::json!("the reflex.attest ledger record could not anchor this certificate"),
        );
    }
    let _ = signed;
    serde_json::Value::Object(cert)
}

/// The weights hash — over the EXACT storage bytes
/// (`serialize_for_storage` output, the same bytes the BLOB carries).
pub fn weights_sha256(model: &ReflexModel) -> String {
    sha256_hex(&crate::nn::persist::serialize_for_storage(model))
}

// ── Save-side write ──────────────────────────────────────────────────

/// The save-side attestation write. Runs INSIDE the caller's transaction:
/// ledger append (loud failure = loud save error, the artifact rule) →
/// supersede the previous row → insert the new one (with the `spec_json`
/// snapshot the changed-field contract consumes). Returns the cert JSON.
pub fn write_attestation(
    conn: &rusqlite::Connection,
    model: &ReflexModel,
    attest: &AttestInput,
) -> Result<String, String> {
    ensure_attestation_table(conn)?;

    // 1. The ledger anchor FIRST — a failure here is a loud save error
    //    (ADR-0200 §4: the attestation is an artifact, never a silent
    //    side effect).
    let s_hash = spec_hash(model, &attest.spec);
    let weights_sha = weights_sha256(model);
    let manifest = data_manifest(conn, attest.spec.pattern_name.as_deref(), &attest.examples);
    let provisional =
        build_certificate(model, &attest.spec, &manifest, &s_hash, &weights_sha, None);
    let provisional_str = serde_json::to_string(&provisional)
        .map_err(|e| format!("reflex attest: certificate serialize: {}", e))?;
    let cert_sha = sha256_hex(provisional_str.as_bytes());
    let record = crate::ledger::append_record(
        "action",
        "runtime",
        "reflex.attest",
        &model.name,
        &cert_sha,
        None,
    )
    .map_err(|e| {
        format!(
            "reflex_save: attestation ledger write failed: {} — the weights are NOT saved \
             (the attestation is the artifact; ADR-0200 §4: a loud error, never a silent save)",
            e
        )
    })?;

    // 2. The certificate WITH the ledger seq (the anchor is part of the
    //    stored cert).
    let cert = build_certificate(
        model,
        &attest.spec,
        &manifest,
        &s_hash,
        &weights_sha,
        Some(record.seq as i64),
    );
    let cert_json = serde_json::to_string(&cert)
        .map_err(|e| format!("reflex attest: certificate serialize: {}", e))?;
    let spec_json = canonical_spec_json(model, &attest.spec);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 3. Supersede the previous current row, insert the new one.
    conn.execute(
        "UPDATE reflex_attestation SET superseded = 1 WHERE model = ?1 AND superseded = 0",
        rusqlite::params![model.name],
    )
    .map_err(|e| format!("reflex attest: supersede previous: {}", e))?;
    conn.execute(
        "INSERT INTO reflex_attestation \
         (model, spec_hash, data_hash, embed_signature, weights_sha256, cert_json, ledger_seq, created_at, superseded, spec_json) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9)",
        rusqlite::params![
            model.name,
            s_hash,
            manifest.hash,
            model.feature_signature,
            weights_sha,
            cert_json,
            record.seq as i64,
            now,
            spec_json,
        ],
    )
    .map_err(|e| format!("reflex attest: insert row: {}", e))?;

    Ok(cert_json)
}

// ── Verify side ──────────────────────────────────────────────────────

/// The stored certificate row (the model's current = last non-superseded).
pub struct StoredCert {
    pub seq: i64,
    pub spec_hash: String,
    pub data_hash: String,
    pub embed_signature: String,
    pub weights_sha256: String,
    pub cert_json: String,
    pub ledger_seq: Option<i64>,
    pub spec_json: String,
}

pub fn current_cert(
    conn: &rusqlite::Connection,
    model: &str,
) -> Result<Option<StoredCert>, String> {
    ensure_attestation_table(conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT seq, spec_hash, data_hash, embed_signature, weights_sha256, cert_json, ledger_seq, spec_json \
             FROM reflex_attestation WHERE model = ?1 AND superseded = 0 ORDER BY seq DESC LIMIT 1",
        )
        .map_err(|e| format!("reflex attest: prepare query: {}", e))?;
    let res = stmt.query_row(rusqlite::params![model], |row| {
        Ok(StoredCert {
            seq: row.get(0)?,
            spec_hash: row.get(1)?,
            data_hash: row.get(2)?,
            embed_signature: row.get(3)?,
            weights_sha256: row.get(4)?,
            cert_json: row.get(5)?,
            ledger_seq: row.get(6)?,
            spec_json: row.get(7)?,
        })
    });
    match res {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("reflex attest: query: {}", e)),
    }
}

/// Name the FIRST changed spec field (the `[ATTESTATION_STALE]`
/// contract): diff the stored canonical spec against the current one.
pub fn first_changed_field(old_spec_json: &str, new_spec_json: &str) -> String {
    let parsed: Option<(serde_json::Value, serde_json::Value)> = match (
        serde_json::from_str(old_spec_json),
        serde_json::from_str(new_spec_json),
    ) {
        (Ok(a), Ok(b)) => Some((a, b)),
        _ => None,
    };
    if let Some((serde_json::Value::Object(ma), serde_json::Value::Object(mb))) = parsed {
        for (k, va) in &ma {
            if mb.get(k) != Some(va) {
                return k.clone();
            }
        }
        for k in mb.keys() {
            if !ma.contains_key(k) {
                return k.clone();
            }
        }
    }
    "spec".to_string()
}

/// The full verify pass over an OPENED database connection.
///
/// `weights_blob` — the exact stored BLOB bytes (the hash must match the
/// certificate); `effective_signature` — the row's feature signature
/// (what the weights were trained against, the №504 resolution);
/// `current_canonical_spec` — the CURRENT canonical spec JSON (rebuilt
/// from the live declarations); `current_data_examples` — the caller's
/// live teaching set (or empty — the DB manifest is used).
#[allow(clippy::too_many_arguments)] // the ADR-0200 §4 verify order is the contract — each argument is one lane of it
pub fn verify_attestation(
    conn: &rusqlite::Connection,
    model_name: &str,
    weights_blob: &[u8],
    effective_signature: &str,
    current_canonical_spec: &str,
    current_s_hash: &str,
    current_data_examples: &[(String, String)],
    spec_pattern: Option<&str>,
    policy: AttestPolicy,
) -> Result<AttestVerdict, String> {
    let none_verdict = |reason: String| AttestVerdict {
        ok: false,
        level: "none".to_string(),
        spec_hash: current_s_hash.to_string(),
        reason,
        cert_json: String::new(),
    };

    let Some(stored) = current_cert(conn, model_name)? else {
        // Owner decision 4 (variant б): no certificate → loud warning +
        // `cert: none`, load CONTINUES; strict turns it into a refusal.
        if policy.strict {
            return Ok(none_verdict(format!(
                "{} no attestation certificate for model '{}' (saved before W1) — \
                 METALOGOS_ATTEST_STRICT=1 refuses uncertified weights",
                CODE_NONE, model_name
            )));
        }
        eprintln!(
            "[reflex] warning: model '{}' has no attestation certificate (saved before W1) — \
             loading with cert: none (owner decision 4, transitional; \
             METALOGOS_ATTEST_STRICT=1 turns this into a refusal)",
            model_name
        );
        return Ok(none_verdict(format!(
            "{} no attestation certificate for model '{}' (saved before W1)",
            CODE_NONE, model_name
        )));
    };

    let cert_level = stored
        .cert_json
        .parse::<serde_json::Value>()
        .ok()
        .and_then(|v| v.get("level").and_then(|l| l.as_str()).map(String::from))
        .unwrap_or_else(|| "unsigned".to_string());

    // The output level: the data level dominates while every W1 dataset
    // is unfiltered; signed/unsigned surface from W4 on.
    let level_for_output = |data_unfiltered: bool| -> String {
        if data_unfiltered {
            "unfiltered".to_string()
        } else {
            cert_level.clone()
        }
    };

    // 1. Weights bytes — always a hard refusal (tamper/corruption).
    let actual_weights = sha256_hex(weights_blob);
    if actual_weights != stored.weights_sha256 {
        return Ok(AttestVerdict {
            ok: false,
            level: level_for_output(false),
            spec_hash: current_s_hash.to_string(),
            reason: format!(
                "{} stored weights hash {} does not match the certificate {}",
                CODE_WEIGHTS_CHANGED,
                &actual_weights[..16.min(actual_weights.len())],
                &stored.weights_sha256[..16.min(stored.weights_sha256.len())],
            ),
            cert_json: stored.cert_json.clone(),
        });
    }

    // 2. Embedding signature — hard refusal.
    if effective_signature != stored.embed_signature {
        return Ok(AttestVerdict {
            ok: false,
            level: level_for_output(false),
            spec_hash: current_s_hash.to_string(),
            reason: format!(
                "{} weights were trained on feature signature '{}', the certificate pins '{}' \
                 — the extractor provenance disagrees",
                CODE_EMBED_MISMATCH, effective_signature, stored.embed_signature
            ),
            cert_json: stored.cert_json.clone(),
        });
    }

    // 3. spec_hash — refusal unless the explicit stale bypass is active.
    if current_s_hash != stored.spec_hash {
        let changed = first_changed_field(&stored.spec_json, current_canonical_spec);
        if !policy.allow_stale {
            return Ok(AttestVerdict {
                ok: false,
                level: level_for_output(false),
                spec_hash: current_s_hash.to_string(),
                reason: format!(
                    "{} the pattern spec changed since save: field '{}' — retrain and re-save \
                     (or bypass with --allow-stale / METALOGOS_ATTEST_ALLOW_STALE, loudly)",
                    CODE_STALE, changed
                ),
                cert_json: stored.cert_json.clone(),
            });
        }
        // The bypass is LOUD: stderr audit line + a ledger record.
        eprintln!(
            "[AUDIT] reflex.attest.allow-stale: model '{}' loaded with a STALE spec (field '{}' changed) — \
             the bypass is on (--allow-stale / METALOGOS_ATTEST_ALLOW_STALE)",
            model_name, changed
        );
        crate::ledger::record(
            "reflex.attest.allow-stale",
            "runtime",
            "reflex",
            &format!("model={};changed_field={}", model_name, changed),
        );
        return Ok(AttestVerdict {
            ok: true,
            level: level_for_output(true),
            spec_hash: current_s_hash.to_string(),
            reason: format!(
                "{} the pattern spec changed since save: field '{}' — the stale bypass is ACTIVE (audited)",
                CODE_STALE, changed
            ),
            cert_json: stored.cert_json.clone(),
        });
    }

    // 4. data_hash — informational, NOT a refusal (ADR-0200 §4).
    let manifest = data_manifest(conn, spec_pattern, current_data_examples);
    let mut reason = String::new();
    if manifest.hash != stored.data_hash {
        reason = format!(
            "{} the training-set manifest changed since save (informational, not a refusal)",
            CODE_DATA_CHANGED
        );
    }

    // 5. All three hashes agree — provenance consistency holds. The W1
    //    dataset filter is "none" by construction (the cert's own word
    //    is authoritative when present).
    let cert_filter = stored
        .cert_json
        .parse::<serde_json::Value>()
        .ok()
        .and_then(|v| {
            v.get("dataset")
                .and_then(|d| d.get("filter"))
                .and_then(|f| f.as_str())
                .map(String::from)
        });
    let data_unfiltered = cert_filter.map(|f| f == "none").unwrap_or(true);

    // 6. Owner decision 4 (variant б): an `unsigned` certificate (no
    //    configured ledger key at save — the per-process random key is
    //    verifiable only inside its own process) is a loud warning in the
    //    transitional window; METALOGOS_ATTEST_STRICT=1 makes it a
    //    refusal, exactly like the missing-certificate case.
    if cert_level == "unsigned" {
        if policy.strict {
            return Ok(AttestVerdict {
                ok: false,
                level: "unsigned".to_string(),
                spec_hash: current_s_hash.to_string(),
                reason: format!(
                    "{} the certificate is not anchored to a configured ledger \
                     (no {}) — METALOGOS_ATTEST_STRICT=1 refuses unanchored certificates",
                    CODE_UNSIGNED,
                    crate::ledger::LEDGER_KEY_ENV
                ),
                cert_json: stored.cert_json.clone(),
            });
        }
        eprintln!(
            "[reflex] warning: model '{}' carries an UNSIGNED certificate (no configured \
             ledger key at save) — loud warning, owner decision 4 (transitional; \
             METALOGOS_ATTEST_STRICT=1 refuses unanchored certificates)",
            model_name
        );
    }

    Ok(AttestVerdict {
        ok: true,
        level: level_for_output(data_unfiltered),
        spec_hash: current_s_hash.to_string(),
        reason,
        cert_json: stored.cert_json.clone(),
    })
}

/// Verify WITHOUT a loaded runtime model (the CLI `mlog attest verify`
/// route): the current declarations come from the parsed file, the
/// weights bytes from the DB row itself.
pub struct CliVerifyReport {
    pub ok: bool,
    pub level: String,
    pub spec_hash: String,
    pub reason: String,
    pub model: String,
}

pub fn verify_from_db_only(
    conn: &rusqlite::Connection,
    model_name: &str,
    canonical_spec: &str,
    s_hash: &str,
    policy: AttestPolicy,
) -> Result<CliVerifyReport, String> {
    let Some(stored) = current_cert(conn, model_name)? else {
        return Ok(CliVerifyReport {
            ok: false,
            level: "none".to_string(),
            spec_hash: s_hash.to_string(),
            reason: format!(
                "{} no attestation certificate for model '{}' (saved before W1)",
                CODE_NONE, model_name
            ),
            model: model_name.to_string(),
        });
    };
    // Weights: read the BLOB and hash it.
    let blob: Vec<u8> = {
        let mut stmt = conn
            .prepare("SELECT weights FROM reflex_models WHERE name = ?1")
            .map_err(|e| format!("reflex attest: prepare: {}", e))?;
        stmt.query_row(rusqlite::params![model_name], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(|e| {
            format!(
                "reflex attest: no stored weights for '{}': {}",
                model_name, e
            )
        })?
    };
    let actual = sha256_hex(&blob);
    if actual != stored.weights_sha256 {
        return Ok(CliVerifyReport {
            ok: false,
            level: "unsigned".to_string(),
            spec_hash: s_hash.to_string(),
            reason: format!("{} stored weights hash mismatch", CODE_WEIGHTS_CHANGED),
            model: model_name.to_string(),
        });
    }
    // Embedding: the row's stored feature signature (what the weights
    // were trained against) must match the certificate's pin.
    let row_signature: Option<String> = {
        let mut stmt = conn
            .prepare("SELECT feature_signature FROM reflex_models WHERE name = ?1")
            .map_err(|e| format!("reflex attest: prepare: {}", e))?;
        stmt.query_row(rusqlite::params![model_name], |row| {
            row.get::<_, Option<String>>(0)
        })
        .map_err(|e| format!("reflex attest: read signature: {}", e))?
    };
    let effective = row_signature
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| crate::embeddings::LEGACY_FEATURE_SIGNATURE.to_string());
    if effective != stored.embed_signature {
        return Ok(CliVerifyReport {
            ok: false,
            level: "unsigned".to_string(),
            spec_hash: s_hash.to_string(),
            reason: format!("{} embedding signature mismatch", CODE_EMBED_MISMATCH),
            model: model_name.to_string(),
        });
    }
    if s_hash != stored.spec_hash {
        let changed = first_changed_field(&stored.spec_json, canonical_spec);
        if !policy.allow_stale {
            return Ok(CliVerifyReport {
                ok: false,
                level: "unsigned".to_string(),
                spec_hash: s_hash.to_string(),
                reason: format!(
                    "{} the pattern spec changed since save: field '{}' — retrain and re-save \
                     (or bypass with --allow-stale, loudly)",
                    CODE_STALE, changed
                ),
                model: model_name.to_string(),
            });
        }
        crate::ledger::record(
            "reflex.attest.allow-stale",
            "runtime",
            "reflex",
            &format!("model={};lane=cli;changed_field={}", model_name, changed),
        );
        return Ok(CliVerifyReport {
            ok: true,
            level: "unfiltered".to_string(),
            spec_hash: s_hash.to_string(),
            reason: format!(
                "{} field '{}' changed — the stale bypass is ACTIVE (audited)",
                CODE_STALE, changed
            ),
            model: model_name.to_string(),
        });
    }
    // Data change — informational.
    let manifest = data_manifest(conn, None, &[]);
    let reason = if manifest.hash != stored.data_hash {
        format!(
            "{} the training-set manifest changed since save (informational, not a refusal)",
            CODE_DATA_CHANGED
        )
    } else {
        String::new()
    };
    Ok(CliVerifyReport {
        ok: true,
        level: "unfiltered".to_string(),
        spec_hash: s_hash.to_string(),
        reason,
        model: model_name.to_string(),
    })
}

/// The builtin's Struct payload (no new Value variants — ADR-0200 §6.4).
pub fn verdict_to_struct(verdict: &AttestVerdict) -> crate::interpreter::Value {
    use crate::interpreter::Value;
    use std::collections::HashMap;
    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("ok".to_string(), Value::Bool(verdict.ok));
    fields.insert("level".to_string(), Value::String(verdict.level.clone()));
    fields.insert(
        "spec_hash".to_string(),
        Value::String(verdict.spec_hash.clone()),
    );
    fields.insert("reason".to_string(), Value::String(verdict.reason.clone()));
    Value::Struct {
        type_name: "AttestVerdict".to_string(),
        fields,
    }
}
