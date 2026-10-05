// ── Наряд №595 (Волна 30, Камертон Н1-05/Н1-07): UTC calendar arithmetic ──
//
// `now_unix() -> Float`           — the epoch-seconds read (the UTC-facing
//                                   twin of now(); Unix timestamps carry no
//                                   timezone by nature).
// `date_parse_iso(s) -> Float`    — the ISO-8601/RFC 3339 datetime string →
//                                   the Unix UTC timestamp (Float seconds,
//                                   microsecond precision). The missing
//                                   half of the date surface: the registry
//                                   had no ISO parse at all.
// `date_diff_days(a, b) -> Float` — the SIGNED fractional day difference
//                                   (a − b)/86400 — the response-age and
//                                   synodic-month arithmetic the consumer
//                                   needs; days_between (pre-existing)
//                                   stays absolute and untouched.
// `date_format_iso(t) -> String`  — the Unix UTC timestamp → the canonical
//                                   RFC 3339 UTC form "YYYY-MM-DDTHH:MM:SSZ"
//                                   (whole seconds — the sub-second part is
//                                   truncated at DISPLAY, never in the
//                                   arithmetic).
//
// THE UTC RULE (the naryad's core row): every computation here is UTC —
// no Local anywhere; a timezone is applied only at DISPLAY and as an
// explicit parameter (the offset IN the parsed string is honored and
// converted to UTC; a naive datetime — no offset — is interpreted as UTC,
// documented, never the machine's local zone). The pre-existing legacy
// getters (format_date/date_parts/weekday_name) keep their v0.8 LOCAL
// contract — touching them is a separate repair naryad (the wave's
// «не делать» rule: the contour does not expand beyond the listed
// surface); the honest note lives in the module report.
//
// Loud failure (the naryad's row 3): an invalid string is a LOUD typed
// refusal stamped [DATE_INVALID] (№385/ADR-0169 — whitelisted in
// ORIGIN_STAMPED_CODES, so `try{}` classifies it to the typed code on
// BOTH backends) — never a zero date, never a silent fallback. Accepted
// grammar (documented): RFC 3339 (T or space separator, optional
// fractional seconds, Z or ±HH:MM offset), plus the date-only form
// "YYYY-MM-DD" (midnight UTC).
//
// Classification (№316): date_parse_iso/date_diff_days/date_format_iso —
// the provably-pure `time` category default (closed-form calendar
// arithmetic over in-program values; parsing already-present bytes is
// Pure). now_unix reads the WALL CLOCK — the №316 Source semantics — so
// it carries an EXPLICIT override (Source/Internal/Pure with rationale),
// NOT the silent default.

use crate::interpreter::Value;

use super::core::expect_float_arg;
use crate::interpreter::values::{coded_error, CODE_DATE_INVALID};

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// `now_unix()` — the current Unix timestamp as Float (seconds since the
/// epoch; UTC by definition — a Unix timestamp has no timezone). The
/// UTC-facing twin of `now()` (the same read, the honest name for the
/// consumer's UTC-first arithmetic).
pub(crate) fn builtin_now_unix(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    Ok(Value::Float(now))
}

/// Parse one ISO datetime string into a UTC timestamp (microsecond
/// precision). The attempts are ordered: the full RFC 3339 form (offset
/// honored and converted to UTC) first; then the naive datetime with T or
/// space separator (interpreted as UTC — the documented UTC-first rule);
/// then the date-only form (midnight UTC).
fn parse_iso_utc(s: &str) -> Option<i64> {
    let micros = if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        dt.with_timezone(&Utc).timestamp_micros()
    } else if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        naive.and_utc().timestamp_micros()
    } else if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        naive.and_utc().timestamp_micros()
    } else if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        date.and_hms_opt(0, 0, 0)?.and_utc().timestamp_micros()
    } else {
        return None;
    };
    Some(micros)
}

/// `date_parse_iso(s)` — parse an ISO-8601/RFC 3339 datetime string and
/// return the Unix UTC timestamp as Float seconds (microsecond precision).
/// Accepted: the full RFC 3339 form (Z or ±HH:MM — the offset is honored
/// and converted to UTC), the naive datetime with a T or space separator
/// (interpreted as UTC — never the machine's local zone), and the
/// date-only form (midnight UTC). An invalid string is a LOUD typed
/// refusal stamped [DATE_INVALID] — never a zero date. Pure function
/// (parsing already-present bytes, №316).
pub(crate) fn builtin_date_parse_iso(args: &[Value]) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(coded_error(
                CODE_DATE_INVALID,
                format!(
                    "date_parse_iso: argument must be a String, got {}",
                    other.type_name()
                ),
            ));
        }
        None => {
            return Err(coded_error(
                CODE_DATE_INVALID,
                "date_parse_iso: expected 1 argument, got 0",
            ));
        }
    };
    let micros = parse_iso_utc(&s).ok_or_else(|| {
        coded_error(
            CODE_DATE_INVALID,
            format!("date_parse_iso: not a valid ISO-8601/RFC 3339 datetime: {s:?}"),
        )
    })?;
    Ok(Value::Float(micros as f64 / 1_000_000.0))
}

