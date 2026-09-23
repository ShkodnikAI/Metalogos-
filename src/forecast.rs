// ── Naryad #440 (P1, feature/forecast — the forecasting domain) ────────
//
// The forecast contour (plan v2 pattern IV — new domains = new opaque
// handle types + a backend-registry class, NOT new syntax):
//
//   SeriesHandle    — an opaque handle over a stored numeric series;
//                     carries the SOURCE taint label (the №322/№325
//                     lattice, ADR-0154) and the frequency word. The
//                     payload (the values) NEVER enters Value (the
//                     ADR-0114 opaque pattern; the registry is the
//                     state).
//   forecast_next(handle, horizon) -> ForecastHandle
//                   — walks the `timeseries` degradation ladder
//                     (ADR-0165 mechanics over the №333 registry):
//                     timesfm-2.5 (feature-gated off-process rung,
//                     Apache-2.0 weights pin) -> statsforecast
//                     (external software rung, not vendored in-tree —
//                     a DOCUMENTED dispatcher gap, Устав §11 Шаг 3,
//                     never a silent skip) -> seasonal_naive (the
//                     built-in deterministic rung, no dependencies).
//                     Every skipped rung is audited (stderr + the
//                     ledger record); the result carries the prov
//                     block {window hash, rung/pin, degraded flag,
//                     horizon} and the JOINED source label (LabelJoin
//                     transfer — a forecast derived from a tainted
//                     series is tainted).
//   ForecastHandle  — read-only, COPYABLE (a forecast is not an
//                     asset: no linearity — the Grant precedent is
//                     deliberately NOT followed). The prov block is
//                     projectable via forecast_state (metadata only);
//                     the DATA projection is the gated export surface:
//                     print/to_string/json_encode/forecast_points of
//                     a TAINTED forecast refuse fail-closed with the
//                     typed FORECAST_TAINTED stamp (the №413
//                     convention) and the refusal itself is a ledger
//                     record (forecast.denied — the №428 posture: no
//                     silent egress AND no silent refusal). A CLEAN
//                     forecast materializes its points+quantiles.
//
// Ledger: the `forecast.*` family (the embodied.*/memory.* template,
// contract №393/ADR-0167): forecast.series_make / forecast.run
// {window hash, rung, pin, quantiles-present, degraded, skipped} /
// forecast.denied / forecast.pull_denied. Ledger details carry HASHES
// and metadata only — never series or forecast values (they may be
// private; the №415 posture: digests, not payloads).

use crate::interpreter::values::Value;
use crate::labels::{Conf, Integrity, Label};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// The `timeseries` class ladder in priority order (the наряд's
/// degradation ladder). `forecast_next` walks it top-down; the first
/// AVAILABLE rung computes the forecast; every skipped rung is audited.
pub const TIMESERIES_LADDER: [&str; 3] = ["timesfm-2.5", "statsforecast", "seasonal_naive"];

/// The series capacity guard — a loud refusal, not a silent truncation
/// (the MAX_TRAJECTORY_POINTS convention).
pub const MAX_SERIES_LENGTH: usize = 100_000;

/// The horizon guard — a loud refusal outside the range (a horizon is
/// an allocation bound; unbounded horizons are a DoS surface).
pub const MAX_HORIZON: f64 = 1024.0;

// ── The registry state (the embodied.rs template: one mutex, two maps)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SeriesRecord {
    pub id: String,
    pub values: Vec<f64>,
    pub frequency: String,
    pub label: Label,
    /// Where the series came from: "list" (in-program literal data —
    /// genuinely public by construction), "struct" (the explicit form),
    /// or "pull:<source>" (the grant-gated external source word).
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct ForecastRecord {
    pub id: String,
    pub series_id: String,
    pub horizon: usize,
    pub points: Vec<f64>,
    pub p10: Vec<f64>,
    pub p50: Vec<f64>,
    pub p90: Vec<f64>,
    /// The ladder rung that computed the forecast.
    pub rung: String,
    /// The pin word of the rung's registry record (the №334 posture).
    pub pin: String,
    /// The rung note (e.g. "seasonal period reduced: window shorter
    /// than two seasons") — prov provenance, ledger-safe metadata.
    pub rung_note: String,
    /// TRUE when a higher-priority rung was skipped (the forecast is
    /// degraded relative to the class ladder's top rung).
    pub degraded: bool,
    /// sha256 over the canonical rendering of the input window (the
    /// №415 posture: the hash identifies the window, the values stay
    /// in the registry).
    pub window_hash: String,
    /// The JOINED source label (LabelJoin transfer from the series).
    pub label: Label,
    /// The skipped rungs in priority order (the audited gaps).
    pub skipped: Vec<String>,
}

