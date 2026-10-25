// ── №730 (gh#1359, Wave 51): the 0.32 fixed-list movement №6 — the
// ПДн-residue package under the BLOCKING gate ───────────────────────────
//
// The movement №6 walks the ADR-0193 frozen list
// (scripts/ci/gate_032_untyped_sources.txt, 67 ids; closed 23/67 at
// the start — the BLOCKING floor) and returns the HONEST result: the
// first package (the ПДн residue — 13 rows: query, query_row,
// query_scalar, request_body, secret, generate_key, memory_open,
// memory_read, stdin, session_poll_wake, session_take_interrupt,
// recipe_list, recipe_search) closes ZERO rows. Every one of the 44
// open fixed-list rows carries a STANDING refusal posture; the handler
// code has not moved since the №717 walk (the diff
// 58cecc8..2610039 over the handler surface is EMPTY — verified
// 2026-10-20), so every №712/№717 anchor re-verifies and 10 drifted
// anchors were refreshed IN registry.rs (this PR, comment-only — the
// spellings are untouched, the counter stays 23/67, the floors stay:
// the only-up ratchets have nothing honest to move).
//
// The refusal wall of the movement (each row READ, not skipped — the
// №706 rule; the fresh anchors live in registry.rs):
//   query          the ADR-0194 gate (Proposed; the ratification +
//                  packages A(c)+B(i) landing — the wave-51 tail
//                  №734): the backend-DIVERGENT contract is ALIVE
//                  (TW mutations → the String count db_ops.rs:200-204,
//                  VM rows db_ops.rs:699+); NOT typed here either way —
//                  the contract is a union by SQL class after №734.
//   query_row /    the planned stubs — the element/value type is
//   query_scalar   data-dependent by column type (db_ops.rs:541-553,
//                  the №730 re-verified mapping rows).
//   request_body   the dynamic-shape Struct<JsonBody>
//                  (http.rs:469-476 — the registered fallback fields={}
//                  , the per-request keys parsed in the dispatch) —
//                  the №661 armor refuses the static labeling.
//   secret /       the opaque Value::Secret
//   generate_key   (src/builtins/crypto.rs:322-328 / 206-216 — the
//                  №730 path-refreshed anchors) — the Secret spelling
//                  is the dictionary lane (the owner decision).
//   memory_open /  the №539 anchors — the opaque Value::Memory handle
//   memory_read    (src/memory_typed.rs:300); the read is the
//                  String|Secret union by the container label
//                  (src/memory_typed.rs:550-556 — Stored::Plain →
//                  String | Stored::Enc → Value::Secret; the №730
//                  fresh anchors).
//   stdin          the true stub — no handler on any lane.
//   session_poll_wake /  the state-dependent Struct|Unit unions
//   session_take_interrupt (session.rs:276-291 / 326-345 — the №730
//                  re-verified anchors; the №543 form) — BOTH arms are
//                  LIVE and probed below (the empty-queue Unit arm and
//                  the pending-queue Struct arm).
//   recipe_list /  the always-empty List handlers
//   recipe_search  (src/builtins/office/recipes.rs:80-87 / 63-71 —
//                  the №730 path-refreshed anchors) — a List<X>
//                  spelling would be fictitious (the №669 form); the
//                  emptiness is probed below.
//
// The remaining 31 open fixed-list rows re-verify standing from the
// №717 walk (the opaque handles, the heterogeneous unions, the
// data-dependent shapes, the planned stubs, the loud-Err/unreachable
// lanes) — the full 44-row Unknown ground is re-pinned below as the
// interlock with naryad_717_type_closures.rs.
//
// The wave-51 honest outcome: the movement №6 counter criterion
// (23 → N, N > 23) is UNACHIEVABLE within the naryad's own Запреты —
// every growth path runs through an owner/semantics lane (the Secret
// spelling — the dictionary lane; the №543 union forms — the
// semantics lane; the stub handlers — separate naryads; the query*
// eligibility — post-№734). THIS IS THE REPORTED FACT, not a failure
// of the walk: the residue wall is the program's designed endgame
// frontier (ADR-0193 endgame (A)).
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use metalogos::builtins::{sig_types::Type, BUILTIN_REGISTRY};

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

