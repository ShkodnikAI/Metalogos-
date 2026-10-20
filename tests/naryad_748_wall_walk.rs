// ── №748 (gh#1392, Wave 55): the 0.32 fixed-list movement №10 — the
// six-closure package + the 32-row wall walk ──────────────────────────
//
// The В55 dispatch (gh#1394) ordered the movement №10 through the
// №740 UNION vocabulary (the lane α continuation). The machine walk of
// ALL 38 open rows on live main (4b13319) — every row READ, not
// skipped, with the №736 intercept-walk lesson applied to the WHOLE
// wall (the registry comment is the REGISTRY arm, not the whole
// ground) — resolved SIX honest closures:
//
//   reflex_list   → List<String>   the shared reflex_list_dispatch
//                  precedes the registry stub on BOTH lanes (TW
//                  execution.rs:578/2038, VM vm.rs:2918-2923) and
//                  answers the List of the declared model names as
//                  Strings (reflex.rs:1220-1241, sorted by ReflexId.0
//                  — deterministic; the empty registry answers the
//                  honest empty List — the vision_list №744 form);
//   event_count   → Float          the ADR-0052 intercepts are the
//                  LIVE handlers on both lanes (TW execution.rs:2337-
//                  2339, VM vm.rs:2824-2827): the shared
//                  audit_ops::event_count answers the log count as
//                  Value::Float (audit_ops.rs:86-91 — the №757
//                  decrypt single-shape form); the №717 "planned
//                  wrapper, no handler" anchor was factually STALE;
//   event_sum     → Float          the same ADR-0052 ground (TW
//                  execution.rs:2351-2353, VM vm.rs:2829-2832): the
//                  exactly-2-args sum rides Value::Float
//                  (audit_ops.rs:112-124);
//   events_since  → List<Struct<Event>>
//                                 the same ground (TW execution.rs:
//                  2342-2344, VM vm.rs:2834-2837): every element is
//                  Struct{type_name: "Event"} (audit_ops.rs:142-179;
//                  the fields dynamic by construction — NO meta
//                  claimed — the query rows №740 form);
//   deny_event    → Struct<DenyEvent>{7 labeled fields}
//                                 the №392 on_deny intercepts are the
//                  LIVE handlers on both lanes (TW execution.rs:2318-
//                  2328, VM vm.rs:2395-2412): the FIXED 7-field
//                  DenyEvent (deny.rs:122-143 — make_event; the
//                  labels: argument/human echo the refused call →
//                  untrusted, the runtime's own words/numbers →
//                  internal — the listen_start №717 fixed-projection
//                  form); OUTSIDE an on_deny body the usage is a
//                  COMPILE error ([DENY_HANDLER_SCOPE]) — the error
//                  channel, not a shape;
//   deny_reason   → String        the same ground: the reason field
//                  make_event ALWAYS inserts (deny.rs:132) — the
//                  single reachable arm (audit_ops.rs:58-68).
//
// The OTHER 32 rows refused — the wall classes re-verified on live
// main (the registry comments carry the fresh anchors; the report in
// the naryad): the true stub (stdin), the never-Ok/loud stubs
// (reflex_bpe_load/voice_load/video_fetch_weights/series_pull), the
// coarse-opaque grounds (reflex_load Reflex, series_make
// SeriesHandle, media_source_capture Media, secret/generate_key
// Secret — parked ADR-0195 §7 (c)), the no-vocabulary-name opaques
// (memory_open/duplex_open/device_open/pose_make/trajectory_make/
// goal_make/subgraph_extract/forecast_next/vision_load), the
// data-dependent grounds (call_llm_schema/config_load/find), the
// candle-gated duality (reflex_generate — no house precedent for
// feature-conditional typing), the (β)-lane state unions
// (session_poll_wake/session_take_interrupt/inspect — the OWNER gate
// 7; backend_select — the №543 form), the parked Secret-arm
// (memory_read), the dynamic fields (request_body — №661) and the
// always-empty Lists (recipe_list/recipe_search — the №669 form).
//
// The floors raised in the SAME PR (the №757/№613 form): the sensitive
// 7172 → 7586 bp (110/145), the general typed 6958 → 7072 bp (372/526),
// the precise 4543 → 4600 bp (242/526), the parameterized 7580 → 7637
// bp (97/127 — the denominator grows WITH the rows), the goal032
// BLOCKING figure 29 → 35 in ci.yml. The №717/№730 interlocks
// resynced in the SAME PR (the №744 round-1 form).
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
fn n748_the_six_rows_carry_their_spellings() {
    // reflex_list — the parameterized List of the model-name Strings;
    // the row KEEPS its loud-Err stub handler (the query/vision №740
    // posture: the type is the INTERCEPT's shape, not the fallback's).
    assert_eq!(spec_of("reflex_list").return_type, Type::List);
    assert!(spec_of("reflex_list").parameterized);
    assert_eq!(spec_of("reflex_list").field_meta, "");
    assert!(spec_of("reflex_list").handler.is_some());
    // event_count / event_sum — the precise Floats (the single-shape
    // facts); no handler token (the intercept marker variant).
    assert_eq!(spec_of("event_count").return_type, Type::Float);
    assert!(!spec_of("event_count").parameterized);
    assert_eq!(spec_of("event_sum").return_type, Type::Float);
    assert!(!spec_of("event_sum").parameterized);
    // events_since — the parameterized List of the Event structs (the
    // query rows form: the type_name carries the shape, no meta).
    assert_eq!(spec_of("events_since").return_type, Type::List);
    assert!(spec_of("events_since").parameterized);
    assert_eq!(spec_of("events_since").field_meta, "");
    // deny_event — the labeled DenyEvent Struct (the listen_start
    // fixed-projection form; the labeled head is parameterized).
    assert_eq!(spec_of("deny_event").return_type, Type::Struct);
    assert!(spec_of("deny_event").parameterized);
    assert_eq!(
        spec_of("deny_event").field_meta,
        "reason:internal,sink:internal,class:internal,argument:untrusted,label:internal,line:internal,human:untrusted"
    );
    // deny_reason — the precise String (the single reachable arm).
    assert_eq!(spec_of("deny_reason").return_type, Type::String);
    assert!(!spec_of("deny_reason").parameterized);
    // The plain vocabulary — no union in the six (the inner unions of
    // №740 were not needed here).
    assert_eq!(Type::from_path("List<String>"), Type::List);
    assert_eq!(Type::from_path("List<Struct<Event>>"), Type::List);
}

