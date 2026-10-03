// ── tests/naryad_559_registry_vs_corpus.rs ───────────────────────────
// №559 (Wave 25 P1; the audit 02.10 M-2 systematic + §6.3; dispatch
// gh#925): the corpus contract is machine-checked.
//
// The respond_html lesson: a change that turns a warning into an error
// (a spec hardening) must run against the KNOWN corpus, not wait for a
// user. The machinery pinned here:
//   - scripts/ci/office_call_shapes.txt — the corpus snapshot (the
//     name|argc|count triples; examples/ + self-host/ now, the office
//     snapshot is the owner's hand-off — no code, no query strings);
//   - scripts/ci/registry_arity.txt — the builtin-registry arity export
//     (the machine-read SSOT the Python validator reads);
//   - scripts/ci/registry_vs_corpus.py — the blocking validator.
//
// THIS file pins the syncs: the export moves with BUILTIN_REGISTRY (a
// registry edit without the export = a failure here, and vice versa —
// the №551-sync posture), the dynamic-arity list stays ONE posture with
// semantic.rs, the Python self-test stays green, and the COMMITTED
// corpus is accepted by the registry TODAY (the snapshot cannot drift
// from the registry through a merge).
#![allow(clippy::disallowed_methods)]

use std::process::Command;

const REGISTRY_EXPORT: &str = include_str!("../scripts/ci/registry_arity.txt");
const SEMANTIC_SRC: &str = include_str!("../src/semantic.rs");

fn build_export() -> String {
    let mut lines: Vec<String> = vec![
        "# №559 (issue #920): the builtin-registry arity export — the machine-read".into(),
        "# SSOT the corpus check (scripts/ci/registry_vs_corpus.py) validates".into(),
        "# against. Format: name|min|max (max \"*\" = unbounded variadic; max == min".into(),
        "# = exact). Pinned by tests/naryad_559_registry_vs_corpus.rs — the file".into(),
        "# moves with BUILTIN_REGISTRY, never alone. The dynamic-arity names".into(),
        "# (forget, render — the semantic.rs dynamic_arity list) are listed in the".into(),
        "# header below and SKIPPED by the corpus check (the builtin is the loud".into(),
        "# runtime validator; one spec cannot state their contract).".into(),
        format!("# dynamic-arity: {}", crate_dynamic_arity().join(", ")),
    ];
    let mut specs: Vec<(String, usize, Option<usize>)> = metalogos::builtins::BUILTIN_REGISTRY
        .iter()
        .map(|s| (s.name.to_string(), s.arity, s.max_arity))
        .collect();
    specs.sort();
    specs.dedup();
    for (name, amin, amax) in specs {
        let max_repr = match amax {
            None => {
                if amin == 0 {
                    "*".to_string() // truly variadic
                } else {
                    amin.to_string() // exact (defensive: arity>0 with no max)
                }
            }
            Some(m) => m.to_string(),
        };
        lines.push(format!("{name}|{amin}|{max_repr}"));
    }
    lines.join("\n") + "\n"
}

fn crate_dynamic_arity() -> Vec<&'static str> {
    vec!["forget", "render"]
}

#[test]
fn n559_registry_arity_export_is_in_sync() {
    let expected = build_export();
    if std::env::var("METALOGOS_N559_BLESS").as_deref() == Ok("1") {
        // the dev-only bless path: regenerate the committed export
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/ci/registry_arity.txt");
        std::fs::write(path, &expected).expect("the bless write must succeed");
        println!("blessed {}", path);
        return;
    }
    assert!(
        REGISTRY_EXPORT == expected,
        "scripts/ci/registry_arity.txt is STALE against BUILTIN_REGISTRY — \
         re-run with METALOGOS_N559_BLESS=1 cargo test --test naryad_559_registry_vs_corpus \
         (the file moves with the registry, never alone — №559)"
    );
}

#[test]
fn n559_dynamic_arity_list_stays_one_posture_with_semantic() {
    // The semantic.rs dynamic_arity list is the source of the posture; the
    // export header mirrors it. A semantic-side edit without the export
    // (or vice versa) fails here.
    for name in crate_dynamic_arity() {
        assert!(
            SEMANTIC_SRC.contains(&format!("\"{name}\"")),
            "the dynamic-arity name '{name}' must stay in semantic.rs's \
             dynamic_arity list (the ONE named place) — or be removed from \
             the export header in the same PR"
        );
        assert!(
            REGISTRY_EXPORT.contains(&format!(
                "# dynamic-arity: {}",
                crate_dynamic_arity().join(", ")
            )),
            "the export header must pin the dynamic-arity list verbatim"
        );
    }
}

#[test]
fn n559_validator_self_test_is_green() {
    let out = Command::new("python3")
        .arg("scripts/ci/registry_vs_corpus.py")
        .arg("--self-test")
        .output()
        .expect("python3 must exist (the CI image runs the gate scripts)");
    assert!(
        out.status.success(),
        "the registry-vs-corpus self-test must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("SELF-TEST OK"),
        "the self-test must print SELF-TEST OK, got: {stdout}"
    );
    for case in [
        "green",
        "arity-divergence",
        "not-a-builtin-skipped",
        "dynamic-arity-skipped",
        "range-accepts",
        "range-refuses",
        "skip-annotation-roundtrip",
    ] {
        assert!(
            stdout.contains(case),
            "the self-test must cover the '{case}' case, got: {stdout}"
        );
    }
}

#[test]
fn n559_committed_corpus_is_accepted_by_the_registry() {
    // The live check against the COMMITTED snapshot: every registry-bound
    // pair is accepted TODAY. A spec hardening that breaks a corpus call
    // turns this red — before the merge, not after a user hits it.
    let out = Command::new("python3")
        .arg("scripts/ci/registry_vs_corpus.py")
        .output()
        .expect("python3 must exist");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "the committed corpus must be accepted by the registry (the №559 \
         blocking contract), output: {stdout}",
    );
    assert!(
        stdout.contains("registry-vs-corpus: OK"),
        "the validator must print the OK verdict, got: {stdout}"
    );
}
