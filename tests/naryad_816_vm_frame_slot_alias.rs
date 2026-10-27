// ── tests/naryad_816_vm_frame_slot_alias.rs ──────────────────────────
// Наряд №816 (issue #1584, wave 67, P1, release-block): the VM frame-slot
// alias divergence — the compiler's per-arm `next_slot` rollback handed a
// NEW binding the slot of a LIVE binding from an earlier if-arm (the map
// is never rolled back, the counter was), so two locals shared one stack
// cell and a pattern-call result was replaced by an unrelated string
// (the #1555 self-parse symptom: `get() requires List as first argument`
// deep in the ParseWhile→ParseBlock→ParseStmt→ParseIfStmt chain).
//
// Two pins (the №772 RED-before form):
//   1. the minimal slot-alias repro — a NEW binding after an if-arm must
//      NOT share the arm binding's slot (RED on unpatched main: the VM
//      answers 9 where the TW answers 2);
//   2. the #1555 headline: `self-host/parser.mlog` self-parses IDENTICALLY
//      on the VM lane and the TW reference lane (RED on unpatched main:
//      the VM lane errors out mid-recursion).

use std::process::Command;
use std::sync::OnceLock;

static MLOG_BIN: OnceLock<String> = OnceLock::new();

fn mlog_bin() -> &'static str {
    MLOG_BIN.get_or_init(|| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&manifest_dir)
            .join("target")
            .join("debug")
            .join("mlog")
            .to_string_lossy()
            .into_owned()
    })
}

fn run_lane(
    program_path: &std::path::Path,
    reference: bool,
    parse_target: Option<&str>,
) -> (bool, String, String) {
    let mut cmd = Command::new(mlog_bin());
    cmd.arg("run");
    if reference {
        cmd.arg("--reference");
    }
    if let Some(target) = parse_target {
        cmd.env("MLOG_PARSE_TARGET", target);
    }
    let out = cmd
        .arg(program_path)
        .env("METALOGOS_SENSITIVE_PATH_ALLOWLIST", "parser.mlog")
        .output()
        .expect("failed to spawn mlog process");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const SLOT_ALIAS_REPRO_SEED: &str = "tests/fuzz_corpus/seed_frame_slots.mlog";

#[test]
fn naryad_816_frame_slot_alias_new_binding_after_if_arm() {
    // The repro lives in the differential-fuzzer corpus (the №783 form —
    // the seed IS the minimal class shape); the test points the CLI at it
    // (no fs calls — the №475 ratchet).
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let program = std::path::Path::new(&manifest_dir).join(SLOT_ALIAS_REPRO_SEED);
    assert!(program.exists(), "corpus seed missing: {:?}", program);

    let (vm_ok, vm_out, vm_err) = run_lane(&program, false, None);
    let (tw_ok, tw_out, tw_err) = run_lane(&program, true, None);
    assert!(
        tw_ok,
        "TW reference lane must answer the repro:\n{}",
        tw_err
    );
    assert!(vm_ok, "VM lane failed on the slot-alias repro:\n{}", vm_err);

    let parse = |s: &str| {
        s.trim()
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .parse::<f64>()
            .unwrap_or(f64::NAN)
    };
    let vm_val = parse(&vm_out);
    let tw_val = parse(&tw_out);
    assert_eq!(
        vm_val, tw_val,
        "VM diverged from the TW on the slot-alias repro (vm_out={:?}, tw_out={:?}) — \
         a binding after an if-arm must not share the arm binding's slot",
        vm_out, tw_out
    );
    assert_eq!(
        vm_val, 2.0,
        "the arm binding's value must survive: get(a, 1) == 2.0 (vm_out={:?})",
        vm_out
    );
}

#[test]
fn naryad_816_parser_self_parse_vm_matches_reference() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let parser_path = std::path::Path::new(&manifest_dir).join("self-host/parser.mlog");
    assert!(
        parser_path.exists(),
        "self-host/parser.mlog not found at {:?}",
        parser_path
    );

    // The #1555 shape: the parser parses ITS OWN source; both lanes must
    // succeed and agree byte-for-byte. The TW lane takes minutes on the
    // debug build (the deeply recursive Tokenize chain) — the 12-minute
    // budget mirrors the №197 bootstrap test.
    let (vm_ok, vm_out, vm_err) = run_lane(&parser_path, false, Some("self-host/parser.mlog"));
    assert!(
        vm_ok,
        "VM lane failed the self-parse (the #1555 divergence):\n{}",
        vm_err
    );
    let (tw_ok, tw_out, tw_err) = run_lane(&parser_path, true, Some("self-host/parser.mlog"));
    assert!(
        tw_ok,
        "TW reference lane failed the self-parse:\n{}",
        tw_err
    );
    assert!(
        !vm_out.trim().is_empty(),
        "VM self-parse produced empty output"
    );
    assert_eq!(
        vm_out, tw_out,
        "the VM self-parse output diverged from the TW reference"
    );
}