#[test]
fn n748_the_wall_stays_unknown_in_registry() {
    // The 32 refused rows keep the honest Unknown ground — each READ
    // on live main 4b13319, the classes in the file header. The SIX
    // closures left the ground (above); the residue stays.
    for name in [
        // the true stub (1)
        "stdin",
        // never-Ok / loud stubs (4)
        "reflex_bpe_load",
        "series_pull",
        // opaque without a vocabulary name (9)
        "memory_open",
        "duplex_open",
        "device_open",
        "pose_make",
        "trajectory_make",
        "goal_make",
        "subgraph_extract",
        "forecast_next",
        "vision_load",
        // opaque with a name — coarse, not covered (4)
        "series_make",
        "media_source_capture",
        "secret",
        "generate_key",
        // data-dependent — the bare arms not covered (3)
        "call_llm_schema",
        "config_load",
        "find",
        // the candle-gated duality (1)
        "reflex_generate",
        // the coarse-opaque dispatch arm (1)
        "reflex_load",
        // owner-gated lanes — the (β) gate 7 (4)
        "session_poll_wake",
        "session_take_interrupt",
        "backend_select",
        "inspect",
        // parked / dynamic (2)
        "memory_read",
        "request_body",
        // always-empty List (2)
        "recipe_list",
        "recipe_search",
    ] {
        let s = spec_of(name);
        assert_eq!(
            s.return_type,
            Type::Unknown,
            "the wall row {name} must stay Unknown honest"
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
}

// ── The live both-backend probes: the spellings match the runs ───────

#[test]
fn n748_reflex_list_answers_the_name_list_on_both_backends() {
    // The shared reflex_list_dispatch is the LIVE path on both lanes:
    // the fresh program's reflex registry is empty → the honest empty
    // List (reflex.rs:1220-1241 — the vision_list №744 form).
    let src = r#"
pattern Probe(_x: String) -> String {
  let l = reflex_list()
  return type_of(l) + "|" + to_string(len(l))
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both(src, "List|0");
}

#[test]
fn n748_the_events_trio_answers_on_both_backends() {
    // The ADR-0052 intercepts are the LIVE handlers on both lanes: the
    // fresh program's event log is empty → count 0.0 (Float), the sum
    // 0.0 (Float), the since-window List (empty on a fresh log). The
    // element shape (Struct "Event") is pinned registry-side; the
    // wrong-arity refusals stay loud (the error channel).
    let src = r#"
pattern Probe(_x: String) -> String {
  let c = event_count()
  let s = event_sum("custom", "ms")
  let l = events_since(1.0)
  return type_of(c) + "|" + type_of(s) + "|" + type_of(l)
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    // The SHAPES are the closure evidence: Float|Float|List. The COUNT
    // values are data-dependent BY DESIGN (the shared ledger grows as
    // the flow emits its audit events — and the exact per-lane emission
    // order is the audit machinery's business, not the type contract),
    // so the len pin would be a flaky fixture — the shapes pin only.
    assert_both(src, "Float|Float|List");
}

#[test]
fn n748_the_deny_pair_inside_the_handler_both_backends() {
    // The №392 on_deny intercepts are the LIVE handlers on both lanes:
    // inside the handler body deny_event() reads the live DenyEvent
    // Struct and deny_reason() the reason String (audit_ops.rs:58-68).
    // The probe rides the kv bridge (the handler env is local — the
    // kv keys carry the reads out; the one-time grant's second call
    // fires IRREVERSIBLE_NO_GRANT — the №392 HANDLED_SCENARIO form).
    let src = r#"
on_deny(db) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then {
      let _ = kv_set("n748-ev", type_of(deny_event()))
      let _ = kv_set("n748-rs", deny_reason() + "|" + type_of(deny_reason()))
    }
    "VOICE_EGRESS_UNCONSENTED" then { print("d") }
    "UNTRUSTED_EXEC_DECISION" then { print("d") }
    "SECRET_TO_EXEC" then { print("d") }
    "SECRET_EGRESS_VCS" then { print("d") }
    "SECRET_EGRESS_NETWORK" then { print("d") }
    "PII_EGRESS_NETWORK" then { print("d") }
    "PII_EGRESS_OUTPUT" then { print("d") }
    "UNTRUSTED_EGRESS_NETWORK" then { print("d") }
    "UNTRUSTED_FILE_PATH" then { print("d") }
    "SINK_CLEARANCE" then { print("d") }
    "HTML_INJECTION" then { print("d") }
    "TAINT_PERSISTENCE" then { print("d") }
    "SECRET_LEAK" then { print("d") }
  }
}
db { url: "sqlite::memory:" }
pattern D(_t: String) -> String {
  let _ = db_execute("CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, user TEXT)")
  let _ = db_execute("INSERT INTO notes (user) VALUES ('alice')")
  let g = grant_issue("db:delete:notes", 60, "n", 1)
  let _ = db_execute_with_grant(g, "DELETE FROM notes WHERE user = 'alice'")
  let _ = db_execute_with_grant(g, "DELETE FROM notes")
  return kv_get("n748-ev") + "|" + kv_get("n748-rs")
}
flow Main { input: String = "t" -> D -> output }
"#;
    assert_both(src, "Struct|IRREVERSIBLE_NO_GRANT|String");
}

#[test]
fn n748_the_deny_pair_outside_the_handler_stays_compile_loud() {
    // The outside-handler usage is a COMPILE error ([DENY_HANDLER_SCOPE]
    // — the event is runtime-constructed and cannot be forged): the
    // error channel, not a shape (the №392 (г) form).
    let src = r#"
pattern Leaky() -> String {
  let ev = deny_event()
  return "forged"
}
"#;
    let declarations = metalogos::parser::parse(src.trim()).expect("parse must succeed");
    let errors = metalogos::semantic::check_program(&declarations);
    assert!(
        errors
            .errors
            .iter()
            .any(|e| e.message.contains("DENY_HANDLER_SCOPE")),
        "deny_event outside a handler must be a compile error: {:?}",
        errors.errors
    );
}

#[test]
fn n748_the_true_stub_refuses_loud_on_both_backends() {
    // The A-class ground holds for stdin: NO handler on any lane (the
    // interpreter/VM name walk found no dispatch) — the call refuses
    // loudly and names the builtin (the №706 posture; the №736 t6
    // form).
    let src = r#"
pattern M(_t: String) -> String {
  let v = stdin()
  return type_of(v)
}
flow Main { input: String = "t" -> M -> output }
"#;
    let tw = run_tw(src).expect_err("TW must refuse loudly (the true stub)");
    assert!(tw.contains("stdin"), "TW refusal must name stdin: {tw}");
    let vm = run_vm(src).expect_err("VM must refuse loudly (the true stub)");
    assert!(vm.contains("stdin"), "VM refusal must name stdin: {vm}");
}

#[test]
fn n748_the_opaque_and_empty_list_grouns_live_on_both_backends() {
    // The D/H-class live grounds, pinned per shape:
    //   generate_key → the Secret opaque (crypto.rs:216 — the parked
    //     ADR-0195 §7 (c) class; type_of "Secret");
    //   secret("HOME") → the same Secret opaque (crypto.rs:325);
    //   recipe_list() → the always-empty List (recipes.rs:80-87 — the
    //     №669 ground: the List exists, its element X never does).
    let src = r#"
pattern M(_t: String) -> String {
  let k = generate_key()
  let s = secret("HOME")
  let r = recipe_list()
  return type_of(k) + "|" + type_of(s) + "|" + type_of(r) + "|"
    + to_string(len(r))
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "Secret|Secret|List|0");
}
