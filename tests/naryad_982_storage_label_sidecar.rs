// ── №982 (issue #1996): the store-label sidecar pins — BOTH streams ──
//
// The owner's 19.11 decision: the №952 kv sidecar is THE mechanism for
// label persistence through storage, generalized to mem/session (the
// store axis) and the file axis (write_file/append_file record, the
// read-back rides the recorded label), with NO write gate. The miss
// defaults: the stores answer `bottom` (the №951 owner decision), the
// file stream answers `public, untrusted` (a file nobody in THIS
// process wrote was written by someone outside — the №325 ingress
// default, unchanged).
//
// Separate file = separate process env: METALOGOS_STATIC_LABEL_GATE_OFF=1
// silences the static sink-clearance twin so the refusal can come ONLY
// from the runtime value-label line. The probes use INLINE forms (the
// №980 lesson) and non-secret untrusted data (query_param/exec output —
// the Category-A secret line stays silent). `exec` runs under
// METALOGOS_ALLOW_EXEC=1 (the named host permit) so the value exists
// and the label line can take the credit. THE SIGNATURES ARE PART OF
// THE PIN: mem_set(key, value) — key at #0; session_set(sid, key,
// value) — the KEY at #1 and the VALUE at #2; a wrong index is a
// mutation the pins catch.
#![allow(clippy::disallowed_methods)]