// ── The registry-side ground: the 38 open fixed-list rows stay Unknown ──

#[test]
fn n730_open_fixed_list_ground_stays_unknown_in_registry() {
    // The movement №6 closes ZERO rows — the open ground (38 =
    // 36 default-build rows + the two feature-gated video/voice rows
    // asserted under their gates below) is re-pinned as the interlock
    // with naryad_717_type_closures.rs: any future PR that types one
    // of these rows WITHOUT the honest handler fact breaks THIS pin
    // first (the №688 rule: an id closes once, by the №644 predicate,
    // in its own naryad). №740 (gh#1378, wave 53) closed the THREE
    // query* rows through the №740 UNION vocabulary (lane α) — the
    // rows left THIS pin with the live handler fact (the spellings
    // pinned in tests/naryad_740_union_spellings.rs). №744 (gh#1385,
    // wave 54) closed graph_path, vision_list, vision_lora_load —
    // the spellings pinned in tests/naryad_744_lane_alpha_spellings.rs.
    for name in [
        // the ПДн residue (10 after №740) — the movement №6 package,
        // the query* three closed by №740 with the live-handler fact
        "generate_key",
        "memory_open",
        "memory_read",
        "recipe_list",
        "recipe_search",
        "request_body",
        "secret",
        "session_poll_wake",
        "session_take_interrupt",
        "stdin",
        // the re-verified №717 residue — graph_path, vision_list,
        // vision_lora_load left the ground: №744 (gh#1385, wave 54)
        // closed them (the №744 battery); reflex_list + the deny pair
        // + the events trio left: №748 (gh#1392, wave 55 — the live
        // intercepts); the residue (20)
        "backend_select",
        "call_llm_schema",
        "config_load",
        // deny_event/deny_reason/event_count/event_sum/events_since left
        // the refusal ground: №748 (gh#1392, wave 55) closed them — the
        // ADR-0051/0052 intercepts are the live both-lane handlers (the
        // №736 intercept-walk lesson) — tests/naryad_748_wall_walk.rs.
        "device_open",
        "duplex_open",
        "find",
        "forecast_next",
        "goal_make",
        "inspect",
        "media_source_capture",
        "pose_make",
        "reflex_bpe_load",
        // reflex_generate left the DEFAULT-build ground: №799 (gh#1546,
        // wave 65) gated the row with `candle` — asserted under the
        // same gate below (the parked default refusal is pinned by
        // naryad_799).
        // reflex_list left the refusal ground: №748 (gh#1392, wave 55)
        // closed it List<String> through the live shared dispatch —
        // tests/naryad_748_wall_walk.rs.
        "reflex_load",
        // resolve_skill_index — №751 (wave 56) re-classed the №717
        // "true stub" anchor: the row is LIVE on both lanes but the
        // lanes DIVERGE (the TW type_name SkillIndex_{dept} vs the
        // fixed "SkillIndex"; the truncation field TW-only) — the
        // divergence class (the ADR-0075 family); the Unknown pin
        // stays — tests/naryad_751_endgame_walk.rs.
        "resolve_skill_index",
        "series_make",
        "series_pull",
        "subgraph_extract",
        "trajectory_make",
        "vision_load",
    ] {
        let s = spec_of(name);
        assert_eq!(
            s.return_type,
            Type::Unknown,
            "the refusal {name} must stay Unknown honest (the movement №6 closed nothing)"
        );
    }
    // The feature-gated rows (the video/voice model lanes — NOT default
    // features): the registry TEXT carries the refusal (the №644
    // machine reads the text rows), the runtime row exists only under
    // its feature — asserted under the same gate (the №717 form).
    #[cfg(feature = "video")]
    assert_eq!(
        spec_of("video_fetch_weights").return_type,
        Type::Unknown,
        "the refusal video_fetch_weights must stay Unknown honest"
    );
    #[cfg(feature = "voice")]
    assert_eq!(
        spec_of("voice_load").return_type,
        Type::Unknown,
        "the refusal voice_load must stay Unknown honest"
    );
    // №799 (T2, gh#1488): reflex_generate left the default-build ground —
    // the row gates with `candle`; the parked default refusal is pinned
    // by naryad_799.
    #[cfg(feature = "candle")]
    assert_eq!(
        spec_of("reflex_generate").return_type,
        Type::Unknown,
        "the refusal reflex_generate must stay Unknown honest (№799: the row gates with candle)"
    );
}

