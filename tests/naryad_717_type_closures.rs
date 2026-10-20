// ── №717 (gh#1329, Wave 50): the 0.32 fixed-list movement №5 ──────────
//
// The next package over the ADR-0193 frozen list
// (scripts/ci/gate_032_untyped_sources.txt, 67 ids): THREE honest
// closures by the №644 criterion — every signature landed from the
// LIVE handler and verified HERE by a both-backend test:
//
//   listen_start        → Struct<DuplexFlow>{stream_id:internal,
//                         direction:internal,priority:untrusted,
//                         preempted:internal} (duplex.rs
//                         start_result_value, the SINGLE Ok arm —
//                         duplex.rs:122; stream_id is the FRESH
//                         registry-generated stream id → internal (the
//                         cron_list generated-count form); direction is
//                         the fixed speak|listen word → internal;
//                         priority echoes the caller's ladder word (or
//                         the session default) → untrusted (the №706
//                         echo form); preempted is the registry-
//                         generated List of preempted stream ids →
//                         internal). THE FROZEN-LIST CLOSURE — goal032
//                         20 → 23/67 (with llm_stream_open and
//                         vision_fetch_weights).
//   llm_stream_open     → Struct<LlmStream>{handle:internal,
//                         model:internal,provider:internal}
//                         (llm_stream.rs — the SINGLE Ok arm
//                         llm_stream.rs:59; handle carries the in-tree
//                         LlmStream id → internal; model/provider peek
//                         the CONFIGURED provider registry
//                         (llm_stream.rs:52-53, the "unknown" fallback)
//                         → internal; the stream CONTENT is not in the
//                         projection — next/close read the handle).
//   vision_fetch_weights → String (vision.rs:1524 — the SINGLE Ok arm,
//                         the caller's dest_dir echo; the
//                         non-vision arm is the loud refusal; the
//                         allowlist default-deny + SSRF guard +
//                         pickle-extension refusals stay loud in the
//                         error channel — the №757 decrypt form).
//
// The RE-VERIFIED refusals of the movement №5 (each was READ, not
// skipped — the fresh anchors live in registry.rs; the №712/№669
// refusal ground holds):
//   query          the backend-DIVERGENT contract is ALIVE and PINNED
//                  by №711 → ADR-0194 (Proposed): TW mutations → the
//                  String count (db_ops.rs:200-204), VM mutations →
//                  the empty List (db_ops.rs:699+).
//   query_row /    the planned stubs — the element/value type is
//   query_scalar   data-dependent by column type (db_ops.rs:535-557).
//   request_body   the dynamic-shape Struct<JsonBody> (http.rs:469-476)
//                  — the №661 field-meta armor refuses the static
//                  labeling.
//   secret /       the opaque Value::Secret (crypto.rs:322-328 /
//   generate_key   crypto.rs:206-216) — no Secret spelling in the flat
//                  vocabulary (the №538 posture).
//   memory_open /  the №539 anchors hold (memory_typed.rs:28-44/137+).
//   memory_read
//   stdin          the true stub — no handler on any lane.
//   session_poll_wake /  the state-dependent Struct|Unit unions
//   session_take_interrupt (session.rs:149-156 / 199-206 — the №543
//                  send_message form).
//   recipe_list    the always-empty List handlers (recipes.rs:86/75) —
//   recipe_search  a List<X> spelling would be fictitious (the №669
//                  graph_path form).
//   forecast_next /  the opaque SeriesHandle/ForecastHandle handles —
//   series_make      no stage-0 spellings (the №669 form; series_pull
//                    additionally NEVER returns Ok — the loud
//                    no-backend Err, forecast.rs:451-457).
//   graph_path     the heterogeneous List (PathNode | PathError) —
//                  memory.rs:786-805.
//   backend_select the heterogeneous Ok union — BackendSelected
//                  (backends.rs:225) | Degraded (backends.rs:262).
//   call_llm_schema /  the data-dependent shapes (llm_schema.rs:162,
//   config_load        287-303 / config.rs:130 → json.rs:321-341).
//   device_open / duplex_open / pose_make / trajectory_make /
//   goal_make / subgraph_extract — the opaque Device/Duplex/Pose/
//   Trajectory/GoalPredicate/Subgraph handles (no stage-0 spellings).
//   reflex_bpe_load / reflex_load / reflex_list / reflex_generate —
//   the loud-Err handlers (typing an unreachable Ok shape would be
//   fictitious).
//   media_source_capture / vision_list / vision_load /
//   vision_lora_load — the state-carrying interceptions (the
//   registry fallback is the loud Err — ADR-0164, №240/№242/№244).
//   video_fetch_weights (the formal No-Go, №294/ADR-0151 D7) /
//   voice_load (not implemented, phase A5/ADR-0143).
//   deny_event / deny_reason / event_count / events_since /
//   event_sum / find / inspect / resolve_skill_index — the planned
//   stubs, no handler on any lane.
//
// The ratchet: the sensitive floor moved UP in the SAME PR
// (6551 → 6758 bp; scripts/ci/sensitive_surface_baseline.txt —
// 98*10000//145), the fact twins moved in the SAME PR (gate_031
// sensitive 6551 → 6758; gate_030 precise 4505 → 4524 = 238/526;
// gate_030 parameterized 7478 → 7520 = 91/121). The frozen list itself
// is NOT edited (the id closes once). goal032 stays report-only
// (ADR-0193 Proposed — the parameters are the OWNER's verdict, not
// self-captured; the verdict package is the parallel №718).
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

