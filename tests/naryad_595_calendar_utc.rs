//! Наряд №595 (Волна 30, Камертон Н1-05/Н1-07): the UTC calendar
//! arithmetic — `now_unix()`, `date_parse_iso(s)`, `date_diff_days(a, b)`,
//! `date_format_iso(t)`.
//!
//! Contracts under test (all pinned on BOTH backends):
//! 1. GOLDEN (the DoD row «12 контрольных дат сверены со вторым
//!    независимым источником»): the 12 control dates were computed by the
//!    Python datetime/calendar stack at authoring time (INDEPENDENT of
//!    chrono — the Rust-side implementation) — epoch, the leap Feb 29,
//!    the year end, the midnight crossing, ±offsets, fractional seconds,
//!    the date-only form, pre-2000, the 2000 century-leap window, the
//!    space separator. The values are reproduced THROUGH THE LANGUAGE on
//!    both backends (byte-identical outputs — the TW↔VM parity).
//! 2. The UTC rule: an offset-bearing string converts to UTC; a naive
//!    datetime reads as UTC (never the machine's local zone — the test
//!    would flake under any non-UTC TZ if the rule broke, which is the
//!    pin); the display form is the canonical "YYYY-MM-DDTHH:MM:SSZ".
//! 3. The loud refusal row: invalid strings are typed [DATE_INVALID] —
//!    never zero dates; `try{}` classifies to DATE_INVALID on both
//!    backends (№385/ADR-0169).
//! 4. date_diff_days: signed, fractional, exact; date_format_iso: the Z
//!    form, the sub-second truncation at DISPLAY only.
//! 5. now_unix: the read stays within the wall-clock envelope of the test
//!    process (a Rust-side before/after bracket); TW and VM reads differ
//!    by less than the harness overhead — the honest parity statement for
//!    a wall-clock read.
//!
//! The midnight-crossing, leap-year and year-end rows live in the golden
//! set AND in the src unit tests (src/builtins/calendar_utc.rs) — the
//! DoD's explicit rows.

use std::path::{Path, PathBuf};

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

fn assert_parity(name: &str, source: &str) -> String {
    let base_dir = PathBuf::from("examples");
    let tw =
        run_tw(source, &base_dir).unwrap_or_else(|e| panic!("{}: TW must succeed: {}", name, e));
    let vm =
        run_vm(source, &base_dir).unwrap_or_else(|e| panic!("{}: VM must succeed: {}", name, e));
    let tw_out = tw
        .as_deref()
        .map(str::trim_end)
        .unwrap_or_default()
        .to_string();
    let vm_out = vm
        .as_deref()
        .map(str::trim_end)
        .unwrap_or_default()
        .to_string();
    assert_eq!(tw_out, vm_out, "{}: TW and VM outputs diverge", name);
    tw_out
}

fn program(body: &str) -> String {
    format!(
        "pattern T(_input: String) -> String {{\n{}\n}}\nflow Main {{ input: String = \"s\" -> T -> output }}",
        body
    )
}

fn assert_close(got: f64, want: f64, what: &str) {
    if want == 0.0 {
        assert_eq!(got, want, "{}: got {}, want 0", what, got);
        return;
    }
    let rel = ((got - want) / want).abs();
    assert!(
        rel <= 1e-9,
        "{}: got {}, want {} (rel err {:.3e} > 1e-9)",
        what,
        got,
        want,
        rel
    );
}

/// The 12 control dates (the Python golden) — run through the language on
/// both backends; every parse must land within 1e-9 relative of the
/// independent source.
#[test]
fn n595_twelve_control_dates_pinned_on_both_backends() {
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
        let src = program(&format!(
            "  return to_string(date_parse_iso(\"{}\"))",
            input.replace('"', "\\\"")
        ));
        let out = assert_parity("n595_golden", &src);
        let got: f64 = out
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("{}: output {:?} must parse: {}", input, out, e));
        assert_close(got, want, input);
    }
}

/// The UTC display + the signed fractional difference + the midnight
/// crossing, all through one program (parity-pinned):
/// 2026-01-01T00:00:00Z minus 2025-12-31T23:59:59Z = 1 second =
/// 1/86400 days (the midnight crossing); the year-end date displays as
/// the canonical Z-form in UTC.
#[test]
fn n595_midnight_crossing_and_utc_display_parity() {
    let src = program(
        "  let a = date_parse_iso(\"2026-01-01T00:00:00Z\")\n  let b = date_parse_iso(\"2025-12-31T23:59:59Z\")\n  let d = date_diff_days(a, b)\n  return to_string(d) + \"|\" + date_format_iso(a) + \"|\" + date_format_iso(b)",
    );
    let out = assert_parity("n595_midnight", &src);
    let parts: Vec<&str> = out.split('|').map(str::trim).collect();
    assert_eq!(parts.len(), 3, "layout: days|fmt_a|fmt_b, got {}", out);
    let days: f64 = parts[0]
        .parse()
        .unwrap_or_else(|e| panic!("days {:?} must parse: {}", parts[0], e));
    assert_close(days, 1.0 / 86400.0, "the midnight crossing in days");
    assert_eq!(
        parts[1], "2026-01-01T00:00:00Z",
        "the year-end display (UTC)"
    );
    assert_eq!(
        parts[2], "2025-12-31T23:59:59Z",
        "the year-end display (UTC)"
    );
}