use std::path::Path;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program(source)
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(Path::new("examples").to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn seed_env() {
    std::env::set_var("METALOGOS_STATIC_LABEL_GATE_OFF", "1");
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
}

fn assert_refusal(backend: &str, err: &str) {
    assert!(
        err.contains("[SINK_CLEARANCE_RUNTIME]"),
        "{backend}: expected the sink-clearance runtime refusal on the \
         stored-label read-back, got: {}",
        err
    );
}

const MEM_UNTRUSTED_ROUNDTRIP: &str = r#"
pattern P(d: String) -> String {
  let q = query_param("r")
  mem_set("k982", q)
  let back = mem_get("k982")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

const MEM_TRUSTED_ROUNDTRIP: &str = r#"
pattern P(d: String) -> String {
  mem_set("k982t", "own config value")
  let back = mem_get("k982t")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

const SESSION_UNTRUSTED_ROUNDTRIP: &str = r#"
pattern P(d: String) -> String {
  let q = query_param("r")
  session_set("sid982", "k982s", q)
  let back = session_get("sid982", "k982s")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

const FILE_UNTRUSTED_ROUNDTRIP: &str = r#"
pattern P(d: String) -> String {
  let out = exec("echo hi")
  write_file("n982_probe_untrusted.txt", out)
  let back = read_file("n982_probe_untrusted.txt")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

const FILE_TRUSTED_ROUNDTRIP: &str = r#"
pattern P(d: String) -> String {
  write_file("n982_probe_trusted.txt", "own config v1")
  let back = read_file("n982_probe_trusted.txt")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

const KV_UNTRUSTED_ROUNDTRIP: &str = r#"
pattern P(d: String) -> String {
  let q = query_param("r")
  kv_set("k982kv", q)
  let back = kv_get("k982kv")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;

/// TW: the store axis's UNTRUSTED case — MEASURED HONESTLY: the refusal
/// fires at the STORE ENTRANCE (mem_set's runtime clearance, the
/// TAINT_PERSISTENCE deny class — untrusted data never enters the
/// store), NOT at the read-back: the entrance closure predates №982 and
/// shadows the sidecar's read-back for the untrusted axis. This pin is
/// a PRESERVATION pin (the laundering closure the independent red-team
/// pass confirmed stays); the read-back hand's isolation lives on the
/// FILE axis (the trusted-lift pins), where the entrance accepts the
/// write and only the sidecar decides the read-back.
#[test]
fn n982_tw_mem_roundtrip_the_untrusted_label_rides_the_read_back() {
    seed_env();
    if let Err(e) = run_tw(MEM_UNTRUSTED_ROUNDTRIP) {
        assert_refusal("TW/mem-roundtrip", &e);
    } else {
        panic!(
            "TW/mem-roundtrip: the stored untrusted value was laundered \
             through mem_set/mem_get into respond — the store-axis \
             sidecar is dead (№982)"
        );
    }
}

/// TW: the trusted value's own round-trip stays green — the recorded
/// trusted label (and the `bottom` miss default) never refuse a
/// trusted display. This is the «легальный round-trip» fixture the
/// done-when names.
#[test]
fn n982_tw_mem_roundtrip_the_trusted_value_stays_green() {
    seed_env();
    match run_tw(MEM_TRUSTED_ROUNDTRIP) {
        Ok(_) => {}
        Err(e) => panic!(
            "TW/mem-roundtrip-trusted: a trusted value's own round-trip \
             refused — the sidecar must carry the STORED label, not a \
             blanket refusal; got: {}",
            e
        ),
    }
}

/// TW: the session axis — session_set(sid, key, value) refuses the
/// untrusted value AT THE ENTRANCE (the same TAINT_PERSISTENCE runtime
/// clearance; the signatures are part of the pin — the KEY at #1, the
/// VALUE at #2, a wrong index changes nothing here but the trusted
/// lift's file pin catches a mis-indexed record). Preservation of the
/// entrance closure; the sidecar's session read-back arms stay correct
/// for the recorded-label form (the owner's one-table decision).
#[test]
fn n982_tw_session_roundtrip_the_untrusted_label_rides_the_read_back() {
    seed_env();
    if let Err(e) = run_tw(SESSION_UNTRUSTED_ROUNDTRIP) {
        assert_refusal("TW/session-roundtrip", &e);
    } else {
        panic!(
            "TW/session-roundtrip: the stored untrusted value was \
             laundered through session_set/session_get into respond — \
             the session arm is dead or mis-indexed (№982)"
        );
    }
}

/// TW: the file axis — the external content written and read back
/// refuses at respond. The verdict is stable under both mechanisms (the
/// recorded untrusted label AND the №325 miss default agree), so this
/// pin preserves the behavior; the hand isolation is the trusted-lift
/// pin below, whose verdict changes iff the record dies.
#[test]
fn n982_tw_file_roundtrip_the_untrusted_label_rides_the_read_back() {
    seed_env();
    if let Err(e) = run_tw(FILE_UNTRUSTED_ROUNDTRIP) {
        assert_refusal("TW/file-roundtrip", &e);
    } else {
        panic!(
            "TW/file-roundtrip: the stored external content was \
             laundered through write_file/read_file into respond — the \
             file-axis sidecar is dead (№982)"
        );
    }
}

/// TW: THE BEHAVIOR CHANGE on the trusted axis (the №325 default
/// pre-№982 refused the own-config round-trip at runtime): the recorded
/// trusted label LIFTS the read-back above the ingress default — the
/// legal own-config round-trip is green. If this pin reddens, the
/// sidecar stopped carrying the stored label (or the miss default
/// leaked over a recorded one).
#[test]
fn n982_tw_file_roundtrip_the_trusted_own_config_stays_green() {
    seed_env();
    match run_tw(FILE_TRUSTED_ROUNDTRIP) {
        Ok(_) => {}
        Err(e) => panic!(
            "TW/file-roundtrip-trusted: the legal own-config round-trip \
             refused — the recorded label must ride the read-back \
             (the owner's 19.11 decision); got: {}",
            e
        ),
    }
}

/// TW: the №952 kv contract is PRESERVED by the generalization — the
/// same table, the same bare keys, the same behavior (the entrance
/// closure refuses the untrusted value at kv_set, as before).
#[test]
fn n982_tw_kv_roundtrip_still_rides_the_sidecar() {
    seed_env();
    if let Err(e) = run_tw(KV_UNTRUSTED_ROUNDTRIP) {
        assert_refusal("TW/kv-roundtrip", &e);
    } else {
        panic!(
            "TW/kv-roundtrip: the №952 kv contract broke under the \
             generalization — the kv labels stopped riding the \
             read-back (№982 regression)"
        );
    }
}

/// TW: THE MEASURED BOUNDARY, pinned — a file written by NO ONE in this
/// process reads back through the miss default (`public, untrusted`):
/// the test pre-creates the file OUTSIDE the program (the
/// inter-process form), and the label line refuses the display. This
/// is the default the owner fixed («писал кто-то снаружи» = untrusted);
/// the INTER-PROCESS label transport itself stays open (the
/// threat-model residual).
#[test]
fn n982_tw_file_read_written_by_no_one_in_process_refuses() {
    seed_env();
    let path = Path::new("n982_probe_external.txt");
    std::fs::write(path, "written by the process HOST, not the program")
        .expect("the harness writes the out-of-process file");
    let src = r#"
pattern P(d: String) -> String {
  let back = read_file("n982_probe_external.txt")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let r = run_tw(src);
    let _ = std::fs::remove_file(path);
    if let Err(e) = r {
        assert_refusal("TW/file-no-record", &e);
    } else {
        panic!(
            "TW/file-no-record: content written OUTSIDE the process was \
             displayed without the ingress default — the file-stream \
             miss default is dead (№982)"
        );
    }
}

/// VM: the store axis on the executor backend — the ENTRANCE closure
/// (the same TAINT_PERSISTENCE refusal at mem_set), preserved. The
/// read-back hand's isolation lives on the file axis.
#[test]
fn n982_vm_mem_roundtrip_the_untrusted_label_rides_the_read_back() {
    seed_env();
    if let Err(e) = run_vm(MEM_UNTRUSTED_ROUNDTRIP) {
        assert_refusal("VM/mem-roundtrip", &e);
    } else {
        panic!(
            "VM/mem-roundtrip: the stored untrusted value was laundered \
             on the executor — the VM's store-axis record is dead (№982)"
        );
    }
}

/// VM: the file axis on the executor backend.
#[test]
fn n982_vm_file_roundtrip_the_untrusted_label_rides_the_read_back() {
    seed_env();
    if let Err(e) = run_vm(FILE_UNTRUSTED_ROUNDTRIP) {
        assert_refusal("VM/file-roundtrip", &e);
    } else {
        panic!(
            "VM/file-roundtrip: the stored external content was \
             laundered on the executor — the VM's file-axis record is \
             dead (№982)"
        );
    }
}

/// VM: the trusted lift on the executor backend (the behavior change).
#[test]
fn n982_vm_file_roundtrip_the_trusted_own_config_stays_green() {
    seed_env();
    match run_vm(FILE_TRUSTED_ROUNDTRIP) {
        Ok(_) => {}
        Err(e) => panic!(
            "VM/file-roundtrip-trusted: the legal own-config round-trip \
             refused on the executor; got: {}",
            e
        ),
    }
}

/// VM: the miss default on the executor backend (the boundary pin).
#[test]
fn n982_vm_file_read_written_by_no_one_in_process_refuses() {
    seed_env();
    let path = Path::new("n982_probe_external_vm.txt");
    std::fs::write(path, "written by the process HOST, not the program")
        .expect("the harness writes the out-of-process file");
    let src = r#"
pattern P(d: String) -> String {
  let back = read_file("n982_probe_external_vm.txt")
  let _ = respond(back)
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let r = run_vm(src);
    let _ = std::fs::remove_file(path);
    if let Err(e) = r {
        assert_refusal("VM/file-no-record", &e);
    } else {
        panic!(
            "VM/file-no-record: out-of-process content displayed \
             without the ingress default (№982)"
        );
    }
}