#[derive(Default)]
pub struct ForecastState {
    pub series: HashMap<String, SeriesRecord>,
    pub forecasts: HashMap<String, ForecastRecord>,
}

fn state() -> &'static Mutex<ForecastState> {
    static REG: OnceLock<Mutex<ForecastState>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(ForecastState::default()))
}

static SEQ: AtomicU64 = AtomicU64::new(1);

fn fresh_id(prefix: &str, seed: &str) -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let preimage = format!("{}|{}|{}|{}", seed, millis, seq, prefix);
    format!(
        "{}-{}",
        prefix,
        &crate::ledger::sha256_hex(preimage.as_bytes())[..16]
    )
}

/// The Action-Ledger record (`forecast.*` family — the embodied.*
/// template, best-effort per ADR-0167 §2 driver 5).
fn ledger_forecast_event(kind: &str, id: &str, detail: &str) {
    eprintln!("[FORECAST_{}] {} {}", kind.to_uppercase(), id, detail);
    crate::ledger::record(&format!("forecast.{}", kind), id, "forecast", detail);
}

// ── The typed origin stamps (the №413 convention — position-0 markers
//    the `try` classifier branches on; whitelisted in values.rs). ──────

fn err_handle_unknown(fn_name: &str, kind: &str, id: &str) -> String {
    crate::interpreter::values::coded_error(
        crate::interpreter::values::CODE_FORECAST_HANDLE_UNKNOWN,
        format!(
            "{}: unknown {} handle '{}' — construct one with the forecast constructors first",
            fn_name, kind, id
        ),
    )
}

/// The taint export refusal — the shared engine of the print /
/// to_string / json_encode / forecast_points gates (the
/// deny_world_state_materialization template, taint-conditional).
pub fn deny_tainted_export(surface: &str, forecast_id: &str, label_word: &str) -> String {
    ledger_forecast_event(
        "denied",
        forecast_id,
        &format!("surface={}|reason=taint|label={}", surface, label_word),
    );
    crate::interpreter::values::coded_error(
        crate::interpreter::values::CODE_FORECAST_TAINTED,
        format!(
            "{} refused: the forecast carries a tainted source label ({}) — exporting derived data from a non-public series is an error (the №322/№325 lattice; branch on this code or re-derive from a public series)",
            surface, label_word
        ),
    )
}

// ── Label helpers ───────────────────────────────────────────────────────

/// The taint predicate: anything above the bottom label on the conf
/// axis, or below trusted on the integrity axis, taints the export.
pub fn is_tainted(label: &Label) -> bool {
    label.conf != Conf::Public || label.integrity != Integrity::Trusted
}

// ── The projection maps (the ADR-0114 opaque pattern: the map in the
//    Value is the printable PROJECTION; the registry is the state). ─────

fn series_map(rec: &SeriesRecord) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("id".to_string(), rec.id.clone());
    m.insert("frequency".to_string(), rec.frequency.clone());
    m.insert("label".to_string(), rec.label.to_string());
    m.insert("length".to_string(), rec.values.len().to_string());
    m.insert("source".to_string(), rec.source.clone());
    m
}

