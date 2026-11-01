#![allow(clippy::disallowed_methods)]

// ── Naryad №868 (wave 76 — W-E, P1, [compliance]): the evidence-package
// compiler — the deterministic, digest-pinned export of the attestation
// map (ADR-0206 §2.4/§4) ────────────────────────────────────────────────
//
// The done-when, pinned here:
//   - the compiler is in main: the seed map (45 rows over the LIVE
//     in-repo sources — the frozen vocabulary, the machine gates, the
//     baselines, the golden ledger trace, the parent ADRs and the
//     parent SSOT files) compiles into docs/compliance/attestation-map-v1.txt;
//   - the e2e trace action → Ledger → attestation → map row → package
//     green (the cron-mandate arc of №861 runs on BOTH backends, the
//     Ledger's persistent golden form is attested, the row lands in
//     the package);
//   - the double compiler run is byte-identical (the determinism
//     contract, ADR-0206 §2.4);
//   - the fail-closed controls: a live row with a drifted evidence
//     digest refuses the compilation; a space in a v1 field refuses;
//     the digest round-trip refuses an edited package.
//
// REGENERATION (the ride form of №863 — «the map follows the fact»):
// when a referenced source changes, the package is re-compiled in the
// SAME PR:  METALOGOS_GEN_COMPLIANCE=1 cargo test --test naryad_868_package
// then commit the refreshed docs/compliance/attestation-map-v1.txt.

use metalogos::compliance::{Attestation, AttestationMap, Evidence, EvidenceKind};
use metalogos::compliance_pack::{
    compile_package, count_live_rows, count_raw_rows, package_digest_of, parse_row_line,
    verify_row_digests,
};
use sha2::{Digest, Sha256};

const PACKAGE_PATH: &str = "docs/compliance/attestation-map-v1.txt";
const GEN_ENV: &str = "METALOGOS_GEN_COMPLIANCE";