/// `date_diff_days(a, b)` — the SIGNED fractional difference of two Unix
/// timestamps in days: (a − b)/86400 (positive when a is later). No
/// rounding — the consumer (the response-age decay, the synodic-month
/// arithmetic) owns the rounding policy. Non-finite inputs are a loud
/// [DATE_INVALID] refusal (NaN would poison the arithmetic silently).
/// Pure function (№316).
pub(crate) fn builtin_date_diff_days(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("date_diff_days", args, 0)?;
    let b = expect_float_arg("date_diff_days", args, 1)?;
    if !a.is_finite() || !b.is_finite() {
        return Err(coded_error(
            CODE_DATE_INVALID,
            format!("date_diff_days: inputs must be finite, got {a}, {b}"),
        ));
    }
    Ok(Value::Float((a - b) / 86_400.0))
}

/// `date_format_iso(t)` — format a Unix UTC timestamp as the canonical RFC
/// 3339 UTC string "YYYY-MM-DDTHH:MM:SSZ". The display is UTC (the Z
/// form); the sub-second part of the input is truncated at DISPLAY only —
/// the arithmetic surfaces (date_diff_days, the raw timestamp) keep full
/// precision. An out-of-range or non-finite timestamp is a loud
/// [DATE_INVALID] refusal. Pure function (№316).
pub(crate) fn builtin_date_format_iso(args: &[Value]) -> Result<Value, String> {
    let t = expect_float_arg("date_format_iso", args, 0)?;
    if !t.is_finite() {
        return Err(coded_error(
            CODE_DATE_INVALID,
            format!("date_format_iso: timestamp must be finite, got {t}"),
        ));
    }
    // floor: the whole second that CONTAINS t (truncation toward zero
    // would misalign negative timestamps by one second).
    let secs = t.floor() as i64;
    let dt = Utc.timestamp_opt(secs, 0).single().ok_or_else(|| {
        coded_error(
            CODE_DATE_INVALID,
            format!("date_format_iso: timestamp {t} is out of the supported range"),
        )
    })?;
    Ok(Value::String(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
}

#[cfg(test)]
mod calendar_utc_tests {
    use super::*;

    fn parse(s: &str) -> Result<Value, String> {
        builtin_date_parse_iso(&[Value::String(s.to_string())])
    }

    fn get_float(v: &Value) -> f64 {
        match v {
            Value::Float(f) => *f,
            other => panic!("expected Float, got {:?}", other.type_name()),
        }
    }

    /// The control-date golden (12 dates computed by an INDEPENDENT source
    /// — the Python datetime/calendar stack, authoring time; the same pins
    /// run through the language on both backends in
    /// tests/naryad_595_calendar_utc.rs): epoch, leap Feb 29, year end,
    /// midnight crossing, offsets, fractional seconds, date-only, pre-2000,
    /// the 2000 century-leap window, the space separator.
    #[test]
    fn n595_twelve_control_dates_match_the_independent_source() {
        const GOLDEN: [(&str, f64); 12] = [
            ("1970-01-01T00:00:00Z", 0.0),
            ("2024-02-29T12:00:00Z", 1709208000.0),
            ("2025-12-31T23:59:59Z", 1767225599.0),
            ("2026-01-01T00:00:00Z", 1767225600.0),
            ("2026-10-05T12:34:56Z", 1791203696.0),
            ("2026-10-05T12:34:56+03:00", 1791192896.0),
            ("2026-10-05T12:34:56-05:30", 1791223496.0),
            ("2026-10-05T12:34:56.123Z", 1791203696.123),
            ("2026-10-05", 1791158400.0),
            ("1999-12-31T23:59:59Z", 946684799.0),
            ("2000-02-28T06:30:00Z", 951719400.0),
            ("2026-10-05 12:34:56", 1791203696.0),
        ];
        for (input, want) in GOLDEN {
            let got = get_float(&parse(input).unwrap_or_else(|e| panic!("{input}: {e}")));
            let rel = ((got - want) / want)
                .abs()
                .max(if want == 0.0 { got.abs() } else { 0.0 });
            assert!(rel <= 1e-9, "{input}: got {got}, want {want}");
        }
    }

    /// The loud refusal row: invalid strings are typed [DATE_INVALID],
    /// never zero dates — month 13, day 30 of a 28-day February, garbage,
    /// an empty string, a bare year, a wrong-typed argument.
    #[test]
    fn n595_invalid_inputs_are_loud_typed_refusals() {
        for bad in [
            "2026-13-01T00:00:00Z",
            "2026-02-30T00:00:00Z",
            "garbage",
            "",
            "2026",
            "not a date at all",
        ] {
            let err = parse(bad).expect_err(bad);
            assert!(
                err.starts_with("[DATE_INVALID] "),
                "{bad:?}: the refusal must carry the stamp, got: {err}"
            );
        }
        let wrong_type = builtin_date_parse_iso(&[Value::Float(1.0)]).unwrap_err();
        assert!(
            wrong_type.starts_with("[DATE_INVALID] "),
            "got: {wrong_type}"
        );
    }

    /// date_diff_days: signed and fractional — (a − b)/86400 exactly; a
    /// NaN input refuses loudly instead of poisoning.
    #[test]
    fn n595_diff_days_signed_fractional_and_finite_guarded() {
        let a = get_float(&parse("2026-10-05T12:34:56Z").unwrap());
        let b = get_float(&parse("2026-10-05T00:00:00Z").unwrap());
        let d = get_float(&builtin_date_diff_days(&[Value::Float(a), Value::Float(b)]).unwrap());
        assert!((d - (a - b) / 86_400.0).abs() < 1e-12);
        // 12:34:56 − 00:00:00 = 45296 s = 45296/86400 days
        assert!((d - 45296.0 / 86_400.0).abs() < 1e-9, "d = {}", d);
        // signed: reversed arguments flip the sign
        let d_rev =
            get_float(&builtin_date_diff_days(&[Value::Float(b), Value::Float(a)]).unwrap());
        assert!((d + d_rev).abs() < 1e-12);
        // NaN refuses loudly
        let nan_err =
            builtin_date_diff_days(&[Value::Float(f64::NAN), Value::Float(0.0)]).unwrap_err();
        assert!(nan_err.starts_with("[DATE_INVALID] "), "got: {nan_err}");
    }

    /// date_format_iso: the canonical UTC Z-form; the sub-second part
    /// truncates at DISPLAY only; non-finite and out-of-range refuse
    /// loudly; negative timestamps floor correctly.
    #[test]
    fn n595_format_iso_utc_form_and_guards() {
        let t = get_float(&parse("2026-10-05T12:34:56Z").unwrap());
        match builtin_date_format_iso(&[Value::Float(t)]).unwrap() {
            Value::String(s) => assert_eq!(s, "2026-10-05T12:34:56Z"),
            other => panic!("expected String, got {:?}", other.type_name()),
        }
        // the fractional input displays whole seconds (truncated at display)
        let tf = get_float(&parse("2026-10-05T12:34:56.999Z").unwrap());
        match builtin_date_format_iso(&[Value::Float(tf)]).unwrap() {
            Value::String(s) => assert_eq!(s, "2026-10-05T12:34:56Z"),
            other => panic!("expected String, got {:?}", other.type_name()),
        }
        // the offset-aware parse and the UTC display agree (12:34:56+03:00 = 09:34:56Z)
        let t_off = get_float(&parse("2026-10-05T12:34:56+03:00").unwrap());
        match builtin_date_format_iso(&[Value::Float(t_off)]).unwrap() {
            Value::String(s) => assert_eq!(s, "2026-10-05T09:34:56Z"),
            other => panic!("expected String, got {:?}", other.type_name()),
        }
        // non-finite refuses
        let nan_err = builtin_date_format_iso(&[Value::Float(f64::NAN)]).unwrap_err();
        assert!(nan_err.starts_with("[DATE_INVALID] "), "got: {nan_err}");
    }

    /// The round-trip contract: format(parse(x)) == x for every canonical
    /// UTC input (the parse/format pair closes on the Z-form).
    #[test]
    fn n595_round_trip_format_parse_closes() {
        for canonical in [
            "1970-01-01T00:00:00Z",
            "2024-02-29T12:00:00Z",
            "2025-12-31T23:59:59Z",
            "2026-01-01T00:00:00Z",
            "2026-10-05T12:34:56Z",
            "1999-12-31T23:59:59Z",
        ] {
            let t = get_float(&parse(canonical).unwrap());
            match builtin_date_format_iso(&[Value::Float(t)]).unwrap() {
                Value::String(s) => assert_eq!(s, canonical, "round trip must close"),
                other => panic!("expected String, got {:?}", other.type_name()),
            }
        }
    }
}