fn forecast_map(rec: &ForecastRecord) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("id".to_string(), rec.id.clone());
    m.insert("series_id".to_string(), rec.series_id.clone());
    m.insert("horizon".to_string(), rec.horizon.to_string());
    m.insert("rung".to_string(), rec.rung.clone());
    m.insert("pin".to_string(), rec.pin.clone());
    m.insert(
        "degraded".to_string(),
        if rec.degraded { "true" } else { "false" }.to_string(),
    );
    m.insert("window_hash".to_string(), rec.window_hash.clone());
    m.insert("label".to_string(), rec.label.to_string());
    m
}

// ── series_make ─────────────────────────────────────────────────────────

/// The parsed constructor source: the values plus the optional explicit
/// frequency/label words and the source-kind tag.
struct SeriesSourceParts {
    values: Vec<f64>,
    frequency: Option<String>,
    label: Option<String>,
    source: String,
}

/// Parse the numeric series payload out of the constructor's source
/// argument: a List[Float] (the literal form — genuinely public by
/// construction: the data is right there in the program text) or a
/// Struct {values: List[Float], frequency?: String, label?: String}
/// (the explicit form; the label word is parsed by the lattice —
/// over-tainting is allowed, UNDER-tainting is impossible: an unknown
/// word is a loud refusal, never a silent public).
fn parse_series_source(fn_name: &str, arg: &Value) -> Result<SeriesSourceParts, String> {
    const MAX: usize = MAX_SERIES_LENGTH;
    let parts = match arg {
        Value::List(items) => {
            let mut values = Vec::with_capacity(items.len());
            for it in items {
                match it {
                    Value::Float(f) if f.is_finite() => values.push(*f),
                    other => {
                        return Err(format!(
                            "{}: series values must be finite numbers, got {}",
                            fn_name,
                            other.type_name()
                        ))
                    }
                }
            }
            SeriesSourceParts {
                values,
                frequency: None,
                label: None,
                source: "list".to_string(),
            }
        }
        Value::Struct { fields, .. } => {
            let values = match fields.get("values") {
                Some(Value::List(items)) => {
                    let mut vs = Vec::with_capacity(items.len());
                    for it in items {
                        match it {
                            Value::Float(f) if f.is_finite() => vs.push(*f),
                            other => {
                                return Err(format!(
                                    "{}: series values must be finite numbers, got {}",
                                    fn_name,
                                    other.type_name()
                                ))
                            }
                        }
                    }
                    vs
                }
                Some(other) => {
                    return Err(format!(
                        "{}: struct form field 'values' must be a List of numbers, got {}",
                        fn_name,
                        other.type_name()
                    ))
                }
                None => {
                    return Err(format!(
                        "{}: struct form requires a 'values' field (List of numbers)",
                        fn_name
                    ))
                }
            };
            let frequency = match fields.get("frequency") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(other) => {
                    return Err(format!(
                        "{}: 'frequency' must be String, got {}",
                        fn_name,
                        other.type_name()
                    ))
                }
                None => None,
            };
            let label = match fields.get("label") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(other) => {
                    return Err(format!(
                        "{}: 'label' must be a lattice word String, got {}",
                        fn_name,
                        other.type_name()
                    ))
                }
                None => None,
            };
            SeriesSourceParts {
                values,
                frequency,
                label,
                source: "struct".to_string(),
            }
        }
        other => {
            return Err(format!(
                "{}: series source must be a List of numbers or a Struct {{values, frequency?, label?}}, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    if parts.values.is_empty() {
        return Err(format!(
            "{}: an EMPTY series cannot be forecast (a ladder without data cannot run — loud, never silent)",
            fn_name
        ));
    }
    if parts.values.len() > MAX {
        return Err(format!(
            "{}: series length {} exceeds the capacity guard {} (loud, not a silent truncation)",
            fn_name,
            parts.values.len(),
            MAX
        ));
    }
    Ok(parts)
}

/// `series_make(source, frequency?) -> SeriesHandle`. The handle
/// carries the source label (default bottom — public, trusted; an
/// explicit label word JOINS over the default, so over-tainting works
/// and under-tainting is impossible). Records forecast.series_make.
pub fn series_make(fn_name: &str, args: &[Value]) -> Result<Value, String> {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1 or 2 arguments (source, frequency?), got {}",
            fn_name,
            args.len()
        ));
    }
    let parts = parse_series_source(fn_name, &args[0])?;
    let values = parts.values;
    let source_kind = parts.source;
    let frequency = match args.get(1) {
        Some(Value::String(s)) => {
            if parts.frequency.is_some() {
                return Err(format!(
                    "{}: frequency given both positionally and in the struct form — pick one",
                    fn_name
                ));
            }
            s.clone()
        }
        Some(other) => {
            return Err(format!(
                "{}: frequency must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
        None => parts.frequency.unwrap_or_else(|| "unknown".to_string()),
    };
    let label = match &parts.label {
        Some(word) => Label::parse(word)
            .map_err(|e| format!("{}: invalid label '{}': {}", fn_name, word, e))?,
        None => Label::bottom(),
    };
    let id = fresh_id("series", &format!("{}|{}", values.len(), frequency));
    let rec = SeriesRecord {
        id: id.clone(),
        values,
        frequency: frequency.clone(),
        label: label.clone(),
        source: source_kind.clone(),
    };
    let map = series_map(&rec);
    let word = rec.label.to_string();
    let len = rec.values.len();
    {
        let mut reg = state().lock().map_err(|_| "forecast registry poisoned")?;
        reg.series.insert(id.clone(), rec);
    }
    ledger_forecast_event(
        "series_make",
        &id,
        &format!(
            "length={}|frequency={}|label={}|source={}",
            len, frequency, word, source_kind
        ),
    );
    Ok(Value::SeriesHandle(map))
}

/// `series_pull(grant, source) -> SeriesHandle` — the grant-gated
/// external pull surface. Honest boundary (loud, documented): this
/// build has NO external series source backend in-tree, so a pull can
/// never succeed; the surface exists to enforce the ORDER of the gate
/// (a pull without an ACTIVE grant refuses first — the grant check is
/// not skippable by the absence of a source) and to record the refusal
/// in the ledger (the №428 posture). The grant is NOT consumed on the
/// no-source refusal (nothing was pulled).
pub fn series_pull(fn_name: &str, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (grant, source), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle = match &args[0] {
        Value::Grant(g) => g.clone(),
        other => {
            return Err(format!(
                "{}: the pull is grant-gated — argument 1 must be a Grant handle, got {} (an ungated external source is a design error, not a degraded mode)",
                fn_name,
                other.type_name()
            ))
        }
    };
    let source = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: source must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    // The gate order: the grant MUST check active BEFORE anything else.
    if let Err(e) = crate::grants::check_active(&handle) {
        ledger_forecast_event(
            "pull_denied",
            &handle.grant_id,
            &format!("reason=grant|source={}", source),
        );
        return Err(format!("{}: grant check failed: {}", fn_name, e));
    }
    ledger_forecast_event(
        "pull_denied",
        &handle.grant_id,
        &format!("reason=no-source-backend|source={}", source),
    );
    Err(format!(
        "{}: no external series source backend is compiled in-tree (documented dispatcher gap — Устав §11 Шаг 3); the grant '{}' verified active, but there is nothing to pull from — derive the series in-program with series_make",
        fn_name, handle.grant_id
    ))
}

// ── The seasonal_naive rung (the built-in deterministic leg) ───────────

/// The canonical season period of a frequency word. Unknown words mean
/// period 1 (the naive leg) — documented, deterministic.
fn season_period(frequency: &str) -> usize {
    match frequency.trim().to_ascii_lowercase().as_str() {
        "hourly" => 24,
        "daily" => 7,
        "monthly" => 12,
        "quarterly" => 4,
        _ => 1,
    }
}

/// Nearest-rank empirical quantile over an ascending slice —
/// deterministic, no RNG, no external dependency.
fn nearest_rank(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[idx.clamp(1, sorted.len()) - 1]
}

/// The one-step seasonal-naive in-sample residuals -> the p10/p50/p90
/// offsets added to every forecast point (the honest uncertainty band
/// of the rung: the empirical distribution of its own errors).
fn residual_quantiles(values: &[f64], period: usize) -> ([f64; 3], String) {
    let mut residuals: Vec<f64> = Vec::new();
    if period >= 1 && values.len() > period {
        for i in period..values.len() {
            residuals.push(values[i] - values[i - period]);
        }
    }
    if residuals.is_empty() {
        return (
            [0.0, 0.0, 0.0],
            "no residuals (window <= one season)".to_string(),
        );
    }
    residuals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (
        [
            nearest_rank(&residuals, 10.0),
            nearest_rank(&residuals, 50.0),
            nearest_rank(&residuals, 90.0),
        ],
        String::new(),
    )
}

fn seasonal_naive_forecast(
    values: &[f64],
    horizon: usize,
    frequency: &str,
) -> (Vec<f64>, [f64; 3], String) {
    let nominal = season_period(frequency);
    let (period, note) = if values.len() >= 2 * nominal {
        (nominal, String::new())
    } else {
        (
            1,
            format!(
                "seasonal period reduced to 1: window ({}) shorter than two {}-seasons ({})",
                values.len(),
                frequency,
                nominal
            ),
        )
    };
    let last = values[values.len() - 1];
    let mut points = Vec::with_capacity(horizon);
    for h in 0..horizon {
        if period >= 1 && values.len() > period {
            let idx = values.len() - period + (h % period);
            points.push(values[idx]);
        } else {
            points.push(last);
        }
    }
    let (q, qnote) = residual_quantiles(values, period);
    let mut full_note = note;
    if !qnote.is_empty() {
        if !full_note.is_empty() {
            full_note.push_str("; ");
        }
        full_note.push_str(&qnote);
    }
    (points, q, full_note)
}

/// The canonical window rendering — the preimage of the window hash.
/// Rust's float Debug is the shortest round-trip form: identical values
/// render identically on both backends, so the hash is TW/VM-stable.
fn window_canonical(values: &[f64]) -> String {
    let parts: Vec<String> = values.iter().map(|v| format!("{:?}", v)).collect();
    parts.join(",")
}

// ── The ladder availability (the honest gates per rung) ────────────────

/// Why a rung is unavailable RIGHT NOW. `Ok(())` = available. The
/// reasons are loud, documented, and NEVER a silent skip (Устав §11
/// Шаг 3): every unavailable rung lands in the attempts trail and the
/// ledger record.
fn rung_unavailable_reason(rung: &str) -> Option<String> {
    match rung {
        "timesfm-2.5" => {
            // The off-process CPU-inference rung is feature-gated: the
            // CI-promo build never calls it (no network, no model), and
            // even with the feature compiled the №334 contract demands
            // the weights fetched and SHA-verified on disk first.
            if !cfg!(feature = "timesfm") {
                Some(
                    "feature-gated: the off-process timesfm backend is not compiled into this build (compile with --features timesfm)"
                        .to_string(),
                )
            } else {
                match crate::backends::find_by_name("timesfm-2.5") {
                    Some(entry) => match entry.pin {
                        crate::backends::ShaPin::Pinned(_) => {
                            match crate::backends_weights::first_manifest_file(entry.weights_id) {
                                Some(_) => None,
                                None => Some(format!(
                                    "real mode: weights ({}) not fetched and SHA-verified first (MLOG_BACKEND_WEIGHTS_ALLOWLIST + backends::fetch_weights)",
                                    entry.weights_id
                                )),
                            }
                        }
                        crate::backends::ShaPin::PendingNo334 => Some(
                            "weights manifest pending (№334 sha-pin path) — rung cannot be verified"
                                .to_string(),
                        ),
                    },
                    None => Some("no registry record (the №333 registry is the SSOT)".to_string()),
                }
            }
        }
        "statsforecast" => Some(
            "external software rung not vendored in-tree (documented dispatcher gap — Устав §11 Шаг 3)"
                .to_string(),
        ),
        "seasonal_naive" => None,
        other => Some(format!("unknown rung '{}' (no registry record)", other)),
    }
}

// ── forecast_next ───────────────────────────────────────────────────────

/// `forecast_next(handle, horizon) -> ForecastHandle`.
pub fn forecast_next(fn_name: &str, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (handle, horizon), got {}",
            fn_name,
            args.len()
        ));
    }
    let series_id = match &args[0] {
        Value::SeriesHandle(m) => m
            .get("id")
            .cloned()
            .ok_or_else(|| format!("{}: SeriesHandle has no id field", fn_name))?,
        other => {
            return Err(format!(
                "{}: argument 1 must be a SeriesHandle, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let horizon = match &args[1] {
        Value::Float(h) if h.is_finite() && *h >= 1.0 && h.fract() == 0.0 && *h <= MAX_HORIZON => {
            *h as usize
        }
        Value::Float(h) if h.is_finite() && h.fract() == 0.0 => {
            return Err(format!(
            "{}: horizon {} is outside the loud guard 1..={} (a horizon is an allocation bound)",
            fn_name, h, MAX_HORIZON as i64
        ))
        }
        other => {
            return Err(format!(
                "{}: horizon must be a whole number (Float), got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let forecast = {
        let mut reg = state().lock().map_err(|_| "forecast registry poisoned")?;
        let series = reg
            .series
            .get(&series_id)
            .cloned()
            .ok_or_else(|| err_handle_unknown(fn_name, "SeriesHandle", &series_id))?;
        // The LabelJoin transfer: the forecast derives from the series,
        // so its label is the JOIN of the source label (single source —
        // the identity join; poisoned absorbs, private stays private).
        let label = series.label.join(&Label::bottom());
        // The ladder walk: first AVAILABLE rung computes; every skipped
        // rung is audited (stderr + ledger), never silent.
        let mut skipped: Vec<String> = Vec::new();
        let mut chosen: Option<&str> = None;
        for rung in TIMESERIES_LADDER.iter() {
            match rung_unavailable_reason(rung) {
                Some(reason) => {
                    eprintln!("[FORECAST_NEXT] rung '{}' unavailable ({})", rung, reason);
                    skipped.push((*rung).to_string());
                }
                None => {
                    chosen = Some(rung);
                    break;
                }
            }
        }
        let rung = chosen.ok_or_else(|| {
            format!(
                "{}: the timeseries ladder is exhausted — no rung is available ({} skipped, audited)",
                fn_name,
                skipped.len()
            )
        })?;
        // The compute (in this build: seasonal_naive — the deterministic
        // built-in rung; the feature-gated timesfm rung would replace
        // this leg when compiled and weight-verified).
        let (points, quantiles, note) =
            seasonal_naive_forecast(&series.values, horizon, &series.frequency);
        let window_hash = crate::ledger::sha256_hex(window_canonical(&series.values).as_bytes());
        let entry = crate::backends::find_by_name(rung);
        let pin = match entry.map(|e| &e.pin) {
            Some(crate::backends::ShaPin::Pinned(_)) => "pinned".to_string(),
            Some(crate::backends::ShaPin::PendingNo334) => "pending-no334".to_string(),
            None => "unregistered".to_string(),
        };
        let degraded = !skipped.is_empty();
        let id = fresh_id("forecast", &format!("{}|{}", series_id, horizon));
        let rec = ForecastRecord {
            id: id.clone(),
            series_id: series_id.clone(),
            horizon,
            p10: points.iter().map(|p| p + quantiles[0]).collect(),
            p50: points.iter().map(|p| p + quantiles[1]).collect(),
            p90: points.iter().map(|p| p + quantiles[2]).collect(),
            points,
            rung: (*rung).to_string(),
            pin,
            rung_note: note,
            degraded,
            window_hash: window_hash.clone(),
            label: label.clone(),
            skipped: skipped.clone(),
        };
        let map = forecast_map(&rec);
        // The forecast.run contract (№393/ADR-0167): {window hash, rung/
        // pin, quantiles-present, degraded} + the audited skipped rungs —
        // metadata and HASHES only, never series/forecast values.
        let detail = format!(
            "window_hash={}|rung={}|pin={}|horizon={}|degraded={}|quantiles=p10,p50,p90|skipped={}|note={}|series={}|frequency={}|label={}",
            &rec.window_hash[..12],
            rec.rung,
            rec.pin,
            rec.horizon,
            rec.degraded,
            if rec.skipped.is_empty() {
                "none".to_string()
            } else {
                rec.skipped.join(",")
            },
            if rec.rung_note.is_empty() {
                "none".to_string()
            } else {
                rec.rung_note.clone()
            },
            series.id,
            series.frequency,
            series.label
        );
        ledger_forecast_event("run", &id, &detail);
        reg.forecasts.insert(id.clone(), rec);
        map
    };
    Ok(Value::ForecastHandle(forecast))
}

// ── forecast_state (the prov-block projection — metadata only) ─────────

/// `forecast_state(handle) -> Struct {id, series_id, horizon, rung,
/// pin, degraded, window_hash, label, rung_note, skipped}` — the
/// prov block. Metadata is NOT taint-bearing payload (the
/// device_state precedent: digests and words, no data).
pub fn forecast_state(fn_name: &str, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (handle), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = match &args[0] {
        Value::ForecastHandle(m) => m
            .get("id")
            .cloned()
            .ok_or_else(|| format!("{}: ForecastHandle has no id field", fn_name))?,
        other => {
            return Err(format!(
                "{}: argument 1 must be a ForecastHandle, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let reg = state().lock().map_err(|_| "forecast registry poisoned")?;
    let rec = reg
        .forecasts
        .get(&id)
        .ok_or_else(|| err_handle_unknown(fn_name, "ForecastHandle", &id))?;
    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("id".to_string(), Value::String(rec.id.clone()));
    fields.insert(
        "series_id".to_string(),
        Value::String(rec.series_id.clone()),
    );
    fields.insert("horizon".to_string(), Value::Float(rec.horizon as f64));
    fields.insert("rung".to_string(), Value::String(rec.rung.clone()));
    fields.insert("pin".to_string(), Value::String(rec.pin.clone()));
    fields.insert("degraded".to_string(), Value::Bool(rec.degraded));
    fields.insert(
        "window_hash".to_string(),
        Value::String(rec.window_hash.clone()),
    );
    fields.insert("label".to_string(), Value::String(rec.label.to_string()));
    fields.insert(
        "rung_note".to_string(),
        Value::String(rec.rung_note.clone()),
    );
    fields.insert(
        "skipped".to_string(),
        Value::List(
            rec.skipped
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect(),
        ),
    );
    Ok(Value::Struct {
        type_name: "ForecastState".to_string(),
        fields,
    })
}

// ── The gated export surfaces ───────────────────────────────────────────

/// The gate EVERY forecast-data export passes first (print /
/// to_string / json_encode / forecast_points). A tainted forecast
/// refuses fail-closed with the typed FORECAST_TAINTED stamp and the
/// refusal itself is a ledger record (forecast.denied).
pub fn check_export_allowed(surface: &str, map: &HashMap<String, String>) -> Result<(), String> {
    let id = map
        .get("id")
        .cloned()
        .unwrap_or_else(|| "<unbound>".to_string());
    let reg = state().lock().map_err(|_| "forecast registry poisoned")?;
    let rec = match reg.forecasts.get(&id) {
        Some(r) => r,
        None => return Err(err_handle_unknown("forecast export", "ForecastHandle", &id)),
    };
    if is_tainted(&rec.label) {
        return Err(deny_tainted_export(surface, &id, &rec.label.to_string()));
    }
    Ok(())
}

/// `forecast_points(handle) -> Struct {points, p10, p50, p90}` — the
/// sanctioned data projection (gated like every export surface).
pub fn forecast_points(fn_name: &str, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (handle), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = match &args[0] {
        Value::ForecastHandle(m) => m
            .get("id")
            .cloned()
            .ok_or_else(|| format!("{}: ForecastHandle has no id field", fn_name))?,
        other => {
            return Err(format!(
                "{}: argument 1 must be a ForecastHandle, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let reg = state().lock().map_err(|_| "forecast registry poisoned")?;
    let rec = match reg.forecasts.get(&id) {
        Some(r) => r,
        None => return Err(err_handle_unknown(fn_name, "ForecastHandle", &id)),
    };
    if is_tainted(&rec.label) {
        return Err(deny_tainted_export(
            "forecast_points",
            &id,
            &rec.label.to_string(),
        ));
    }
    let list = |xs: &[f64]| Value::List(xs.iter().map(|v| Value::Float(*v)).collect());
    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("points".to_string(), list(&rec.points));
    fields.insert("p10".to_string(), list(&rec.p10));
    fields.insert("p50".to_string(), list(&rec.p50));
    fields.insert("p90".to_string(), list(&rec.p90));
    Ok(Value::Struct {
        type_name: "ForecastPoints".to_string(),
        fields,
    })
}

/// The Display engine for `Value::ForecastHandle` — taint-aware by
/// construction: a CLEAN forecast renders its content (the read-only
/// materialization), a TAINTED one renders the opaque marker (Display
/// cannot error — fail-closed means no content, and the loud typed
/// refusal lives on the gated surfaces). An unknown/dropped id also
/// renders the marker (never a panic from Display).
pub fn forecast_display(map: &HashMap<String, String>) -> String {
    let id = match map.get("id") {
        Some(id) => id.clone(),
        None => return "[Forecast]".to_string(),
    };
    let reg = match state().lock() {
        Ok(r) => r,
        Err(_) => return "[Forecast]".to_string(),
    };
    match reg.forecasts.get(&id) {
        Some(rec) if !is_tainted(&rec.label) => {
            let fmt_list = |xs: &[f64]| {
                let parts: Vec<String> = xs.iter().map(|v| format!("{:?}", v)).collect();
                format!("[{}]", parts.join(", "))
            };
            format!(
                "Forecast {{ rung: {:?}, horizon: {}, points: {}, p10: {}, p50: {}, p90: {}, degraded: {} }}",
                rec.rung,
                rec.horizon,
                fmt_list(&rec.points),
                fmt_list(&rec.p10),
                fmt_list(&rec.p50),
                fmt_list(&rec.p90),
                rec.degraded
            )
        }
        _ => "[Forecast]".to_string(),
    }
}

// ── Registry introspection (tests / docs gates) ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_period_words_are_canonical() {
        assert_eq!(season_period("hourly"), 24);
        assert_eq!(season_period("daily"), 7);
        assert_eq!(season_period("monthly"), 12);
        assert_eq!(season_period("quarterly"), 4);
        assert_eq!(season_period("unknown-word"), 1);
        assert_eq!(season_period(""), 1);
    }

    #[test]
    fn nearest_rank_is_deterministic() {
        let mut xs = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(nearest_rank(&xs, 10.0), 1.0);
        assert_eq!(nearest_rank(&xs, 50.0), 3.0);
        assert_eq!(nearest_rank(&xs, 90.0), 5.0);
        assert_eq!(nearest_rank(&[], 50.0), 0.0);
    }

    #[test]
    fn constant_series_forecasts_constantly() {
        let (points, q, note) = seasonal_naive_forecast(&[2.0, 2.0, 2.0, 2.0], 3, "daily");
        assert_eq!(points, vec![2.0, 2.0, 2.0]);
        assert_eq!(q, [0.0, 0.0, 0.0]);
        assert!(note.contains("two daily-seasons"));
    }

    #[test]
    fn taint_predicate_follows_the_lattice() {
        assert!(!is_tainted(&Label::bottom()));
        let mut private = Label::bottom();
        private.conf = Conf::Private;
        assert!(is_tainted(&private));
        let mut untrusted = Label::bottom();
        untrusted.integrity = Integrity::Untrusted;
        assert!(is_tainted(&untrusted));
    }
}
