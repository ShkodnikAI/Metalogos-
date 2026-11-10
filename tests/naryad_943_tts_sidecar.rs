// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Наряд №943 (волна 95 — W-Q, №Q2): the C2PA generation-path egress in
//    code — the tts_generate sidecar (ADR-0222) ──
//
// Контракты (the ADR-0222 §5 roster):
//   G1 (golden): tts_generate против mock-базы (METALOGOS_TTS_BASE_URL,
//       без API-ключей — инфраструктура №279) пишет sidecar
//       `<fname>.manifest.json` рядом с аудиофайлом; запись несёт
//       kind="tts_generate", origin="", conf="unspecified",
//       synthetic:true, provider/model/voice фактического вызова
//       (дефолты "openai"/"tts-1" включительно), bytes_sha256 == SHA-256
//       байтов, реально лежащих на диске; timestamp — RFC 3339 (не
//       пиннится, как у обоих сестринских манифестов); JSON — pretty
//       (форма №241). TW и VM — один контракт (ADR-0156).
//   G2 (compat): media_manifest_read над 9-полевым документом читает
//       6-полевую базу №706 (лишние provider/model/voice игнорируются
//       серде-постурой — MediaManifest без deny_unknown_fields); пин
//       через настоящий builtin и через sidecar_parse_manifest.
//   G3 (негатив — в src, unit): провал sidecar-записи → громкая ошибка
//       `tts_generate(): …` (механика §2.2 — tests in llm.rs).
//   Границы: возвращаемое значение — путь (№279 не меняется); строковые
//   золотые не тронуты; spec-таблица не растёт (registry не меняется).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const MOCK_AUDIO: &[u8] = b"FAKE-MP3-BYTES-943-RIFFxxxxWAVEfmt";

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Mock TTS server: loud-checks the Authorization header and the JSON
/// body, then responds with MOCK_AUDIO bytes (the №279 infrastructure —
/// copied verbatim; accepts in a LOOP: one test drives BOTH backends).
fn spawn_tts_mock() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().unwrap().to_string();
    let _handle = std::thread::spawn(move || {
        use std::io::{Read, Write};
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut raw: Vec<u8> = Vec::new();
            let mut buf = [0u8; 16384];
            let header_end = loop {
                let n = stream.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break None;
                }
                raw.extend_from_slice(&buf[..n]);
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    break Some(pos + 4);
                }
            };
            if let Some(hend) = header_end {
                let headers = String::from_utf8_lossy(&raw[..hend]).to_lowercase();
                let content_length: usize = headers
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                while raw.len() - hend < content_length {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                }
                let req = String::from_utf8_lossy(&raw).to_lowercase();
                let auth_ok = req.contains("authorization: bearer test-key-943");
                let body_ok = req.contains("\"model\"") && req.contains("\"voice\"");
                let (code, payload) = if auth_ok && body_ok {
                    ("200 OK", MOCK_AUDIO.to_vec())
                } else {
                    ("401 Unauthorized", b"unauthorized".to_vec())
                };
                let resp = format!(
                    "HTTP/1.1 {}\r\nContent-Type: audio/mpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    code,
                    payload.len()
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.write_all(&payload);
                let _ = stream.flush();
            }
        }
    });
    format!("http://{}", addr)
}

fn set_tts_env(base: &str) -> (Option<String>, Option<String>, Option<String>) {
    let prev_base = std::env::var("METALOGOS_TTS_BASE_URL").ok();
    let prev_key = std::env::var("METALOGOS_TTS_API_KEY").ok();
    let prev_openai = std::env::var("OPENAI_API_KEY").ok();
    std::env::set_var("METALOGOS_TTS_BASE_URL", base);
    std::env::set_var("METALOGOS_TTS_API_KEY", "test-key-943");
    std::env::remove_var("OPENAI_API_KEY"); // METALOGOS_TTS_API_KEY suffices
    (prev_base, prev_key, prev_openai)
}

