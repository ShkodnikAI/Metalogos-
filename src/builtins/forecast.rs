// ── Naryad #440 (P1, feature/forecast): the forecasting-domain
//    builtins ─────────────────────────────────────────────────────────
//
// The language surface of the forecast contour (state: src/forecast.rs).
// Arity/typing contract (registry SSOT — keep in sync with spec! rows):
//
//   series_make(source, frequency?)     -> Series   the opaque series
//     handle; source = List[Float] (the literal form — public by
//     construction) or Struct {values, frequency?, label?} (the
//     explicit form; the label word parses through the №322 lattice —
//     over-tainting allowed, under-tainting impossible). An empty or
//     over-capacity series refuses loudly.
//   series_pull(grant, source)          -> Series   the GRANT-GATED
//     external pull (the №335/№390 contour): an ungrantable argument
//     refuses BEFORE anything else; with an active grant the refusal
//     is the documented no-source-backend gap (Устав §11 Шаг 3) —
//     never a silent substitution.
//   forecast_next(handle, horizon)      -> Forecast  the ladder walk
//     (timesfm-2.5 -> statsforecast -> seasonal_naive); skipped rungs
//     are audited, the result carries the prov block and the JOINED
//     source label; records forecast.run.
//   forecast_state(handle)              -> Struct   the prov-block
//     projection (metadata only — id/series/horizon/rung/pin/degraded/
//     window_hash/label/note/skipped; NO points).
//   forecast_points(handle)             -> Struct   THE gated data
//     projection {points, p10, p50, p90} — a tainted forecast refuses
//     with the typed FORECAST_TAINTED stamp + forecast.denied.

use crate::interpreter::values::Value;

/// `series_make(source, frequency?) -> Series`
pub(crate) fn builtin_series_make(args: &[Value]) -> Result<Value, String> {
    crate::forecast::series_make("series_make", args)
}

/// `series_pull(grant, source) -> Series`
pub(crate) fn builtin_series_pull(args: &[Value]) -> Result<Value, String> {
    crate::forecast::series_pull("series_pull", args)
}

/// `forecast_next(handle, horizon) -> Forecast`
pub(crate) fn builtin_forecast_next(args: &[Value]) -> Result<Value, String> {
    crate::forecast::forecast_next("forecast_next", args)
}

/// `forecast_state(handle) -> Struct`
pub(crate) fn builtin_forecast_state(args: &[Value]) -> Result<Value, String> {
    crate::forecast::forecast_state("forecast_state", args)
}

/// `forecast_points(handle) -> Struct`
pub(crate) fn builtin_forecast_points(args: &[Value]) -> Result<Value, String> {
    crate::forecast::forecast_points("forecast_points", args)
}

// ── The export gates (the guard_world_state_* template, taint-
//    conditional instead of always-on) ────────────────────────────────

/// The `to_string`/interpolation-adjacent leg: a TAINTED forecast
/// refuses (typed stamp + forecast.denied); a clean one materializes
/// its read-only content through Display. Series handles never
/// materialize (the generic opaque refusal is the floor).
pub(crate) fn guard_forecast_to_string(v: &Value) -> Result<(), String> {
    if let Value::ForecastHandle(map) = v {
        crate::forecast::check_export_allowed("to_string", map)?;
    }
    Ok(())
}

/// The `json_encode` leg — the one surface where the projection map
/// becomes content; the taint gate fires before serialization.
pub(crate) fn guard_forecast_json(v: &Value) -> Result<(), String> {
    if let Value::ForecastHandle(map) = v {
        crate::forecast::check_export_allowed("json_encode", map)?;
    }
    Ok(())
}

/// The `print` leg: `print(forecast)` on a TAINTED forecast gives the
/// typed refusal (more precise than the generic String-arity error).
pub(crate) fn check_print_forecast(v: &Value) -> Result<(), String> {
    if let Value::ForecastHandle(map) = v {
        crate::forecast::check_export_allowed("print", map)?;
    }
    Ok(())
}
