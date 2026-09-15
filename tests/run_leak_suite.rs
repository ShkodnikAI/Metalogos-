// ── Leak-suite runner (Наряд №317, issue #404) ─────────────────────────
//
// Corpus contract: examples/leak/n*.mlog are programs that MUST NOT
// compile once the label lattice lands (Фаза 1, №325); each n*.error file
// fixes the expected CLASS of the compile failure in the format:
//
//     EXPECTED: <CLASS> — <сценарий>
//
// examples/leak/ok_*.mlog are legal flows (redact-before-sink, local log
// for private data, plain public transforms) that MUST keep compiling AND
// running — before and after №325 — with the output fixed by ok_*.expected.
//
// CLASS vocabulary (two groups):
// 1. Existing audit check_ids (caught TODAY on the compile path, №98
//    promotion): SECRET_LEAK, HTML_INJECTION, UNTRUSTED_FRAME,
//    MEDIA_SYNTHETIC_UNMARKED, TAINT_PERSISTENCE, SQL_DYNAMIC, ...
// 2. Planned lattice classes for scenarios NOT caught yet (the №325
//    label-checker must produce exactly these): PII_EGRESS_NETWORK,
//    PII_EGRESS_OUTPUT, VOICE_EGRESS_UNCONSENTED, UNTRUSTED_EXEC_DECISION,
//    SECRET_TO_EXEC, UNTRUSTED_SQL, IRREVERSIBLE_NO_GRANT,
//    SECRET_EGRESS_VCS, SECRET_EGRESS_NETWORK, UNTRUSTED_EGRESS_NETWORK.
//
// Mode: до №325 раннер ОТЧЁТНЫЙ — «not caught» is printed red-by-design
// and does NOT fail the test (pre-Phase-1 the hole is expected to be
// open). С №325 — один атрибут: BLOCKING = true → «not caught» fails.
// Class MISMATCH (a negative fails for a FOREIGN reason, or a positive
// output drifts) fails in BOTH modes: that is a corpus bug, not a hole.
//
// The corpus lives in examples/leak/ — OUTSIDE the main golden cycle
// (tests/golden.rs scans examples/ non-recursively by design).

use std::fs;
use std::path::{Path, PathBuf};

/// One-attribute blocking switch (№325 flipped this to true — the gate
/// is live; see tests/naryad_325_sink_clearance.rs for the gate contract).
const BLOCKING: bool = true;

const MIN_NEGATIVES: usize = 25;
const MIN_POSITIVES: usize = 15;

fn leak_dir() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    Path::new(&manifest).join("examples").join("leak")
}

/// Parse `EXPECTED: <CLASS> — <сценарий>` from an .error file.
fn parse_expected(path: &Path) -> Result<(String, String), String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("EXPECTED:") {
            let rest = rest.trim();
            let (class, scenario) = match rest.split_once('—') {
                Some((c, s)) => (c.trim().to_string(), s.trim().to_string()),
                None => (rest.to_string(), String::new()),
            };
            return Ok((class.to_uppercase(), scenario));
        }
    }
    Err(format!(
        "{}: missing EXPECTED line (format: EXPECTED: <CLASS> — <сценарий>)",
        path.display()
    ))
}

/// Extract the failure CLASS from a compile error: the `[CHECK_ID]`
/// prefix of audit findings (№98 promotion format), else "COMPILE".
fn error_class(err: &str) -> String {
    for line in err.lines() {
        if let Some(start) = line.find('[') {
            if let Some(end_rel) = line[start + 1..].find(']') {
                let id = &line[start + 1..start + 1 + end_rel];
                let is_id = !id.is_empty()
                    && id
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
                if is_id {
                    return id.to_string();
                }
            }
        }
    }
    "COMPILE".to_string()
}

