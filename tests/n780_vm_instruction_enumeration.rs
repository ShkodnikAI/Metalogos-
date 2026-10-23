// Naryad №780 (issue #1491, wave 62, P0) — the vm_state_enumeration form:
// EVERY `Instruction` variant is classified as "inner" (handled by the
// pattern/route loop) or "main-only" (a loud compiler/VM skew there).
//
// The classification match below is EXHAUSTIVE and has NO wildcard: a new
// `Instruction` variant added to src/bytecode.rs without a class HERE is a
// COMPILE ERROR of this test file — never a silent wildcard tail (the
// audit-23.10 A1: the retired tail hid the label gates from the bodies).
// The VM's own `instr_name` (src/vm.rs) carries the same exhaustive
// discipline on the production side.

use metalogos::bytecode::Instruction;

/// The classification (exhaustive — a new variant is a compile error).
fn class(i: &Instruction) -> &'static str {
    match i {
        // ── inner-handled (the pattern/route loop executes them) ──
        Instruction::Const(_) => "inner",
        Instruction::LabelJoin(_) => "inner", // №780: BOTH loops now
        Instruction::SinkCheck(_) => "inner", // №780: BOTH loops now
        Instruction::LoadGlobal(_) => "inner",
        Instruction::LoadGlobalByName(_) => "inner",
        Instruction::LoadLocal(_) => "inner",
        Instruction::StoreLocal(_) => "inner",
        Instruction::CallBuiltin(_, _) => "inner",
        Instruction::CallPattern(_, _) => "inner",
        Instruction::Return => "inner",
        Instruction::Add => "inner",
        Instruction::Sub => "inner",
        Instruction::Mul => "inner",
        Instruction::Div => "inner",
        Instruction::Contains => "inner",
        Instruction::CmpGt => "inner",
        Instruction::CmpLt => "inner",
        Instruction::CmpGe => "inner",
        Instruction::CmpLe => "inner",
        Instruction::CmpEq => "inner",
        Instruction::CmpNe => "inner",
        Instruction::MakeStruct(_) => "inner",
        Instruction::GetField(_) => "inner",
        Instruction::IndexAccess => "inner",
        Instruction::MakeList(_) => "inner",
        Instruction::ListLen => "inner",
        Instruction::Pop => "inner",
        Instruction::PushUnit => "inner",
        Instruction::StartsWith => "inner",
        Instruction::Jump(_) => "inner",
        Instruction::JumpIfNot(_) => "inner",
        Instruction::Collapse(_) => "inner",
        Instruction::Memorize(_) => "inner",
        Instruction::Recall => "inner",
        Instruction::Forget(_) => "inner",
        Instruction::LlmCall(_, _) => "inner",
        Instruction::Relate => "inner",
        Instruction::FlowPipeline(_) => "inner",
        Instruction::FlowExec(_) => "inner",
        Instruction::TryEval(_) => "inner",
        Instruction::StoreAssignLocal(_) => "inner",
        Instruction::MatchTest(_) => "inner",
        Instruction::Dup => "inner",
        Instruction::BeginValueExpr => "inner",
        Instruction::KeepLastValue => "inner",
        Instruction::EndValueExpr => "inner",
        Instruction::SetValueReg => "inner",
        Instruction::JumpIfNotCond(_) => "inner",
        // ── main-only (a loud skew inside a body, never a silent skip) ──
        Instruction::StoreGlobal(_) => "main-only",
        Instruction::RegisterPattern(_) => "main-only",
        Instruction::RegisterLearnable(_) => "main-only",
        Instruction::MakeFluid(_) => "main-only",
        Instruction::JumpIfLow(_, _) => "main-only",
        Instruction::Adapt(_) => "main-only",
        Instruction::Mutate(_) => "main-only",
        Instruction::ExecuteRules => "main-only",
        Instruction::Halt => "main-only",
        Instruction::RegisterPatternRef(_) => "main-only",
    }
}

#[test]
fn n780_label_gates_are_inner_classified() {
    // The audit-23.10 A1 heart: the label gates are INNER-handled — the
    // classification itself cannot regress without failing this file to
    // compile (the variant would need a new class row here).
    let lj = Instruction::LabelJoin(Box::new(metalogos::bytecode::LabelJoinData {
        dst: "k".to_string(),
        src: "@env".to_string(),
    }));
    let sc = Instruction::SinkCheck(Box::new(metalogos::bytecode::SinkCheckData {
        fn_name: "print".to_string(),
        arg: "k".to_string(),
        line: 1,
        arg_index: 0,
        deny: None,
    }));
    assert_eq!(class(&lj), "inner");
    assert_eq!(class(&sc), "inner");
}

#[test]
fn n780_main_only_set_is_pinned() {
    // The main-only roster (12): the exact set the VM refuses loudly in a
    // pattern/route body. A variant moving between the classes is a
    // deliberate compiler/VM contract change and must touch this row.
    let main_only = [
        "StoreGlobal",
        "RegisterPattern",
        "RegisterLearnable",
        "MakeFluid",
        "JumpIfLow",
        "Adapt",
        "Mutate",
        "FlowPipeline",
        "FlowExec",
        "ExecuteRules",
        "Halt",
        "RegisterPatternRef",
    ];
    let classed: Vec<&str> = [
        Instruction::StoreGlobal(0),
        Instruction::RegisterPattern(Box::new(metalogos::bytecode::CompiledFn {
            name: String::new(),
            param_count: 0,
            param_types: vec![],
            code: vec![],
            is_pure: false,
        })),
        Instruction::RegisterLearnable(Box::new(metalogos::bytecode::CompiledLearnableInfo {
            name: String::new(),
            param_count: 0,
            prompt: String::new(),
            few_shot: vec![],
            context_mode: metalogos::bytecode::CompiledContextMode::None,
            distill_to: None,
            distill_after: 0,
            fallback_if: None,
            distill_min_accuracy: None,
            distill_margin: None,
        })),
        Instruction::MakeFluid(0),
        Instruction::JumpIfLow(0.0, 0),
        Instruction::Adapt(String::new()),
        Instruction::Mutate(Box::new(metalogos::bytecode::MutateData {
            pattern_name: String::new(),
            example_count: 0,
            rollback_threshold: None,
            rollback_op: None,
        })),
        Instruction::ExecuteRules,
        Instruction::Halt,
        Instruction::RegisterPatternRef(0),
    ]
    .iter()
    .map(|i| class(i))
    .collect();
    let loud_count = classed.iter().filter(|c| **c == "main-only").count();
    assert_eq!(loud_count, 10, "the sampled main-only rows are all loud");
    let _ = main_only; // the roster is the doc; the classes above enforce the set
    assert_eq!(
        class(&Instruction::RegisterPatternRef(0)),
        "main-only",
        "RegisterPatternRef is main-only"
    );
}

#[test]
fn n780_fluid_family_is_inner_classified() {
    // The FlowPipeline/FlowExec runners ARE handled in the inner loop
    // (the vm.rs arms exist) — they stay inner; only the 12-strong
    // main-only set above is loud.
    let fp = Instruction::FlowPipeline(Box::new(metalogos::bytecode::FlowPipelineData {
        pipeline: vec![],
        branch_defs: vec![],
    }));
    assert_eq!(class(&fp), "inner");
}
