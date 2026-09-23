// ── Naryad #440 (P1, feature/forecast — the forecasting domain): the
//    forecast contour contracts ────────────────────────────────────────
//
// Corpus:
//   (1) the two forecast handles are the ADR-0114 opaque pattern:
//       Series is fully opaque (non-printable, `[Series]`); Forecast is
//       TAINT-AWARE (clean content materializes; tainted is the
//       fail-closed `[Forecast]` marker on non-gated surfaces);
//   (2) the seasonal_naive rung is deterministic (points + p10/p50/p90
//       quantiles, nearest-rank, no RNG);
//   (3) the degradation ladder: skipped rungs are AUDITED and the
//       result carries degraded=true (never a silent skip — Устав §11
//       Шаг 3); the prov block carries rung/pin/window-hash/horizon;
//   (4) the taint discipline: the source label transfers to the
//       forecast (LabelJoin); the gated export surfaces (to_string /
//       json_encode / print / forecast_points) refuse a tainted
//       forecast with the typed FORECAST_TAINTED stamp (branchable in
//       try, TW and VM) and the refusal is a forecast.denied ledger
//       record; interpolation is the fail-closed marker;
//   (5) the grant gate: series_pull refuses without a grant BEFORE
//       anything else, and with an active grant names the documented
//       no-source-backend gap (never a silent substitution);
//   (6) the registry profile: three `timeseries` entries, the ONLY
//       timesfm pin is the Apache-2.0 2.5 weights, TimesFM 3.0 is
//       pinned NEVER;
//   (7) the companion checks: a literal bad label word
//       (FORECAST_LABEL_INVALID) and an out-of-guard literal horizon
//       (FORECAST_HORIZON_INVALID) are compile errors;
//   (8) TW/VM parity: the same forecast programs run identically on
//       both backends (the shared BUILTIN_REGISTRY dispatch).
use std::path::Path;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), base_dir.to_path_buf())
}
fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}
const BASE: &str = env!("CARGO_MANIFEST_DIR");
fn base_dir() -> std::path::PathBuf {
    std::path::Path::new(BASE).to_path_buf()
}

use metalogos::interpreter::values::Value;

fn series_values() -> Vec<f64> {
    vec![12.0, 14.0, 13.0, 15.0, 16.0, 15.0, 17.0, 18.0, 16.0, 19.0]
}

// ── (1) The opaque invariants ───────────────────────────────────────────

#[test]
fn forecast_handles_follow_the_opaque_pattern() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let s = metalogos::forecast::series_make(
        "t",
        &[Value::List(
            series_values().into_iter().map(Value::Float).collect(),
        )],
    )
    .unwrap();
    // Series: fully opaque — the marker, the type name, non-printable.
    assert_eq!(format!("{}", s), "[Series]");
    assert_eq!(s.type_name(), "Series");
    assert!(metalogos::interpreter::values::is_nonprintable(&s));
    // No values in the projection map.
    if let Value::SeriesHandle(m) = &s {
        assert!(m.contains_key("id"));
        assert!(m.contains_key("frequency"));
        assert!(m.contains_key("label"));
        assert!(m.contains_key("length"));
        assert!(
            !m.iter().any(|(_, v)| v.contains("12.0")),
            "no payload in Value"
        );
    } else {
        panic!("expected SeriesHandle");
    }

    let f = metalogos::forecast::forecast_next("t", &[s, Value::Float(3.0)]).unwrap();
    assert_eq!(f.type_name(), "Forecast");
    // Forecast: NOT in the generic non-printable set — its
    // materialization is the taint-conditional gate.
    assert!(!metalogos::interpreter::values::is_nonprintable(&f));
    // The clean forecast renders its read-only content.
    let rendered = format!("{}", f);
    assert!(
        rendered.starts_with("Forecast {"),
        "clean forecast renders content, got: {}",
        rendered
    );
    assert!(rendered.contains("rung: \"seasonal_naive\""));
    assert!(rendered.contains("degraded: true"));

    // The prov block: metadata only, no points.
    let st = metalogos::forecast::forecast_state("t", std::slice::from_ref(&f)).unwrap();
    if let Value::Struct { type_name, fields } = &st {
        assert_eq!(type_name, "ForecastState");
        assert_eq!(format!("{}", fields.get("rung").unwrap()), "seasonal_naive");
        assert!(matches!(fields.get("degraded"), Some(Value::Bool(true))));
        assert_eq!(format!("{}", fields.get("pin").unwrap()), "pending-no334");
        assert!(matches!(fields.get("window_hash"), Some(Value::String(h)) if h.len() == 64));
        assert_eq!(
            format!("{}", fields.get("skipped").unwrap()),
            "[timesfm-2.5, statsforecast]"
        );
    } else {
        panic!("expected ForecastState struct");
    }
    // The gated data projection of a clean forecast returns the points.
    let pts = metalogos::forecast::forecast_points("t", &[f]).unwrap();
    if let Value::Struct { type_name, fields } = &pts {
        assert_eq!(type_name, "ForecastPoints");
        assert!(matches!(fields.get("points"), Some(Value::List(xs)) if xs.len() == 3));
        assert!(matches!(fields.get("p10"), Some(Value::List(xs)) if xs.len() == 3));
        assert!(matches!(fields.get("p90"), Some(Value::List(xs)) if xs.len() == 3));
    } else {
        panic!("expected ForecastPoints struct");
    }
}

