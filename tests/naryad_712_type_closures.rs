// ── №712 (gh#1319, Wave 49): the 0.32 fixed-list movement №4 ──────────
//
// The next package over the ADR-0193 frozen list
// (scripts/ci/gate_032_untyped_sources.txt, 67 ids): THREE honest
// closures by the №644 criterion — every signature landed from the
// LIVE handler and verified HERE by a both-backend test:
//
//   duplex_state     → Struct<DuplexState>{id:untrusted,session:internal,
//                      speak:internal,listen:internal,speak_last:internal,
//                      listen_last:internal} (duplex.rs state — struct_of
//                      "DuplexState", the SINGLE Ok arm; id echoes the
//                      caller's channel handle → untrusted (the №706
//                      echo form); session + the four stream projections
//                      read off the in-tree duplex registry → internal
//                      (the goal_get form); the OPTIONAL
//                      Struct<DuplexStream>|Unit slots — the
//                      memory_revise precedent; the inner DuplexStream
//                      carries stream_id/direction/priority/outcome
//                      [+interrupted_by/interrupted_priority] metadata,
//                      no content payload). THE FROZEN-LIST CLOSURE —
//                      goal032 19 → 20/67: the ≥20 figure of ADR-0193
//                      reached as a machine FACT (the parameter stays
//                      the OWNER's verdict — report-only, no
//                      self-capture).
//   read_file_tokens → Struct<FileInfo>{content:untrusted,chars:untrusted,
//                      tokens:untrusted} (text.rs — the SINGLE Ok arm;
//                      the bare №543 "Struct" upgraded to the full
//                      meta; content is the sandboxed file body →
//                      untrusted; chars/tokens are the numeric
//                      derivations → untrusted (the №623 label-flow:
//                      derivation conducts; the length-oracle
//                      conservatism — the gh#1317 class)).
//   user_profile     → Struct<UserProfile>{container:untrusted,
//                      count:internal,static:private,dynamic:private,
//                      buckets:private} (profile.rs — struct_value
//                      "UserProfile", the SINGLE Ok arm; the
//                      fresh-build and cache-hit paths construct the
//                      SAME value; container echoes the caller's name →
//                      untrusted; count is the derived record count →
//                      internal (the cron_list run_count form);
//                      static/dynamic/buckets carry the user-authored
//                      KV content → private (the №689 mail form)).
//                      NOT on the frozen 67 list (typed-coarse at the
//                      anchor f92c61c) — the bp move, not a goal032
//                      closure.
//
// The RE-VERIFIED refusals of the movement №4 (each was READ, not
// skipped — the fresh anchors live in registry.rs; the №702/№706
// refusal ground holds):
//   query          the backend-DIVERGENT contract is ALIVE and now
//                  PINNED by №711 → ADR-0194 (Proposed): TW mutations →
//                  the String count (db_ops.rs:200-204), VM mutations →
//                  the empty List (db_ops.rs:699+); the unification is
//                  the OWNER's language gate.
//   query_row /    the planned stubs — the element/value type is
//   query_scalar   data-dependent by column type (db_ops.rs:535-557).
//   request_body   the dynamic-shape Struct<JsonBody> (http.rs:469-476)
//                  — the №661 field-meta armor refuses the static
//                  labeling.
//   secret /       the opaque Value::Secret (crypto.rs:322-328 /
//   generate_key   crypto.rs:206-216) — no Secret spelling in the flat
//                  vocabulary (the №538 posture); typing String would
//                  breach the SECRET_LEAK Category-A contract.
//   memory_open /  the №539 anchors hold (the opaque Memory handle
//   memory_read    behind the consent gate; String|Secret by the
//                  container label).
//   stdin          the true stub — no handler on any lane.
//   session_poll_wake /  the state-dependent Struct|Unit unions
//   session_take_interrupt (session.rs:149-156 / 199-206 — the №543
//                  send_message form).
//   recipe_list /  the always-empty List handlers (recipes.rs:75/86) —
//   recipe_search  a List<X> spelling would be fictitious (the №669
//                  graph_path form).
//
// The ratchet: the sensitive floor moved UP in the SAME PR
// (6344 → 6551 bp; scripts/ci/sensitive_surface_baseline.txt —
// 95*10000//145); the frozen list itself is NOT edited (the id closes
// once). goal032 stays report-only (ADR-0193 Proposed — the ≥20 figure
// is the OWNER's verdict, not self-captured).
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

// ── The registry-side facts: the three closures carry the full meta ────

#[test]
fn n712_duplex_state_carries_struct_meta() {
    let s = spec_of("duplex_state");
    assert!(
        s.parameterized,
        "duplex_state must be parameterized (Struct<DuplexState>)"
    );
    for label in [
        "id:untrusted",
        "session:internal",
        "speak:internal",
        "listen:internal",
        "speak_last:internal",
        "listen_last:internal",
    ] {
        assert!(
            s.field_meta.contains(label),
            "duplex_state field_meta must carry {label}: {}",
            s.field_meta
        );
    }
}

#[test]
fn n712_read_file_tokens_carries_fileinfo_meta() {
    let s = spec_of("read_file_tokens");
    assert!(
        s.parameterized,
        "read_file_tokens must be parameterized (Struct<FileInfo>)"
    );
    for label in ["content:untrusted", "chars:untrusted", "tokens:untrusted"] {
        assert!(
            s.field_meta.contains(label),
            "read_file_tokens field_meta must carry {label}: {}",
            s.field_meta
        );
    }
}