fn restore_tts_env(prev: (Option<String>, Option<String>, Option<String>)) {
    match prev.0 {
        Some(v) => std::env::set_var("METALOGOS_TTS_BASE_URL", v),
        None => std::env::remove_var("METALOGOS_TTS_BASE_URL"),
    }
    match prev.1 {
        Some(v) => std::env::set_var("METALOGOS_TTS_API_KEY", v),
        None => std::env::remove_var("METALOGOS_TTS_API_KEY"),
    }
    match prev.2 {
        Some(v) => std::env::set_var("OPENAI_API_KEY", v),
        None => std::env::remove_var("OPENAI_API_KEY"),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(repo_root());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const GEN_DEFAULTS: &str = r#"
pattern Gen(x: String) -> String {
  let p = tts_generate("Привет, мир", "alloy")
  return p
}
flow Main {
  input: String = "go"
  -> Gen
  -> output
}
"#;

const GEN_EXPLICIT: &str = r#"
pattern Gen(x: String) -> String {
  let p = tts_generate("Привет, мир", "nova", "openai", "tts-1-hd")
  return p
}
flow Main {
  input: String = "go"
  -> Gen
  -> output
}
"#;

/// G1: the golden sidecar — TW (defaults) and VM (explicit args). The
/// sidecar exists next to the file, carries the №706 base + the event
/// fields, and bytes_sha256 == the hash of the bytes ACTUALLY ON DISK.
#[test]
#[serial_test::serial]
fn n943_tts_generate_sidecar_golden_tw_and_vm() {
    let _env = lock_env();
    let base = spawn_tts_mock();
    let prev = set_tts_env(&base);

    // ── TW, defaults: provider/model come from the builtin resolution ──
    // The fs sandbox root is the process CWD (the №131 sandbox — the
    // n279 golden posture): both backends' files land there, read and
    // cleaned by exact name (the returned path → no globbing races).
    let out_tw = metalogos::run_program_with_dir(GEN_DEFAULTS.trim(), repo_root())
        .expect("TW must synthesize against the mock");
    let fname_tw = out_tw.expect("flow returns the path").trim().to_string();
    // The №279 contract UNCHANGED: the returned value is the path.
    assert!(
        !fname_tw.is_empty() && !fname_tw.contains('/'),
        "returned value is the sandbox-relative path, got: {}",
        fname_tw
    );
    let audio_bytes =
        std::fs::read(&fname_tw).unwrap_or_else(|e| panic!("TW: read audio {}: {}", fname_tw, e));
    assert_eq!(audio_bytes, MOCK_AUDIO, "TW: audio bytes byte-for-byte");
    // The №241 naming: `<path>.manifest.json` next to the audio.
    let sidecar_name = format!("{}.manifest.json", fname_tw);
    let sidecar_json = std::fs::read_to_string(&sidecar_name)
        .unwrap_or_else(|e| panic!("TW: sidecar must exist next to the file: {}", e));
    let v: serde_json::Value =
        serde_json::from_str(&sidecar_json).expect("TW: sidecar is valid JSON");
    assert_eq!(v["kind"], "tts_generate", "TW: the GENERATION EVENT kind");
    assert_eq!(v["origin"], "", "TW: no declared origin at synthesis time");
    assert_eq!(v["conf"], "unspecified", "TW: the ADR-0125 honest marker");
    assert_eq!(v["synthetic"], true, "TW: synthetic BY CONSTRUCTION");
    assert_eq!(
        v["bytes_sha256"],
        sha256_hex(&audio_bytes),
        "TW: hash == the bytes actually on disk"
    );
    assert_eq!(v["provider"], "openai", "TW: the resolved default");
    assert_eq!(v["model"], "tts-1", "TW: the resolved default");
    assert_eq!(v["voice"], "alloy", "TW: the actual call argument");
    // The timestamp is RFC 3339 (the form pinned, not the value — the
    // both-siblings posture).
    let ts = v["timestamp"].as_str().expect("timestamp is a string");
    chrono::DateTime::parse_from_rfc3339(ts).expect("TW: timestamp parses as RFC 3339");
    // The pretty №241 form — provenance you cannot read is provenance
    // you do not have.
    assert!(
        sidecar_json.contains('\n'),
        "TW: the sidecar is pretty JSON"
    );

    // ── VM, explicit args: the same contract, the event fields carry ──
    //    the ACTUAL call arguments (the crosscheck — ADR-0156).
    let out_vm = run_vm(GEN_EXPLICIT).expect("VM must synthesize against the mock");
    let fname_vm = out_vm.expect("flow returns the path").trim().to_string();
    assert!(
        !fname_vm.is_empty() && !fname_vm.contains('/'),
        "VM: returned value is the sandbox-relative path, got: {}",
        fname_vm
    );
    let vm_audio = std::fs::read(repo_root().join(&fname_vm))
        .unwrap_or_else(|e| panic!("VM: read audio {}: {}", fname_vm, e));
    assert_eq!(vm_audio, MOCK_AUDIO, "VM: audio bytes byte-for-byte");
    let vm_sidecar =
        std::fs::read_to_string(repo_root().join(format!("{}.manifest.json", fname_vm)))
            .unwrap_or_else(|e| panic!("VM: sidecar must exist next to the file: {}", e));
    let vm_v: serde_json::Value =
        serde_json::from_str(&vm_sidecar).expect("VM: sidecar is valid JSON");
    assert_eq!(vm_v["kind"], "tts_generate", "VM: the generation kind");
    assert_eq!(vm_v["synthetic"], true, "VM: synthetic BY CONSTRUCTION");
    assert_eq!(
        vm_v["bytes_sha256"],
        sha256_hex(&vm_audio),
        "VM: hash == the bytes actually on disk"
    );
    assert_eq!(vm_v["provider"], "openai", "VM: the explicit provider");
    assert_eq!(vm_v["model"], "tts-1-hd", "VM: the explicit model rides");
    assert_eq!(vm_v["voice"], "nova", "VM: the explicit voice rides");

    // The fixture must not leak (audio + sidecar, both backends).
    let _ = std::fs::remove_file(&fname_tw);
    let _ = std::fs::remove_file(&sidecar_name);
    let _ = std::fs::remove_file(&fname_vm);
    let _ = std::fs::remove_file(format!("{}.manifest.json", fname_vm));
    restore_tts_env(prev);
}

/// G2: media_manifest_read over the tts sidecar returns the 6-field
/// №706 base with the additive fields ignored (the serde posture pin;
/// a break flips to the separate-schema honesty ADR-0222 already
/// states). Direct parse + the REAL builtin in-program, TW and VM.
#[test]
#[serial_test::serial]
fn n943_media_manifest_read_reads_tts_sidecar_base() {
    let _env = lock_env();
    let base = spawn_tts_mock();
    let prev = set_tts_env(&base);

    // Synthesize (TW) — the sidecar lands next to the audio (CWD sandbox).
    let out = metalogos::run_program_with_dir(GEN_DEFAULTS.trim(), repo_root())
        .expect("TW must synthesize against the mock");
    let fname = out.expect("flow returns the path").trim().to_string();
    let sidecar_name = format!("{}.manifest.json", fname);

    // The direct serde posture: the parser reads the 9-field document
    // as the 6-field base (provider/model/voice are unknown fields —
    // MediaManifest has no deny_unknown_fields).
    let json = std::fs::read_to_string(&sidecar_name).expect("sidecar exists");
    assert!(
        json.contains("\"provider\"") && json.contains("\"model\"") && json.contains("\"voice\""),
        "the additive fields ARE in the document"
    );
    let manifest = metalogos::media::sidecar_parse_manifest(&json)
        .expect("the tts sidecar reads as the №706 base");
    assert_eq!(manifest.kind, "tts_generate");
    assert_eq!(manifest.origin, "");
    assert_eq!(manifest.conf, "unspecified");
    assert!(manifest.synthetic, "conservative read: true");
    let disk = std::fs::read(&fname).expect("audio on disk");
    assert_eq!(manifest.bytes_sha256, sha256_hex(&disk));

    // The REAL builtin in-program (TW): the base fields read correctly
    // over the 9-field document.
    let probe = format!(
        r#"
pattern P(_t: String) -> String {{
  let m = media_manifest_read("{sidecar}")
  if m.synthetic == true {{
    if m.kind == "tts_generate" {{
      if m.origin == "" {{
        if m.conf == "unspecified" {{
          if m.bytes_sha256 == "{hash}" {{
            return "compat-ok"
          }}
        }}
      }}
    }}
  }}
  return "compat-broken"
}}
flow Main {{ input: String = "t" -> P -> output }}
"#,
        sidecar = sidecar_name,
        hash = sha256_hex(&disk)
    );
    let out_tw = metalogos::run_program_with_dir(probe.trim(), repo_root())
        .expect("the compat probe runs on TW");
    assert_eq!(
        out_tw.as_deref().unwrap_or_default().trim_end(),
        "compat-ok",
        "TW: media_manifest_read reads the tts sidecar base"
    );

    // VM parity of the read path over the same document shape.
    let out_vm = run_vm(&probe).expect("the compat probe runs on VM");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "compat-ok",
        "VM: media_manifest_read reads the tts sidecar base"
    );

    let _ = std::fs::remove_file(&fname);
    let _ = std::fs::remove_file(&sidecar_name);
    restore_tts_env(prev);
}