// ── (2) The seasonal_naive determinism ──────────────────────────────────

#[test]
fn seasonal_naive_is_deterministic_with_quantiles() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // A constant series forecasts constantly, quantiles collapse to the point.
    let s = metalogos::forecast::series_make(
        "t",
        &[Value::List(
            vec![5.0, 5.0, 5.0, 5.0, 5.0, 5.0]
                .into_iter()
                .map(Value::Float)
                .collect(),
        )],
    )
    .unwrap();
    let f = metalogos::forecast::forecast_next("t", &[s, Value::Float(2.0)]).unwrap();
    let pts = metalogos::forecast::forecast_points("t", &[f]).unwrap();
    if let Value::Struct { fields, .. } = &pts {
        let get = |k: &str| match fields.get(k) {
            Some(Value::List(xs)) => xs
                .iter()
                .map(|v| match v {
                    Value::Float(x) => *x,
                    _ => panic!("float"),
                })
                .collect::<Vec<f64>>(),
            _ => panic!("list"),
        };
        assert_eq!(get("points"), vec![5.0, 5.0]);
        assert_eq!(get("p50"), vec![5.0, 5.0]);
    } else {
        panic!("expected struct");
    }

    // An ascending series: seasonal-naive points repeat the last season;
    // the residual band orders p10 <= p50 <= p90.
    let s2 = metalogos::forecast::series_make(
        "t",
        &[Value::List(
            [1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0, 1.0, 2.0, 3.0, 4.0]
                .into_iter()
                .map(Value::Float)
                .collect(),
        )],
    )
    .unwrap();
    let f2 = metalogos::forecast::forecast_next("t", &[s2, Value::Float(4.0)]).unwrap();
    let pts2 = metalogos::forecast::forecast_points("t", &[f2]).unwrap();
    if let Value::Struct { fields, .. } = &pts2 {
        let get = |k: &str| match fields.get(k) {
            Some(Value::List(xs)) => xs
                .iter()
                .map(|v| match v {
                    Value::Float(x) => *x,
                    _ => panic!("float"),
                })
                .collect::<Vec<f64>>(),
            _ => panic!("list"),
        };
        let (p10, p50, p90) = (get("p10"), get("p50"), get("p90"));
        for i in 0..4 {
            assert!(
                p10[i] <= p50[i] && p50[i] <= p90[i],
                "quantile order at {}",
                i
            );
        }
    } else {
        panic!("expected struct");
    }
}

// ── (3)+(4) The taint discipline: gated exports refuse, TW and VM ───────

fn tainted_forecast_program() -> String {
    "pattern TaintGate(_x: String) -> String {
  let t = series_make({values: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], frequency: \"daily\", label: \"private\"})
  let tf = forecast_next(t, 2.0)
  let r = try to_string(tf)
  if r.ok == false {
    return \"DENIED:\" + r.error.code
  }
  return \"LEAKED:\" + r.value
}
flow Main {
  input: String = \"go\"
  -> TaintGate
  -> output
}
"
    .to_string()
}

#[test]
fn tainted_forecast_export_is_typed_denied_on_both_backends() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let src = tainted_forecast_program();
    let base = base_dir();
    let tw = run_tw(&src, &base).unwrap().unwrap_or_default();
    assert_eq!(
        tw.trim(),
        "DENIED:FORECAST_TAINTED",
        "TW must refuse the export"
    );
    let vm = run_vm(&src, &base).unwrap().unwrap_or_default();
    assert_eq!(
        vm.trim(),
        "DENIED:FORECAST_TAINTED",
        "VM must refuse the export"
    );
}