#[test]
fn n712_user_profile_carries_userprofile_meta() {
    let s = spec_of("user_profile");
    assert!(
        s.parameterized,
        "user_profile must be parameterized (Struct<UserProfile>)"
    );
    for label in [
        "container:untrusted",
        "count:internal",
        "static:private",
        "dynamic:private",
        "buckets:private",
    ] {
        assert!(
            s.field_meta.contains(label),
            "user_profile field_meta must carry {label}: {}",
            s.field_meta
        );
    }
}

// ── The live both-backend probes ────────────────────────────────────────

#[test]
fn n712_duplex_state_live_shape_both_backends() {
    // The fresh channel: the state projection is the Struct with the
    // echo id (String), the internal session (String), and the four
    // OPTIONAL stream slots at their Unit (no-stream) shape.
    let src = r#"
pattern M(_t: String) -> String {
  let sess = session_login("n712-user", "pw-not-checked-by-design")
  let ch = duplex_open(sess)
  let st = duplex_state(ch)
  return type_of(st) + "/" + type_of(json_get(st, "id")) + "/"
    + type_of(json_get(st, "session")) + "/"
    + type_of(json_get(st, "speak")) + "/"
    + type_of(json_get(st, "speak_last"))
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "Struct/String/String/Unit/Unit");
}

#[test]
fn n712_read_file_tokens_live_shape_both_backends() {
    // Cargo.toml is inside the sandbox root and outside the №455
    // sensitive deny-list — the FileInfo struct comes back with the
    // untrusted String content and the derived Float counters.
    let src = r#"
pattern M(_t: String) -> String {
  let r = read_file_tokens("Cargo.toml")
  return type_of(r) + "/" + type_of(json_get(r, "content")) + "/"
    + type_of(json_get(r, "chars")) + "/" + type_of(json_get(r, "tokens"))
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "Struct/String/Float/Float");
}

#[test]
fn n712_read_file_tokens_counts_are_consistent_both_backends() {
    // The FileInfo contract: tokens = ceil(chars / 4) — the derivation
    // pins the numeric relation, not just the types.
    let src = r#"
pattern M(_t: String) -> String {
  let r = read_file_tokens("Cargo.toml")
  let c = json_get(r, "chars")
  let t = json_get(r, "tokens")
  if t * 4.0 >= c {
    if t * 4.0 < c + 4.0 {
      return "consistent"
    }
  }
  return "drift"
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "consistent");
}

#[test]
fn n712_user_profile_live_shape_both_backends() {
    // The kv_store fixture lives in a sandboxed FILE db inside a TEMP
    // root (the №676 A3 rule: a fixture write never lands in the repo
    // tree — the naryad_387 EgressSandbox form): one static record; the
    // profile projects the SAME constructed value on both the
    // fresh-build and the cache-hit paths (the mtime generation
    // invalidates between the lanes — the profile.rs contract).
    let tmp = tempfile::TempDir::new().expect("temp sandbox root");
    let src = r#"
db { url: "sqlite:n712_profile_fixture.db" }
pattern M(_t: String) -> String {
  let _ = db_execute("CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT)", [])
  let _ = db_execute("INSERT OR REPLACE INTO kv_store (key, value) VALUES ('container:n712c:static:tone', 'warm')", [])
  let p = user_profile("n712_profile_fixture.db", "n712c")
  return type_of(p) + "/" + type_of(json_get(p, "container")) + "/"
    + type_of(json_get(p, "count")) + "/" + type_of(json_get(p, "static"))
}
flow Main { input: String = "t" -> M -> output }
"#;
    let root = tmp.path().to_path_buf();
    let tw = metalogos::run_program_with_dir(src.trim(), root.clone())
        .unwrap_or_else(|e| panic!("TW run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        "Struct/String/Float/List",
        "TW output mismatch\n{src}"
    );
    let declarations = metalogos::parser::parse(src.trim()).map_err(|e| panic!("parse error: {e}"));
    let mut comp = metalogos::compiler::Compiler::with_std_root(root);
    let program = comp
        .compile(declarations.unwrap())
        .unwrap_or_else(|e| panic!("VM compile failed: {e}"));
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm
        .run(program)
        .unwrap_or_else(|e| panic!("VM run failed: {e}"))
        .unwrap_or_default();
    assert_eq!(
        vm_out.trim_end(),
        "Struct/String/Float/List",
        "VM output mismatch\n{src}"
    );
}

// ── The refusal re-verification: the Unknown ground holds ──────────────

#[test]
fn n712_pdna_refusals_stay_unknown_in_registry() {
    // Each refusal was READ, not skipped (the №706 rule): the rows keep
    // the honest Unknown return type — no fictitious spellings landed.
    for name in [
        // query/query_row/query_scalar left the refusal ground: №740
        // (gh#1378, wave 53) closed them through the №740 UNION
        // vocabulary (lane α) with the live handler fact — the
        // spellings pinned in tests/naryad_740_union_spellings.rs.
        "request_body",
        "secret",
        "memory_open",
        "memory_read",
        "stdin",
        "session_poll_wake",
        "session_take_interrupt",
        "generate_key",
        "recipe_list",
        "recipe_search",
    ] {
        let s = spec_of(name);
        assert_eq!(
            s.return_type,
            Type::Unknown,
            "the refusal {name} must stay Unknown honest"
        );
    }
}

#[test]
fn n712_goal032_machine_reads_the_duplex_state_closure() {
    // The goal032 machine counts the closure by the id leaving the
    // uncovered set — the machine itself runs in CI (the blocking
    // docs-metrics + the report step); here the registry-side ground:
    // the duplex_state row is typed + parameterized + labeled, which is
    // exactly the №644 closure predicate.
    let s = spec_of("duplex_state");
    assert!(s.parameterized && !s.field_meta.is_empty());
}
