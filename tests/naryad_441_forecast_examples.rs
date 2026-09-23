// ── Naryad №441 (issue #636, Wave 12) — the forecast-domain example
//    contract ──────────────────────────────────────────────────────────
//
// The wave-12 example line convention (the №436/№437 line): a real run
// of examples/w12_forecast_ladder.mlog against the .expected golden —
// the forecast domain (№440) is LOAD-BEARING at the language surface:
//
//   RED    a series from a tainted source ("private") inherits the
//          label through forecast_next (LabelJoin), and the DATA
//          export refuses typed FORECAST_TAINTED (the №413 convention,
//          branchable in try; the refusal is a forecast.denied ledger
//          record).
//   GREEN  the clean ladder leg degrades LOUDLY: rung=seasonal_naive +
//          degraded=true (timesfm-2.5 is feature-gated, statsforecast
//          is not vendored — the skips are audited, never silent),
//          and the gated points projection materializes.
//   MARKER the non-gated interpolation surface is fail-closed: a
//          tainted forecast renders "[Forecast]", never content.
//
// The mutation harness scripts/mutation_verify_441.sh runs THIS file
// against two live mutants (M1 the taint export gate neutered, M2 the
// degraded flag falsified) and requires the matching anchor test to
// FAIL under each mutation — the tests below are that pair.

use std::fs;
use std::path::Path;
use std::sync::Mutex;

static LEDGER_LOCK: Mutex<()> = Mutex::new(());

/// Execute the wave-12 forecast example in-process (the golden.rs seam:
/// `metalogos::run_program`) and return its output.
fn run_example() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let path = Path::new(&manifest_dir).join("examples/w12_forecast_ladder.mlog");
    let source = fs::read_to_string(&path).expect("examples/w12_forecast_ladder.mlog exists");
    metalogos::run_program(&source)
        .expect("w12_forecast_ladder must execute cleanly (no uncaught refusals)")
        .unwrap_or_default()
}

fn field_of(report: &str, key: &str) -> String {
    report
        .split('|')
        .find(|f| f.starts_with(&format!("{key}:")))
        .unwrap_or_else(|| panic!("the report carries the {key} field: {report}"))
        .trim()
        .to_string()
        .split_once(':')
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

/// The .expected golden matches the real run byte-for-byte.
#[test]
fn n441_expected_golden_matches() {
    let out = run_example();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let expected = fs::read_to_string(Path::new(&manifest_dir).join(
        "examples/w12_forecast_ladder.expected",
    ))
    .expect("examples/w12_forecast_ladder.expected exists");
    assert_eq!(
        out.trim(),
        expected.trim(),
        "the real run must match the .expected golden byte-for-byte"
    );
}

/// M1 anchor: the taint export gate is load-bearing — the tainted
/// forecast refuses the to_string export with the typed
/// FORECAST_TAINTED stamp. Under mutation M1 (check_export_allowed
/// neutered to pass-through) this test MUST go red.
#[test]
fn n441_red_export_is_typed_denied() {
    let out = run_example();
    assert_eq!(
        field_of(&out, "red"),
        "FORECAST_TAINTED",
        "the tainted forecast export must refuse typed FORECAST_TAINTED; \
         got {out:?} — the taint gate is NOT load-bearing"
    );
}

/// M2 anchor: the degraded flag is honest — the ladder SKIPPED two
/// rungs (timesfm-2.5, statsforecast) and the prov block says so.
/// Under mutation M2 (degraded falsified to a constant false — the
/// ladder silently skips) this test MUST go red.
#[test]
fn n441_green_ladder_degrades_loudly() {
    let out = run_example();
    assert_eq!(
        field_of(&out, "rung"),
        "seasonal_naive",
        "the built-in rung computes in this build; got {out:?}"
    );
    assert_eq!(
        field_of(&out, "degraded"),
        "true",
        "the prov block must carry degraded=true when rungs were skipped — \
         a silently skipped rung is the exact defect the ladder contract forbids"
    );
    assert_eq!(
        field_of(&out, "p0"),
        "13",
        "the seasonal_naive projection is deterministic (the last observed week repeats)"
    );
}

/// The interpolation surface is fail-closed: the tainted forecast
/// renders the opaque marker, never content (Display cannot error —
/// the loud typed refusal lives on the gated surfaces).
#[test]
fn n441_interpolation_marker_is_fail_closed() {
    let out = run_example();
    assert_eq!(
        field_of(&out, "marker"),
        "[Forecast]",
        "a tainted forecast renders the opaque marker through format(); got {out:?}"
    );
}

/// The refusal and the run are ledger records (the №428 posture: no
/// silent egress AND no silent refusal) — the forecast.* family is
/// observable after the example run.
#[test]
fn n441_ledger_family_records_the_story() {
    let _guard = LEDGER_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let before = metalogos::ledger::all_records().map(|r| r.len()).unwrap_or(0);
    let _ = run_example();
    let records = metalogos::ledger::all_records().expect("the ledger reads back");
    let tail = &records[records.len().saturating_sub(before + 40).min(records.len())..];
    let has = |prefix: &str| {
        tail.iter().any(|r| r.action.starts_with(prefix))
    };
    assert!(has("forecast.series_make"), "series_make is an audited ledger record");
    assert!(has("forecast.run"), "forecast_next records forecast.run (rung/pin/quantiles/degraded)");
    assert!(has("forecast.denied"), "the export refusal records forecast.denied");
}