fn assert_both_loud_err(src: &str, needle: &str) {
    let tw = run_tw(src).expect_err("TW must refuse loudly");
    assert!(tw.contains(needle), "TW refusal must carry {needle}: {tw}");
    let vm_err = run_vm(src).expect_err("VM must refuse loudly");
    assert!(
        vm_err.contains(needle),
        "VM refusal must carry {needle}: {vm_err}"
    );
}

fn spec_of(name: &str) -> &'static metalogos::builtins::BuiltinSpec {
    BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the registry row {name} must exist"))
}

// ── The registry-side facts: the three closures carry the full meta ────

#[test]
fn n717_listen_start_carries_duplexflow_meta() {
    let s = spec_of("listen_start");
    assert!(
        s.parameterized,
        "listen_start must be parameterized (Struct<DuplexFlow>)"
    );
    for label in [
        "stream_id:internal",
        "direction:internal",
        "priority:untrusted",
        "preempted:internal",
    ] {
        assert!(
            s.field_meta.contains(label),
            "listen_start field_meta must carry {label}: {}",
            s.field_meta
        );
    }
}

#[test]
fn n717_llm_stream_open_carries_llmstream_meta() {
    let s = spec_of("llm_stream_open");
    assert!(
        s.parameterized,
        "llm_stream_open must be parameterized (Struct<LlmStream>)"
    );
    for label in ["handle:internal", "model:internal", "provider:internal"] {
        assert!(
            s.field_meta.contains(label),
            "llm_stream_open field_meta must carry {label}: {}",
            s.field_meta
        );
    }
}

#[test]
fn n717_vision_fetch_weights_is_precise_string() {
    let s = spec_of("vision_fetch_weights");
    assert_eq!(
        s.return_type,
        Type::String,
        "vision_fetch_weights must carry the precise String spelling"
    );
    assert!(!s.parameterized, "the String spelling is not parameterized");
}

// ── The live both-backend probes ────────────────────────────────────────

#[test]
fn n717_listen_start_live_shape_both_backends() {
    // The fresh channel: the flow projection is the Struct with the
    // generated stream id (String), the fixed direction word (String),
    // the echoed ladder word (String), and the (empty) preempted List.
    // The listen flow sits behind the №428 consent gate (audio.listen,
    // fail-closed) — the probe grants the scope up front (the n352
    // harness form; no revoke in this file — parallel-safe).
    let src = r#"
pattern M(_t: String) -> String {
  let sess = session_login("n717-user", "pw-not-checked-by-design")
  let _ = consent_grant("n717-listen-probe", "audio.listen", "naryad-717")
  let ch = duplex_open(sess)
  let fl = listen_start(ch)
  return type_of(fl) + "/" + type_of(json_get(fl, "stream_id")) + "/"
    + type_of(json_get(fl, "direction")) + "/"
    + type_of(json_get(fl, "priority")) + "/"
    + type_of(json_get(fl, "preempted"))
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "Struct/String/String/String/List");
}