fn sha256_file(path: &str) -> String {
    let bytes =
        std::fs::read(path).unwrap_or_else(|e| panic!("the seed source {path} must exist: {e}"));
    let d = Sha256::digest(&bytes);
    let mut hex = String::with_capacity(d.len() * 2);
    for b in d {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    let mut hex = String::with_capacity(d.len() * 2);
    for b in d {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

/// The live-source resolver: every seed reference is a repo-relative
/// file path; the live digest is the file's SHA-256 (the G5 form — the
/// parent bytes are the SSOT, the map pins them).
fn live_resolver(kind: &str, reference: &str) -> Result<String, String> {
    let _ = kind; // the v1 resolver is path-based; the kind rides the row
    if std::path::Path::new(reference).exists() {
        Ok(sha256_file(reference))
    } else {
        Err(format!("the referenced source {reference} does not exist"))
    }
}

/// The seed map: the toolchain attests its own compliance surface.
/// Every row pins the CURRENT bytes of a live in-repo source (the
/// claims come from the EXISTING vocabularies — the gates, the
/// baselines, the golden suites; ADR-0206 §2.1).
fn seed_map() -> AttestationMap {
    let mut map = AttestationMap::new();
    let mut issue = |claim: String,
                     subject: &str,
                     kind: EvidenceKind,
                     reference: &str,
                     verifier: &str,
                     map: &mut AttestationMap| {
        let digest = sha256_file(reference);
        let att = Attestation::issue(
            "office-machine",
            subject,
            &[kind.as_str(), reference],
            &claim,
            Evidence {
                kind,
                reference: reference.to_string(),
                digest,
            },
            verifier,
            0,
            0,
            &|_| Ok(()),
        )
        .expect("the seed row issues");
        map.issue(att).expect("the seed row appends");
    };

    // (1) the frozen 14-word refusal vocabulary (№385/ADR-0169) — one
    // row per word, the parity-pinned source file as the evidence.
    for word in metalogos::refusal_algebra::DENY_REASONS.iter() {
        issue(
            format!("vocabulary.frozen.{word}"),
            "the-deny-reason-vocabulary",
            EvidenceKind::Artifact,
            "src/refusal_algebra.rs",
            "the-14-word-parity-pins",
            &mut map,
        );
    }
    // (2) the machine gates 028–036 (the release/goal records).
    for gate in [
        "gate_028_goals.txt",
        "gate_029_goals.txt",
        "gate_030_goals.txt",
        "gate_031_goals.txt",
        "gate_032_untyped_sources.txt",
        "gate_033_goals.txt",
        "gate_034_goals.txt",
        "gate_035_goals.txt",
        "gate_036_goals.txt",
    ] {
        let reference = format!("scripts/ci/{gate}");
        issue(
            format!("gate.record.{}", gate.trim_end_matches(".txt")),
            "the-machine-gate-records",
            EvidenceKind::Artifact,
            &reference,
            "unfreeze-gate-strict",
            &mut map,
        );
    }
    // (3) the only-up baselines (the floors the release gate reads).
    for baseline in [
        "type_signature_baseline.txt",
        "type_signature_parameterized_baseline.txt",
        "sensitive_surface_baseline.txt",
        "coverage_baseline.txt",
        "tw_vm_dup_names_baseline.txt",
        "ops_pair_baseline.txt",
        "src_mirror_baseline.txt",
        "debt_baseline.txt",
    ] {
        let reference = format!("scripts/ci/{baseline}");
        issue(
            format!("baseline.record.{}", baseline.trim_end_matches(".txt")),
            "the-only-up-baselines",
            EvidenceKind::Artifact,
            &reference,
            "the-gate-floor-checks",
            &mut map,
        );
    }
    // (4) the golden Ledger trace — the persistent record form of the
    // ACTION trace (the ADR-0167 binding, the №847 e2e form).
    issue(
        "ledger.golden.w72-capability-trace".to_string(),
        "the-action-ledger-golden",
        EvidenceKind::Ledger,
        "examples/w72_capability_trace.ledger.golden",
        "cargo-test-ledger-golden",
        &mut map,
    );
    issue(
        "ledger.golden.w72-expected".to_string(),
        "the-action-ledger-golden",
        EvidenceKind::Ledger,
        "examples/w72_capability_trace.expected",
        "cargo-test-ledger-golden",
        &mut map,
    );
    // (5) the parent ADRs and the parent SSOT files — the compliance
    // parents this contour references, never rewrites.
    for (claim, subject, kind, path, verifier) in [
        (
            "parent.adr-0167-ledger",
            "the-action-ledger-v1",
            EvidenceKind::Ledger,
            "docs/adr/0167-action-ledger-v1.md",
            "the-adr-registry",
        ),
        (
            "parent.adr-0196-gateway",
            "the-mcp-policy-gateway",
            EvidenceKind::Gateway,
            "docs/adr/0196-mcp-policy-gateway.md",
            "the-adr-registry",
        ),
        (
            "parent.adr-0202-passport",
            "the-model-passport",
            EvidenceKind::Passport,
            "docs/adr/0202-model-passport.md",
            "the-adr-registry",
        ),
        (
            "parent.adr-0203-capability",
            "the-capability-token",
            EvidenceKind::Capability,
            "docs/adr/0203-capability-token.md",
            "the-adr-registry",
        ),
        (
            "parent.adr-0205-mandate",
            "the-typed-mandate",
            EvidenceKind::Mandate,
            "docs/adr/0205-typed-approval.md",
            "the-adr-registry",
        ),
        (
            "parent.adr-0206-compliance",
            "the-compliance-evidence-compiler",
            EvidenceKind::Artifact,
            "docs/adr/0206-compliance-evidence-compiler.md",
            "the-adr-registry",
        ),
        (
            "parent.ssot.capability-rosters",
            "the-capability-dispositions",
            EvidenceKind::Capability,
            "src/capability.rs",
            "the-classification-totality-test",
        ),
        (
            "parent.ssot.gateway-policy",
            "the-mcp-policy-gateway",
            EvidenceKind::Gateway,
            "metalogos-server/src/mcp_policy.rs",
            "the-g5-digest-pinning",
        ),
        (
            "parent.ssot.passport-registry",
            "the-backend-registry",
            EvidenceKind::Passport,
            "src/backends.rs",
            "the-licensed-floor",
        ),
        (
            "parent.ssot.mandate-kernel",
            "the-typed-mandate",
            EvidenceKind::Mandate,
            "src/approval.rs",
            "the-861-approval-gate",
        ),
        (
            "parent.ssot.compliance-kernel",
            "the-attestation-kernel",
            EvidenceKind::Artifact,
            "src/compliance.rs",
            "the-867-negative-controls",
        ),
        (
            "doc.upgrade-0.34-to-0.38",
            "the-migration-recipe",
            EvidenceKind::Artifact,
            "docs/upgrade.md",
            "the-865-migration-test",
        ),
    ] {
        issue(claim.to_string(), subject, kind, path, verifier, &mut map);
    }
    map
}

// ── the determinism + the pinning ──────────────────────────────────────

#[test]
fn the_double_run_is_byte_identical_and_the_package_is_pinned() {
    let map = seed_map();
    assert!(
        map.len() >= 30,
        "the seed map carries at least the 0.39 floor of rows, got {}",
        map.len()
    );
    let run1 = compile_package(&map, 0, &live_resolver).expect("the seed compiles");
    let run2 = compile_package(&map, 0, &live_resolver).expect("the seed compiles");
    assert_eq!(
        run1, run2,
        "the determinism contract: two compilations of the same inputs are byte-identical"
    );
    // the structural counters ride the raw lines (the №565 lesson)
    assert_eq!(count_raw_rows(&run1), map.len());
    assert_eq!(
        count_live_rows(&run1),
        map.len(),
        "all the seed rows are live"
    );
    // the package digest in the header matches the row block
    let header_digest = run1
        .lines()
        .find(|l| l.starts_with("# package_digest: "))
        .unwrap()
        .trim_start_matches("# package_digest: ")
        .to_string();
    assert_eq!(package_digest_of(&run1).unwrap(), header_digest);

    // the pinning: the checked-in package is byte-identical to a fresh
    // compilation of the live sources (any drift is a LOUD failure —
    // re-attest in the same PR, the №863 form)
    if std::env::var(GEN_ENV).is_ok() {
        std::fs::create_dir_all(std::path::Path::new(PACKAGE_PATH).parent().unwrap())
            .expect("the compliance dir");
        std::fs::write(PACKAGE_PATH, &run1).expect("the package rewrites");
        println!("REGENERATED {PACKAGE_PATH} ({} bytes)", run1.len());
    } else {
        let checked_in = std::fs::read_to_string(PACKAGE_PATH)
            .unwrap_or_else(|e| panic!("{PACKAGE_PATH} must exist: {e}"));
        assert_eq!(
            checked_in, run1,
            "the checked-in package drifted from the live sources — re-attest: {GEN_ENV}=1 cargo test --test naryad_868_package"
        );
    }
}

// ── the e2e trace: action → Ledger → attestation → row → package ──────

fn run_both_backends(source: &str) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // TW
    metalogos::run_program_with_dir(source.trim(), std::path::Path::new(manifest).to_path_buf())
        .expect("the TW lane runs the arc")
        .expect("the TW arc output");
    // VM
    let declarations = metalogos::parser::parse(source.trim()).expect("the parse");
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(std::path::Path::new(manifest).to_path_buf());
    let program = comp.compile(declarations).expect("the compile");
    metalogos::vm::Vm::new()
        .run(program)
        .expect("the VM lane runs the arc")
        .expect("the VM arc output");
}

#[test]
fn the_e2e_trace_action_ledger_attestation_row_package() {
    // (1) the ACTION under the live mandate — the №861 arc on BOTH
    // backends (the gates pass; the Ledger journals the trace).
    let arc = r#"
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N868Archive")
  let _m = approval_issue("owner", "carrier:cron_run", 600)
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output }
"#;
    run_both_backends(arc);
    metalogos::approval::mandate_store_reset();

    // (2) the ATTESTATION over the Ledger's persistent form (the golden
    // trace — the record the action class produces), with a REAL
    // freshness window.
    let digest = sha256_file("examples/w72_capability_trace.ledger.golden");
    let att = Attestation::issue(
        "office-machine",
        "the-w72-ledger-trace",
        &["ledger", "w72_capability_trace.ledger.golden"],
        "ledger.golden.w72-live-window",
        Evidence {
            kind: EvidenceKind::Ledger,
            reference: "examples/w72_capability_trace.ledger.golden".to_string(),
            digest,
        },
        "cargo-test-ledger-golden",
        1_000,
        600,
        &|_| Ok(()),
    )
    .expect("the e2e attestation issues");
    let mut map = AttestationMap::new();
    map.issue(att).expect("the row appends");

    // (3) the ROW in the package — live inside the window.
    let pkg = compile_package(&map, 1_100, &live_resolver).expect("compiles");
    assert_eq!(count_raw_rows(&pkg), 1);
    assert_eq!(count_live_rows(&pkg), 1);
    assert!(pkg.contains("ledger.golden.w72-live-window"));
    // the digest round-trip: the consumer re-verifies the row digests
    assert_eq!(verify_row_digests(&pkg).unwrap(), 1);

    // (4) the freshness verdict is the CONSUMER's date comparison
    // (ADR-0206 §3): past the window the row compiles as expired —
    // deterministically, without touching the pinned digest.
    let later = compile_package(&map, 1_000 + 601, &live_resolver).expect("compiles");
    assert_eq!(count_live_rows(&later), 0);
    assert!(
        later.contains(" expired "),
        "the expired status rides beside the digest"
    );

    // (5) the WITHDRAWAL: the revoked row stays (append-only) and
    // attests nothing — the status flips, the digest does not.
    let dg = map.rows()[0].row_digest.clone();
    map.revoke(&dg).unwrap();
    let revoked_pkg = compile_package(&map, 1_100, &live_resolver).expect("compiles");
    assert!(revoked_pkg.contains(" revoked "));
    assert_eq!(count_live_rows(&revoked_pkg), 0);
    assert_eq!(verify_row_digests(&revoked_pkg).unwrap(), 1);
}

// ── the fail-closed controls ───────────────────────────────────────────

#[test]
fn the_compiler_refuses_a_drifted_live_row_and_a_broken_line_contract() {
    let mut map = AttestationMap::new();
    let att = Attestation::issue(
        "actor",
        "s",
        &["s"],
        "c",
        Evidence {
            kind: EvidenceKind::Artifact,
            reference: "Cargo.toml".to_string(),
            digest: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        },
        "v",
        0,
        0,
        &|_| Ok(()),
    )
    .unwrap();
    map.issue(att).unwrap();
    // the live row's pinned digest does not match the live bytes → the
    // compilation refuses (the map never lies, ADR-0206 §2.5)
    let err = compile_package(&map, 0, &live_resolver).unwrap_err();
    assert!(
        err.contains("does not match the source"),
        "the drifted live row refuses the compilation, got: {err}"
    );
    // the unresolvable parent refuses too (the dangling reference)
    let err = compile_package(&map, 0, &|_, _| Err("no such artifact".into())).unwrap_err();
    assert!(
        err.contains("refuses"),
        "the unresolvable parent refuses, got: {err}"
    );

    // the v1 line contract: a space in a single-line field refuses
    let bad = Attestation::issue(
        "actor with spaces",
        "s",
        &["s"],
        "c",
        Evidence {
            kind: EvidenceKind::Artifact,
            reference: "Cargo.toml".to_string(),
            digest: sha256_file("Cargo.toml"),
        },
        "v",
        0,
        0,
        &|_| Ok(()),
    )
    .unwrap();
    let mut bad_map = AttestationMap::new();
    bad_map.issue(bad).unwrap();
    let err = compile_package(&bad_map, 0, &live_resolver).unwrap_err();
    assert!(
        err.contains("line contract"),
        "the space-bearing field refuses the v1 serialization, got: {err}"
    );
}

#[test]
fn the_consumer_refuses_an_edited_package() {
    let map = seed_map();
    let pkg = compile_package(&map, 0, &live_resolver).expect("compiles");
    // a tampered row: swap a claim word in one line — the pinned digest
    // no longer matches the re-derivation
    let tampered = pkg.replacen(
        "subject=the-deny-reason-vocabulary",
        "subject=the-EDITED",
        1,
    );
    assert_ne!(tampered, pkg, "the tamper applied");
    let err = verify_row_digests(&tampered).unwrap_err();
    assert!(
        err.contains("no longer matches its pinned digest")
            || err.contains("does not re-digest from its own fields"),
        "the edited package refuses the integrity check, got: {err}"
    );
    // the round-trip: every clean row parses back to its kernel value
    let rows: Vec<&str> = pkg.lines().filter(|l| l.starts_with("row ")).collect();
    assert!(!rows.is_empty());
    for r in rows {
        let att = parse_row_line(r).expect("the clean row round-trips");
        assert_eq!(att.row_digest.len(), 64);
    }
}
