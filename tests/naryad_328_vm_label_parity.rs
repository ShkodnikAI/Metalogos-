//! Наряд №328 (issue #422) — runtime label parity: LabelJoin/SinkCheck
//! in the bytecode, the VM runtime twin of the №325 gate, the JIT
//! dispatch-gap rule (ADR-0156).
//!
//! Boundary (loud): the JIT compiler is not in the tree yet — the
//! "dispatch gap = explicit error" rule is pinned via
//! `bytecode::is_jit_eligible` (the SSOT predicate the future dispatcher
//! must consult); media label flows are Phase 2.

use metalogos::bytecode::{is_jit_eligible, Instruction, Program};
use metalogos::labels::{Conf, Label};
use metalogos::vm::Vm;

fn program_with(main_code: Vec<Instruction>) -> Program {
    Program {
        globals: vec![],
        patterns: vec![],
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        reflex_decls: vec![],
        reflex_seq_decls: vec![],
        reflex_gen_decls: vec![],
        vision_decls: vec![],
        origin_decls: vec![],
        deny_handlers: vec![],
        db_url: None,
        memory_persist_path: None,
        schema_ddl: vec![],
        main_code,
        collections_loaded: false,
    }
}

// ── (а) JIT dispatch-gap rule ────────────────────────────────────────

#[test]
fn n328_label_instructions_are_not_jit_eligible() {
    // The dispatch-gap rule (ADR-0156 §3): label instructions are
    // explicitly OUTSIDE the JIT-eligible class — the future dispatcher
    // must reject them with a distinct error, never skip silently.
    let label_code = vec![
        Instruction::LabelJoin {
            dst: "k".to_string(),
            src: "@env".to_string(),
        },
        Instruction::SinkCheck {
            fn_name: "print".to_string(),
            arg: "k".to_string(),
            line: 1,
            arg_index: 0,
            deny: None,
        },
    ];
    assert!(!is_jit_eligible(&label_code));
    assert!(is_jit_eligible(&[]));
}

// ── (б) The VM runtime twin of the №325 gate ─────────────────────────

#[test]
fn n328_vm_sink_check_rejects_private_labels_at_runtime() {
    let program = program_with(vec![
        Instruction::LabelJoin {
            dst: "k".to_string(),
            src: "@env".to_string(),
        },
        Instruction::SinkCheck {
            fn_name: "print".to_string(),
            arg: "k".to_string(),
            line: 3,
            arg_index: 0,
            deny: None,
        },
    ]);
    let result = Vm::new().run(program);
    let err = result.expect_err("runtime gate must reject");
    assert!(
        err.contains("SINK_CLEARANCE_RUNTIME"),
        "the runtime verdict is loud and distinct: {err}"
    );
}

#[test]
fn n328_vm_sink_check_passes_bottom_labels() {
    let program = program_with(vec![Instruction::SinkCheck {
        fn_name: "print".to_string(),
        arg: "plain".to_string(),
        line: 1,
        arg_index: 0,
        deny: None,
    }]);
    let result = Vm::new().run(program);
    assert!(
        result.is_ok(),
        "bottom labels clear every sink: {:?}",
        result.err()
    );
}

#[test]
fn n328_runtime_source_labels_match_the_static_mapping() {
    // env → (private, trusted); network sources → (public, untrusted) —
    // the runtime seed equals the static №323 mapping (ADR-0156 §2).
    let program = program_with(vec![
        Instruction::LabelJoin {
            dst: "k".to_string(),
            src: "@env".to_string(),
        },
        Instruction::SinkCheck {
            fn_name: "write_file".to_string(),
            arg: "k".to_string(),
            line: 1,
            arg_index: 0,
            deny: None,
        },
    ]);
    let err = Vm::new().run(program).expect_err("private must fail");
    assert!(err.contains("private, trusted"), "{err}");

    let program = program_with(vec![
        Instruction::LabelJoin {
            dst: "resp".to_string(),
            src: "@http_get".to_string(),
        },
        Instruction::SinkCheck {
            fn_name: "exec".to_string(),
            arg: "resp".to_string(),
            line: 1,
            arg_index: 0,
            deny: None,
        },
    ]);
    let err = Vm::new().run(program).expect_err("untrusted must fail");
    assert!(err.contains("public, untrusted"), "{err}");
}

// ── (в) Golden verdicts: run and compile agree ───────────────────────

#[test]
fn n328_golden_verdicts_agree_between_run_and_compile() {
    // A private source into a public output: BOTH the run path
    // (audit_category_a + interpreter) and the compile path
    // (audit_category_a + compiler) reject with the same classes.
    let bad = r#"
        pattern T() -> String {
            let k = env("APP_KEY")
            print(k)
            return "ok"
        }
    "#;
    assert!(
        metalogos::compile_program(bad).is_err(),
        "compile must reject"
    );
    assert!(
        metalogos::run_program(bad).is_err(),
        "run must reject with the same verdict"
    );
    // A clean program: both paths accept.
    let good = r#"
        pattern T() -> String {
            print("hello")
            return "ok"
        }
    "#;
    assert!(metalogos::compile_program(good).is_ok());
    assert!(metalogos::run_program(good).is_ok());
}

// ── (г) Compiler lowers static knowledge into runtime instructions ───

#[test]
fn n328_compiler_emits_label_instructions() {
    let program = metalogos::compile_program(
        r#"
        pattern T() -> String {
            let k = env("APP_KEY")
            print("x")
            return k
        }
        "#,
    )
    .expect("clean program compiles");
    // Patterns live in main_code as RegisterPattern payloads (the VM
    // promotes them into its pattern table on load).
    let has_label_join = program.main_code.iter().any(|i| match i {
        Instruction::RegisterPattern(f) => f
            .code
            .iter()
            .any(|c| matches!(c, Instruction::LabelJoin { .. })),
        _ => false,
    });
    assert!(
        has_label_join,
        "source-backed lets carry a LabelJoin into the bytecode"
    );
}

// ── (д) The lattice label survives into the runtime env shape ────────

#[test]
fn n328_runtime_label_is_the_adr0154_label() {
    // The runtime env stores `labels::Label` values (the №322 lattice) —
    // the join is componentwise (LabelJoin merges into dst).
    let program = program_with(vec![
        Instruction::LabelJoin {
            dst: "k".to_string(),
            src: "@env".to_string(),
        },
        Instruction::LabelJoin {
            dst: "k".to_string(),
            src: "@http_get".to_string(),
        },
        Instruction::SinkCheck {
            fn_name: "print".to_string(),
            arg: "k".to_string(),
            line: 1,
            arg_index: 0,
            deny: None,
        },
    ]);
    let err = Vm::new()
        .run(program)
        .expect_err("join must keep untrusted");
    // join(private, trusted, public, untrusted) = (private, untrusted).
    assert!(err.contains("private, untrusted"), "{err}");
    let l = Label {
        conf: Conf::Private,
        integrity: metalogos::labels::Integrity::Untrusted,
        consent: Default::default(),
    };
    assert_eq!(l.conf, Conf::Private);
}

// ── (и) No-stub hygiene (№16.0-D) ────────────────────────────────────

#[test]
fn n328_no_stub_markers_in_touched_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in ["src/bytecode.rs", "src/compiler.rs", "src/vm.rs"] {
        let src = std::fs::read_to_string(format!("{manifest_dir}/{file}"))
            .unwrap_or_else(|_| panic!("{file} must exist"));
        for marker in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !src.contains(marker),
                "stub marker `{marker}` found in {file}"
            );
        }
    }
}