#[test]
fn leak_negatives_report() {
    let dir = leak_dir();
    assert!(dir.is_dir(), "examples/leak/ must exist");

    let mut negatives: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read examples/leak")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "mlog").unwrap_or(false)
                && p.file_stem()
                    .map(|s| s.to_string_lossy().starts_with('n'))
                    .unwrap_or(false)
        })
        .collect();
    negatives.sort();

    assert!(
        negatives.len() >= MIN_NEGATIVES,
        "corpus must hold >= {} negatives, found {}",
        MIN_NEGATIVES,
        negatives.len()
    );

    let mut caught = 0usize;
    let mut not_caught = 0usize;
    let mut mismatches: Vec<String> = Vec::new();

    for mlog in &negatives {
        let name = mlog.file_name().unwrap().to_string_lossy().to_string();
        let error_file = mlog.with_extension("error");
        if !error_file.exists() {
            mismatches.push(format!("{}: missing .error contract file", name));
            continue;
        }
        let (expected_class, scenario) = match parse_expected(&error_file) {
            Ok(v) => v,
            Err(e) => {
                mismatches.push(e);
                continue;
            }
        };
        let source = fs::read_to_string(mlog).unwrap();
        match metalogos::compile_program(&source) {
            Ok(_) => {
                // The hole is open for this scenario (expected pre-№325).
                not_caught += 1;
                eprintln!(
                    "  NOT CAUGHT: {} — expected class {} ({})",
                    name, expected_class, scenario
                );
            }
            Err(err) => {
                let got = error_class(&err);
                if got == expected_class {
                    caught += 1;
                    eprintln!("  caught: {} — class {} ({})", name, got, scenario);
                } else {
                    // A negative must never fail for a FOREIGN reason.
                    mismatches.push(format!(
                        "{}: class mismatch — expected {}, got {} ({}); error: {}",
                        name,
                        expected_class,
                        got,
                        scenario,
                        err.lines().next().unwrap_or("")
                    ));
                }
            }
        }
    }

    eprintln!(
        "leak-suite negatives: {} caught / {} NOT caught (reporting {}); mismatches: {}",
        caught,
        not_caught,
        if BLOCKING { "OFF→BLOCKING" } else { "MODE" },
        mismatches.len()
    );

    if !mismatches.is_empty() {
        panic!(
            "{} corpus integrity violation(s):\n{}",
            mismatches.len(),
            mismatches
                .iter()
                .map(|m| format!("  - {}", m))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    if BLOCKING && not_caught > 0 {
        panic!(
            "BLOCKING mode (№325): {} negative(s) compiled but must not — the lattice \
             does not catch them yet",
            not_caught
        );
    }
}

#[test]
fn leak_positives_pass() {
    let dir = leak_dir();

    let mut positives: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read examples/leak")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "mlog").unwrap_or(false)
                && p.file_stem()
                    .map(|s| s.to_string_lossy().starts_with("ok_"))
                    .unwrap_or(false)
        })
        .collect();
    positives.sort();

    assert!(
        positives.len() >= MIN_POSITIVES,
        "corpus must hold >= {} positives, found {}",
        MIN_POSITIVES,
        positives.len()
    );

    let mut failures: Vec<String> = Vec::new();

    for mlog in &positives {
        let name = mlog.file_name().unwrap().to_string_lossy().to_string();
        let expected_file = mlog.with_extension("expected");
        if !expected_file.exists() {
            failures.push(format!("{}: missing .expected file", name));
            continue;
        }
        let expected = fs::read_to_string(&expected_file).unwrap();
        if expected.trim() == "TBD" {
            failures.push(format!("{}: .expected not calibrated (TBD)", name));
            continue;
        }
        let source = fs::read_to_string(mlog).unwrap();
        match metalogos::run_program(&source) {
            Ok(actual) => {
                let actual = actual.unwrap_or_default();
                if actual.trim_end() != expected.trim_end() {
                    failures.push(format!(
                        "{}: output mismatch\n  expected: {:?}\n  actual:   {:?}",
                        name,
                        expected.trim_end(),
                        actual.trim_end()
                    ));
                }
            }
            Err(e) => failures.push(format!("{}: runtime error: {}", name, e)),
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} positive flow(s) FAILED ({} passed):\n{}",
            failures.len(),
            positives.len() - failures.len(),
            failures
                .iter()
                .map(|f| format!("  - {}", f))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Calibration dump (dev tooling, #[ignore]): prints the actual failure
/// class for EVERY negative without any contract comparison. Run:
/// `cargo test --test run_leak_suite leak_corpus_calibration_dump -- --ignored --nocapture`
#[test]
#[ignore = "calibration tooling for the leak corpus (№317)"]
fn leak_corpus_calibration_dump() {
    let dir = leak_dir();
    let mut negatives: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read examples/leak")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "mlog").unwrap_or(false)
                && p.file_stem()
                    .map(|s| s.to_string_lossy().starts_with('n'))
                    .unwrap_or(false)
        })
        .collect();
    negatives.sort();
    for mlog in &negatives {
        let name = mlog.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(mlog).unwrap();
        match metalogos::compile_program(&source) {
            Ok(_) => println!("{}: COMPILES (hole open)", name),
            Err(err) => println!("{}: class {} | {}", name, error_class(&err), err),
        }
    }
}