#[test]
fn n717_listen_start_direction_word_both_backends() {
    // The direction is the fixed LISTEN word — the flow projection
    // pins the semantic, not just the type. The №428 consent grant
    // arms the audio.listen scope for the probe (see the shape test).
    let src = r#"
pattern M(_t: String) -> String {
  let sess = session_login("n717-user2", "pw-not-checked-by-design")
  let _ = consent_grant("n717-listen-probe2", "audio.listen", "naryad-717")
  let ch = duplex_open(sess)
  let fl = listen_start(ch)
  return json_get(fl, "direction")
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "listen");
}

#[test]
fn n717_vision_fetch_weights_default_deny_loud_both_backends() {
    // The allowlist default-deny fires BEFORE any network activity
    // (ADR-0125): without MLOG_VISION_WEIGHTS_ALLOWLIST the handler
    // refuses loudly on both backends — the error channel stays the
    // security surface (the Ok arm types the SUCCESS shape only).
    let src = r#"
pattern M(_t: String) -> String {
  let r = vision_fetch_weights("https://example.test/weights", "/tmp/n717-weights")
  return type_of(r)
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both_loud_err(src, "MODEL_WEIGHTS_UNSAFE");
}

// ── The refusal re-verification: the Unknown ground holds ──────────────

#[test]
fn n717_movement_refusals_stay_unknown_in_registry() {
    // Each refusal was READ, not skipped (the №706 rule): the 42
    // default-build rows keep the honest Unknown return type (plus the
    // two feature-gated rows asserted under their gates below) — no
    // fictitious spellings landed. The list: 28 fresh №717 anchors +
    // 14 re-verified №712/№669 anchors.
    for name in [
        // fresh №717 (28 in the default build; +2 feature-gated below)
        "backend_select",
        "call_llm_schema",
        "config_load",
        "deny_event",
        "deny_reason",
        "device_open",
        "duplex_open",
        "event_count",
        "event_sum",
        "events_since",
        "find",
        "goal_make",
        "inspect",
        "media_source_capture",
        "pose_make",
        "recipe_search",
        "reflex_bpe_load",
        "reflex_generate",
        "reflex_list",
        "reflex_load",
        "resolve_skill_index",
        "series_make",
        "series_pull",
        "subgraph_extract",
        "trajectory_make",
        "vision_list",
        "vision_load",
        "vision_lora_load",
        // re-verified №712/№669 (14)
        "forecast_next",
        "generate_key",
        "graph_path",
        "memory_open",
        "memory_read",
        // query/query_row/query_scalar left the refusal ground: №740
        // (gh#1378, wave 53) closed them through the №740 UNION
        // vocabulary (lane α) — tests/naryad_740_union_spellings.rs.
        "recipe_list",
        "request_body",
        "secret",
        "session_poll_wake",
        "session_take_interrupt",
        "stdin",
    ] {
        let s = spec_of(name);
        assert_eq!(
            s.return_type,
            Type::Unknown,
            "the refusal {name} must stay Unknown honest"
        );
    }
    // The feature-gated rows (the video/voice model lanes — NOT default
    // features): the registry TEXT carries the refusal (the №644
    // machine reads the text rows), the runtime row exists only under
    // its feature — asserted under the same gate.
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

#[test]
fn n717_goal032_machine_reads_the_movement5_closures() {
    // The goal032 machine counts the closure by the id leaving the
    // uncovered set — the machine itself runs in CI (the report step);
    // here the registry-side ground: the two new Struct rows are typed
    // + parameterized + labeled, which is exactly the №644 closure
    // predicate (the third closure — vision_fetch_weights — is the
    // precise String, checked above).
    for name in ["listen_start", "llm_stream_open"] {
        let s = spec_of(name);
        assert!(
            s.parameterized && !s.field_meta.is_empty(),
            "the {name} row must carry the full №644 closure predicate"
        );
    }
}