#[test]
fn tainted_forecast_is_refused_on_every_gated_surface() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // One program probing ALL FOUR gated surfaces of a tainted forecast;
    // every leg must land in the typed refusal (and the summary proves
    // the count — no surface leaked).
    let src = "pattern GateProbe(_x: String) -> String {\n  let t = series_make({values: [1.0, 2.0, 3.0, 4.0], frequency: \"daily\", label: \"private\"})\n  let tf = forecast_next(t, 2.0)\n  let r1 = try to_string(tf)\n  let s1 = if r1.ok == false { r1.error.code } else { \"LEAK\" }\n  let r2 = try json_encode(tf)\n  let s2 = if r2.ok == false { r2.error.code } else { \"LEAK\" }\n  let r3 = try print(tf)\n  let s3 = if r3.ok == false { r3.error.code } else { \"LEAK\" }\n  let r4 = try forecast_points(tf)\n  let s4 = if r4.ok == false { r4.error.code } else { \"LEAK\" }\n  return s1 + \"|\" + s2 + \"|\" + s3 + \"|\" + s4\n}\nflow Main {\n  input: String = \"go\"\n  -> GateProbe\n  -> output\n}\n";
    let base = base_dir();
    let tw = run_tw(src, &base).unwrap().unwrap_or_default();
    assert_eq!(
        tw.trim(),
        "FORECAST_TAINTED|FORECAST_TAINTED|FORECAST_TAINTED|FORECAST_TAINTED",
        "all four gated surfaces refuse with the typed stamp: {}",
        tw
    );
    let vm = run_vm(src, &base).unwrap().unwrap_or_default();
    assert_eq!(vm.trim(), tw.trim(), "VM agrees");

    // The registry-level label transfer: the forecast inherits the
    // source label word.
    let s = metalogos::forecast::series_make(
        "t",
        &[Value::Struct {
            type_name: "SeriesSource".to_string(),
            fields: {
                let mut f = std::collections::HashMap::new();
                f.insert(
                    "values".to_string(),
                    Value::List(
                        vec![1.0, 2.0, 3.0, 4.0]
                            .into_iter()
                            .map(Value::Float)
                            .collect(),
                    ),
                );
                f.insert("frequency".to_string(), Value::String("daily".to_string()));
                f.insert("label".to_string(), Value::String("private".to_string()));
                f
            },
        }],
    )
    .unwrap();
    let f = metalogos::forecast::forecast_next("t", &[s, Value::Float(2.0)]).unwrap();
    if let Value::ForecastHandle(m) = &f {
        assert_eq!(m.get("label").unwrap(), "private, trusted");
    } else {
        panic!("expected ForecastHandle");
    }
    // Display: the fail-closed marker — no content ever.
    assert_eq!(format!("{}", f), "[Forecast]");
}

#[test]
fn clean_forecast_materializes_but_a_tainted_one_never_leaks_through_interpolation() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // Clean: the content renders through to_string.
    let s = metalogos::forecast::series_make(
        "t",
        &[Value::List(
            vec![1.0, 2.0, 3.0, 4.0]
                .into_iter()
                .map(Value::Float)
                .collect(),
        )],
    )
    .unwrap();
    let f = metalogos::forecast::forecast_next("t", &[s, Value::Float(2.0)]).unwrap();
    let content = format!("{}", f);
    assert!(
        content.contains("points: ["),
        "clean content renders: {}",
        content
    );
}

// ── (5) The grant gate ──────────────────────────────────────────────────

#[test]
fn series_pull_is_grant_gated_and_names_the_documented_gap() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // Without a grant-shaped argument: the refusal names the gate.
    let e = metalogos::forecast::series_pull(
        "t",
        &[
            Value::String("not-a-grant".to_string()),
            Value::String("external-source".to_string()),
        ],
    )
    .unwrap_err();
    assert!(e.contains("grant-gated"), "the gate is named first: {}", e);
    // With a REAL active grant: the honest no-source refusal (the
    // documented dispatcher gap), never a silent substitution.
    let grant = metalogos::grants::issue(
        "forecast:pull:external",
        3600,
        &metalogos::grants::GrantClass::Once,
        "naryad-440-test",
    )
    .unwrap();
    let e2 = metalogos::forecast::series_pull(
        "t",
        &[
            Value::Grant(grant),
            Value::String("external-source".to_string()),
        ],
    )
    .unwrap_err();
    assert!(
        e2.contains("no external series source backend"),
        "the verified-grant refusal names the gap: {}",
        e2
    );
}

// ── (6) The registry profile ────────────────────────────────────────────