// ── The live both-backend probes of the ПДн-residue refusal shapes ──────

#[test]
fn n730_recipe_list_always_empty_list_both_backends() {
    // The refusal shape probe: the handler answers the always-empty
    // List (src/builtins/office/recipes.rs:80-87) — the List fact is
    // pinned, the ELEMENT type stays unspelled (no live element
    // instance exists on any lane — a List<X> spelling would be
    // fictitious, the №669 graph_path form).
    let src = r#"
pattern M(_t: String) -> String {
  let l = recipe_list()
  return type_of(l) + "/" + json_encode(l)
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "List/[]");
}

#[test]
fn n730_session_poll_wake_union_both_arms_live_both_backends() {
    // The №543 union probe: the empty-queue arm returns Unit, the
    // pending-queue arm returns Struct "Wake"{source,payload} — BOTH
    // arms are live (state-dependent), which is exactly why no single
    // flat spelling exists. The wake source rides the closed
    // vocabulary (session.rs:46 — keyword|event|schedule).
    let src = r#"
pattern M(_t: String) -> String {
  let sess = session_login("n730-user-wake", "pw-not-checked-by-design")
  let none = session_poll_wake(sess)
  let _ = session_wake(sess, "keyword", "n730-payload")
  let some = session_poll_wake(sess)
  return type_of(none) + "/" + type_of(some) + "/"
    + json_get(some, "source")
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "Unit/Struct/keyword");
}

#[test]
fn n730_session_take_interrupt_union_both_arms_live_both_backends() {
    // The same №543 union probe for the interrupt lane: the
    // empty-queue arm Unit, the queued arm Struct
    // "Interrupt"{priority,reason} — both arms live (session.rs:326-345
    // — the №730 re-verified anchor).
    let src = r#"
pattern M(_t: String) -> String {
  let sess = session_login("n730-user-int", "pw-not-checked-by-design")
  let none = session_take_interrupt(sess)
  let _ = session_interrupt(sess, "high", "n730-reason")
  let some = session_take_interrupt(sess)
  return type_of(none) + "/" + type_of(some) + "/"
    + json_get(some, "priority")
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "Unit/Struct/high");
}

#[test]
fn n730_memory_read_public_arm_string_both_backends() {
    // The №539 union probe, the String arm: a PUBLIC container read
    // answers String (Stored::Plain — src/memory_typed.rs:550-556).
    // The private arm answers the gated Secret by the SAME signature —
    // which is exactly why typing String would false-warn the private
    // lane (the refusal stands).
    let src = r#"
pattern M(_t: String) -> String {
  let h = memory_open("n730-subject", "public")
  let _ = memory_put(h, "n730-key", "n730-value")
  let v = memory_read(h, "n730-key")
  return type_of(v)
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "String");
}

#[test]
fn n730_stdin_is_the_true_stub_loud_both_backends() {
    // The stdin refusal probe: the true stub (registry.rs:180 — no
    // handler on any lane) refuses loudly — typing its shape would
    // invent the no-stdin fallback contract (the №712 refusal stands;
    // the loud refusal is the typed contract, the Ok shape does not
    // exist).
    let src = r#"
pattern M(_t: String) -> String {
  let v = stdin()
  return type_of(v)
}
flow Main { input: String = "t" -> M -> output }
"#;
    let tw = run_tw(src).expect_err("TW must refuse loudly (the true stub)");
    assert!(tw.contains("stdin"), "TW refusal must name stdin: {tw}");
    let vm_err = run_vm(src).expect_err("VM must refuse loudly (the true stub)");
    assert!(
        vm_err.contains("stdin"),
        "VM refusal must name stdin: {vm_err}"
    );
}
