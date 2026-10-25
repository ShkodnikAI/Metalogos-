// ── №744 (gh#1385, Wave 54): the 0.32 fixed-list movement №9 — the
// lane α continuation through the №740 UNION vocabulary ──────────────
//
// The В54 dispatch (gh#1388) ordered the next ПДн-class group first
// (the ADR-0193 priority). The prima-facie eight rows refuse as a WALL
// (the refusal table in the naryad report — each row READ, not
// skipped): session_poll_wake/session_take_interrupt — the №543-form
// state-dependent unions (the (β) lane — the owner gate, NOT taken),
// memory_read/memory_open/secret — the Secret/opaque-spelling ground
// parked by ADR-0195 §7 (c) (the Opaque arm is not covered-grade by
// the №740 union rule), stdin — the true stub, request_body — the
// dynamic Struct fields (the №661 armor), device_open — the opaque
// Device handle. The honest-zero branch fired for the ПДн group and
// the NEXT group was chosen by the machine (`--goal032 --list` + the
// covered-grade rule). The closable set:
//
//   graph_path       → List<Struct<PathError>|Struct<PathNode>>  (the
//                      union INNER — the №740 query_row form; the
//                      result is ALWAYS a List, the elements are the
//                      PathNode{id,text,level} structs (memory.rs:800)
//                      or the single PathError{status,from,to}
//                      (memory.rs:812); the №669 "a fictitious
//                      List<PathNode>" concern dissolves — BOTH
//                      element shapes named, the №539 guard by
//                      construction);
//   vision_list      → List<String>   (the №240 intercept is the LIVE
//                      path on both lanes — execution.rs:599/2061,
//                      vm.rs:2955 — the `[Vision#N]` String handles,
//                      vision.rs:362; the row keeps its loud-Err
//                      fallback — the query №740 posture);
//   vision_lora_load → String         (the №244 intercept, both
//                      lanes — execution.rs:813/2275, vm.rs:3008; the
//                      SINGLE Ok arm returns the adapter name,
//                      vision.rs:1097 — the №757 decrypt form).
//
// The floors raised in the SAME PR (the №757/№613 form): the sensitive
// 6965 → 7172 bp (104/145), the general typed 6901 → 6958 bp (366/526),
// the precise 4302 → 4543 bp (239/526 — the FIRST-match triple restored
// to the live value, the №536 form), the parameterized 7540 → 7580 bp
// (94/124 — the denominator grows WITH the two new List rows), the
// goal032 BLOCKING figure 26 → 29 in ci.yml.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins::{sig_types::Type, BUILTIN_REGISTRY};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), repo_root())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(repo_root());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_both(src: &str, expected: &str) {
    let tw = run_tw(src)
        .unwrap_or_else(|e| panic!("TW run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(tw.trim_end(), expected, "TW output mismatch\n{src}");
    let vm = run_vm(src)
        .unwrap_or_else(|e| panic!("VM run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(vm.trim_end(), expected, "VM output mismatch\n{src}");
}

fn spec_of(name: &str) -> &'static metalogos::builtins::BuiltinSpec {
    BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the registry row {name} must exist"))
}

// ── The registry-side pins: the spellings ARE the live contracts ─────

#[test]
fn n744_the_three_rows_carry_their_spellings() {
    // graph_path — the union INNER under the List head (the erasure
    // keeps the List; the parameterized spelling — the query_row form).
    assert_eq!(spec_of("graph_path").return_type, Type::List);
    assert!(spec_of("graph_path").parameterized);
    assert_eq!(spec_of("graph_path").field_meta, "");
    // The SSOT handler stays (both lanes dispatch through it — the
    // graph family has no intercept).
    assert!(spec_of("graph_path").handler.is_some());
    // vision_list — the parameterized List of the `[Vision#N]` handles.
    assert_eq!(spec_of("vision_list").return_type, Type::List);
    assert!(spec_of("vision_list").parameterized);
    assert_eq!(spec_of("vision_list").field_meta, "");
    // The row keeps its loud-Err fallback handler (the query posture:
    // the type is the INTERCEPT's shape, not the fallback's).
    assert!(spec_of("vision_list").handler.is_some());
    // vision_lora_load — the precise String (the single Ok arm).
    // №799 (T2, gh#1488): the row gates with the `vision` feature — the
    // spelling pin rides the feature build (the default build's parked
    // refusal is pinned by naryad_799_parked_parity).
    #[cfg(feature = "vision")]
    {
        assert_eq!(spec_of("vision_lora_load").return_type, Type::String);
        assert!(!spec_of("vision_lora_load").parameterized);
        assert!(spec_of("vision_lora_load").handler.is_some());
    }
}

#[test]
fn n744_the_inner_union_is_the_canonical_grammar() {
    // The graph_path inner names BOTH element shapes, strictly
    // ascending byte order ("Struct<PathError>" < "Struct<PathNode>").
    assert_eq!(
        Type::from_path("List<Struct<PathError>|Struct<PathNode>>"),
        Type::List
    );
    // The TOP-LEVEL twin proves the canonical order (the same arms at
    // the top level — a union; the reverse order fails closed).
    assert_eq!(
        Type::from_path("Struct<PathError>|Struct<PathNode>"),
        Type::Union
    );
    assert_eq!(
        Type::from_path("Struct<PathNode>|Struct<PathError>"),
        Type::Unknown
    );
    // The vision spellings are the plain vocabulary.
    assert_eq!(Type::from_path("List<String>"), Type::List);
    assert_eq!(Type::from_path("String"), Type::String);
}

// ── The live both-backend probes: the spellings match the runs ───────

#[test]
fn n744_graph_path_always_answers_a_list_on_both_backends() {
    // The found arm: the stored node pathed onto itself → the List
    // with ONE PathNode (the dijkstra self-path — memory_graph.rs:224
    // answers Some([from]) when from == to and the node exists); the
    // element id equals the stored node's id (the PathNode shape —
    // memory.rs:800). The no-path arm: ghost ids → the List with the
    // SINGLE PathError (memory.rs:812, status "no_path"). BOTH arms
    // are Lists — the always-List fact the spelling names.
    let src = r#"
pattern Probe(_x: String) -> String {
  let a = mtree_store("GraphPath n744 probe: the Distinctive Node Text for the SelfPath arm — the MemoryGraph stores this Node above the admission threshold", 0.9)
  let found = graph_path(a.id, a.id)
  let fe = found[0]
  let ghost = graph_path("n744-ghost-a", "n744-ghost-b")
  let ge = ghost[0]
  return type_of(found) + "|" + to_string(len(found)) + "|"
    + to_string(fe.id == a.id) + "|" + type_of(ghost) + "|"
    + to_string(len(ghost)) + "|" + ge.status
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both(src, "List|1|true|List|1|no_path");
}

#[test]
fn n744_vision_list_answers_the_handle_list_on_both_backends() {
    // The №240 intercept is the LIVE path on both lanes: the fresh
    // program's vision registry is empty → the honest empty List of
    // the `[Vision#N]` String handles (vision.rs:352-366). The type
    // pins the shape; the element is the plain String the dispatch
    // constructs (no fictitious inner).
    let src = r#"
pattern Probe(_x: String) -> String {
  let l = vision_list()
  return type_of(l) + "|" + to_string(len(l))
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both(src, "List|0");
}

#[test]
// №799 (T2, gh#1488): the №244 intercept is the vision build's live
// path — the probe rides the feature build (the default build's parked
// refusal is pinned by naryad_799).
#[cfg(feature = "vision")]
fn n744_vision_lora_load_refusals_stay_loud_on_both_backends() {
    // The №244 intercept is the LIVE path on both lanes; the missing
    // db / missing adapter / missing weights-dir refusals stay LOUD
    // and name the builtin (vision.rs:996-1046). The SINGLE Ok arm
    // (the adapter name String — vision.rs:1097) is pinned
    // registry-side; the error channel is the live probe here.
    let src = r#"
pattern Probe(_x: String) -> String {
  let v = vision_lora_load("n744-no-such-adapter", "n744-no-such-path")
  return type_of(v)
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    for (lane, out) in [("TW", run_tw(src)), ("VM", run_vm(src))] {
        let err = out.err().unwrap_or_else(|| {
            panic!("{lane}: the refusal must be loud (the error channel)\n{src}")
        });
        assert!(
            err.contains("vision_lora_load"),
            "{lane}: the refusal must name the builtin: {err}\n{src}"
        );
    }
}