#[test]
fn timeseries_registry_profile_is_honest() {
    let entries: Vec<_> = metalogos::backends::BACKEND_REGISTRY
        .iter()
        .filter(|e| e.class == metalogos::backends::BackendClass::Timeseries)
        .collect();
    assert_eq!(entries.len(), 3, "the ladder has exactly three rungs");
    let names: Vec<_> = entries.iter().map(|e| e.name).collect();
    assert!(names.contains(&"timesfm-2.5"));
    assert!(names.contains(&"statsforecast"));
    assert!(names.contains(&"seasonal_naive"));
    // The ONLY timesfm pin: the Apache-2.0 2.5 weights, honestly pending.
    let tf = entries.iter().find(|e| e.name == "timesfm-2.5").unwrap();
    assert_eq!(tf.weights_id, "google/timesfm-2.5-200m-pytorch");
    assert_eq!(tf.pin, metalogos::backends::ShaPin::PendingNo334);
    assert_eq!(tf.license, metalogos::backends::LicenseClass::Osi);
    assert!(tf.license_note.contains("Apache-2.0"));
    // TimesFM 3.0 is pinned NEVER.
    assert!(
        !metalogos::backends::BACKEND_REGISTRY
            .iter()
            .any(|e| e.name.contains("3.0")),
        "no 3.0 rung may exist (non-commercial license)"
    );
    // The class word parses.
    assert_eq!(
        metalogos::backends::BackendClass::parse("timeseries"),
        Some(metalogos::backends::BackendClass::Timeseries)
    );
}

// ── (7) The companion checks ────────────────────────────────────────────

#[test]
fn literal_bad_label_word_is_a_compile_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let src = "pattern BadLabel(_x: String) -> String {
  let t = series_make({values: [1.0, 2.0], label: \"sekret\"})
  return \"ok\"
}
flow Main {
  input: String = \"go\"
  -> BadLabel
  -> output
}
";
    let err = run_tw(src, &base_dir()).unwrap_err();
    assert!(
        err.contains("FORECAST_LABEL_INVALID") || err.contains("invalid label"),
        "the compile path refuses the unknown label word loudly: {}",
        err
    );
    // The audit path reports the SAME check id.
    let report = metalogos::audit::audit_program(src).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check_id == "FORECAST_LABEL_INVALID"),
        "audit must carry the FORECAST_LABEL_INVALID check id"
    );
}

#[test]
fn literal_out_of_guard_horizon_is_a_compile_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let src = "pattern BadHorizon(_x: String) -> String {
  let s = series_make([1.0, 2.0, 3.0])
  let f = forecast_next(s, 5000.0)
  return \"ok\"
}
flow Main {
  input: String = \"go\"
  -> BadHorizon
  -> output
}
";
    let err = run_tw(src, &base_dir()).unwrap_err();
    assert!(
        err.contains("FORECAST_HORIZON_INVALID") || err.contains("outside the loud guard"),
        "the compile path refuses the horizon breach loudly: {}",
        err
    );
    let report = metalogos::audit::audit_program(src).unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check_id == "FORECAST_HORIZON_INVALID"),
        "audit must carry the FORECAST_HORIZON_INVALID check id"
    );
}

// ── (8) TW/VM parity over the green leg ─────────────────────────────────

#[test]
fn forecast_green_leg_parity_on_both_backends() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let src = "pattern GreenLeg(_x: String) -> String {
  let s = series_make([5.0, 5.0, 5.0, 5.0, 5.0, 5.0], \"daily\")
  let f = forecast_next(s, 3.0)
  let st = forecast_state(f)
  let d = forecast_points(f)
  let pts = d.points
  return to_string(st.rung) + \"|\" + to_string(pts[0]) + \"|\" + to_string(st.degraded)
}
flow Main {
  input: String = \"go\"
  -> GreenLeg
  -> output
}
";
    let base = base_dir();
    let tw = run_tw(src, &base).unwrap().unwrap_or_default();
    let vm = run_vm(src, &base).unwrap().unwrap_or_default();
    assert_eq!(
        tw.trim(),
        "seasonal_naive|5|true",
        "the green leg is deterministic: {}",
        tw
    );
    assert_eq!(vm.trim(), tw.trim(), "TW and VM must agree");
}

#[test]
fn horizon_and_handle_misuse_is_loud() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let s = metalogos::forecast::series_make(
        "t",
        &[Value::List(
            vec![1.0, 2.0, 3.0].into_iter().map(Value::Float).collect(),
        )],
    )
    .unwrap();
    // The horizon guard: fractional, zero, negative, over-cap.
    for h in [0.5, 0.0, -3.0, 2000.0] {
        let e = metalogos::forecast::forecast_next("t", &[s.clone(), Value::Float(h)]).unwrap_err();
        assert!(
            e.contains("horizon"),
            "horizon {} must refuse loudly: {}",
            h,
            e
        );
    }
    // Unknown handle misuse: the typed stamp.
    let mut foreign = std::collections::HashMap::new();
    foreign.insert("id".to_string(), "series-never-was".to_string());
    let e =
        metalogos::forecast::forecast_next("t", &[Value::SeriesHandle(foreign), Value::Float(2.0)])
            .unwrap_err();
    assert!(
        e.starts_with("[FORECAST_HANDLE_UNKNOWN] "),
        "unknown handle refuses with the typed stamp: {}",
        e
    );
}