/// The leap-year row through the language: 2024-02-29 parses; 2023-02-29
/// does NOT exist and refuses loudly on BOTH backends; the round trip
/// closes on the leap date.
#[test]
fn n595_leap_year_rows_on_both_backends() {
    let ok = program("  return to_string(date_parse_iso(\"2024-02-29T00:00:00Z\"))");
    let out = assert_parity("n595_leap_ok", &ok);
    let got: f64 = out
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("parse: {}", e));
    assert_close(got, 1709164800.0, "2024-02-29T00:00:00Z");

    let bad = program("  return to_string(date_parse_iso(\"2023-02-29T00:00:00Z\"))");
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(&bad, &base_dir)),
        ("VM", run_vm(&bad, &base_dir)),
    ] {
        let err = res.expect_err("2023-02-29 does not exist");
        assert!(
            err.starts_with("[DATE_INVALID] "),
            "{}: got: {}",
            backend,
            err
        );
    }
}

/// The loud refusal row through the language: month 13, day 30 of a
/// 28-day February, garbage — [DATE_INVALID] on both backends; `try`
/// classifies to the typed code DATE_INVALID on both backends.
#[test]
fn n595_invalid_dates_loud_and_typed_on_both_backends() {
    for bad in ["2026-13-01T00:00:00Z", "2026-02-30T00:00:00Z", "garbage"] {
        let src = program(&format!("  return to_string(date_parse_iso(\"{}\"))", bad));
        let base_dir = PathBuf::from("examples");
        for (backend, res) in [
            ("TW", run_tw(&src, &base_dir)),
            ("VM", run_vm(&src, &base_dir)),
        ] {
            let err = res.expect_err("an invalid date must refuse loudly");
            assert!(
                err.starts_with("[DATE_INVALID] "),
                "{} ({}): got: {}",
                bad,
                backend,
                err
            );
        }
    }
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try date_parse_iso("garbage")
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(src, &base_dir)),
        ("VM", run_vm(src, &base_dir)),
    ] {
        let out = res
            .unwrap_or_else(|e| panic!("{}: try must capture the refusal: {}", backend, e))
            .unwrap_or_else(|| panic!("{}: the pattern must return", backend));
        assert_eq!(
            out.trim(),
            "DATE_INVALID",
            "{}: typed code mismatch",
            backend
        );
    }
}

/// now_unix: the read sits inside the wall-clock envelope of the test
/// process; TW and VM reads agree within the harness overhead (the honest
/// parity statement for a wall-clock read — the two backend runs are
/// independent reads of a moving clock).
#[test]
fn n595_now_unix_within_wall_clock_envelope() {
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let src = program("  return to_string(now_unix())");
    let base_dir = PathBuf::from("examples");
    let tw = run_tw(&src, &base_dir).expect("TW run");
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let vm = run_vm(&src, &base_dir).expect("VM run");
    // NO byte-parity assert here: the two backend runs are independent
    // reads of a MOVING clock (the honest parity statement for a
    // wall-clock read is the envelope + the mutual agreement, not byte
    // equality). Every other n595 program pins byte parity.
    let parse_out = |out: &Option<String>, what: &str| -> f64 {
        out.as_deref()
            .unwrap_or_else(|| panic!("{}: the pattern must return", what))
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("{}: output must parse: {}", what, e))
    };
    let tw_read = parse_out(&tw, "TW");
    let vm_read = parse_out(&vm, "VM");
    for (read, what) in [(tw_read, "TW"), (vm_read, "VM")] {
        assert!(
            read >= before.floor() - 1.0 && read <= after.ceil() + 1.0,
            "{} now_unix {} outside the process envelope [{}, {}]",
            what,
            read,
            before,
            after
        );
    }
    assert!(
        (tw_read - vm_read).abs() < 60.0,
        "the two backend reads must agree within the harness overhead: {} vs {}",
        tw_read,
        vm_read
    );
}

/// The diff arithmetic the consumer needs: the fractional day difference
/// of the golden 12:34:56 vs midnight = 45296/86400 days; the reversed
/// arguments flip the sign exactly.
#[test]
fn n595_diff_days_signed_fractional_parity() {
    let src = program(
        "  let a = date_parse_iso(\"2026-10-05T12:34:56Z\")\n  let b = date_parse_iso(\"2026-10-05T00:00:00Z\")\n  let d1 = date_diff_days(a, b)\n  let d2 = date_diff_days(b, a)\n  return to_string(d1) + \"|\" + to_string(d2)",
    );
    let out = assert_parity("n595_diff", &src);
    let parts: Vec<&str> = out.split('|').map(str::trim).collect();
    assert_eq!(parts.len(), 2);
    let d1: f64 = parts[0].parse().unwrap_or_else(|e| panic!("parse: {}", e));
    let d2: f64 = parts[1].parse().unwrap_or_else(|e| panic!("parse: {}", e));
    assert!((d1 - 45296.0 / 86400.0).abs() < 1e-9, "d1 = {}", d1);
    assert!((d1 + d2).abs() < 1e-12, "the sign must flip exactly");
}
