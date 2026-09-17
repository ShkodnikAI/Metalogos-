#!/usr/bin/env python3
"""Generate src/builtins_classification.rs + REFERENCE.md classification block.

Наряд №316 (issue #403, план v2 §16.2): SSOT-классификация builtins —
роль × метка × обратимость.

Direction of truth:
  1. The CURATED table below (this script) is the authoring surface.
  2. It emits the static Rust map `src/builtins_classification.rs` — the
     committed SSOT the compiler and tests see.
  3. The REFERENCE.md block is generated FROM the committed Rust map
     (parse back), so the doc can never drift from the code.

Rules:
  - every builtin must be classified (coverage enforced by the Rust test);
  - every name in a RISKY category must have an EXPLICIT override here —
    silent category defaults are only allowed for provably-pure categories;
  - every non-Pure role carries a one-sentence rationale.

Run from repo root: python3 scripts/gen_classification.py
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
REGISTRY = REPO / "src" / "builtins" / "registry.rs"
OUT_RS = REPO / "src" / "builtins_classification.rs"
REFERENCE = REPO / "REFERENCE.md"

BEGIN = "<!-- BEGIN GENERATED BUILTIN CLASSIFICATION (scripts/gen_classification.py — do not edit inside) -->"
END = "<!-- END GENERATED BUILTIN CLASSIFICATION -->"

RUST_TEMPLATE = r'''// ── Builtin classification: role × label × reversibility (Наряд №316) ──
//
// SSOT static map for ALL registered builtins (устав §11 Шаг 3, план v2
// §13.2 шаг 0.3, §16.2). Authored by scripts/gen_classification.py from
// its curated table; the REFERENCE.md classification block is generated
// FROM this map. Tests in this module enforce: coverage 100% of
// BUILTIN_REGISTRY, uniqueness, no extras, rationale on every non-Pure.
//
// Role semantics:
// - Pure   — deterministic compute on in-program values (no ingress/egress,
//            no persistent-state effect). Parsing already-present bytes is
//            Pure.
// - Source — brings data from beyond the expression boundary INTO the
//            program: external services, files, env, wall clock, entropy,
//            AND the runtime's own persistent stores (state reads carry
//            provenance).
// - Lift   — raises the clearance of its input for further flow: taint
//            sanitizers (redact/render/escape_*, the audit.rs vocabulary)
//            and one-way de-identifiers (hash_password, encrypt, digests).
// - Sink   — sends data beyond the expression boundary OUT: public
//            channels, network delivery, persistent state writes, host
//            effects.
//
// Label semantics (default_label — the clearance of data at the builtin's
// boundary by default; №316 proposal, refined by the Фаза 1 label-checker):
// - Public   — safe for public channels (sanitized outputs, stdout)
// - Internal — program/user data inside the perimeter (local files, DB,
//              persistent stores, untrusted user input)
// - Secret   — secrets and biometrics (env/secret, keys, plaintext
//              credentials, voiceprints — GDPR Art. 9)
// - Network  — data crossing the network boundary (external services)
//
// Reversibility semantics (the EFFECT, not the function):
// - Pure         — no external effect to undo (local compute)
// - Reversible   — external/persistent effect that CAN be undone
// - Irreversible — external/persistent effect that CANNOT be undone
//                  (delivery, host exec, destructive deletes)

/// Data-clearance label at the builtin boundary (№316 proposal;
/// refined by the Фаза 1 label-checker per план v2 §16.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    Public,
    Internal,
    Secret,
    Network,
}

impl Label {
    pub fn as_str(&self) -> &'static str {
        match self {
            Label::Public => "public",
            Label::Internal => "internal",
            Label::Secret => "secret",
            Label::Network => "network",
        }
    }
}

/// Data-flow role of a builtin (№316, устав §11 Шаг 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Pure,
    Source,
    Lift,
    Sink,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Pure => "pure",
            Role::Source => "source",
            Role::Lift => "lift",
            Role::Sink => "sink",
        }
    }
}

/// Undoability of the builtin's external effect (№316).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reversibility {
    Pure,
    Reversible,
    Irreversible,
}

impl Reversibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Reversibility::Pure => "pure",
            Reversibility::Reversible => "reversible",
            Reversibility::Irreversible => "irreversible",
        }
    }
}

/// Classification of one builtin (№316): role × default label ×
/// reversibility + one-sentence rationale for every non-Pure role.
#[derive(Debug, Clone, Copy)]
pub struct BuiltClass {
    pub role: Role,
    pub default_label: Label,
    pub reversibility: Reversibility,
    pub rationale: &'static str,
}

/// One classified registry entry.
#[derive(Debug, Clone, Copy)]
pub struct BuiltClassEntry {
    pub name: &'static str,
    pub class: BuiltClass,
}

/// Classification lookup — linear scan over a small static array; the map
/// is compile-time data, uniqueness is test-enforced.
pub fn classify(name: &str) -> Option<&'static BuiltClass> {
    BUILTIN_CLASSES.iter().find(|e| e.name == name).map(|e| &e.class)
}

/// SSOT map: имя → BuiltClass for EVERY registered builtin (№316).
pub static BUILTIN_CLASSES: &[BuiltClassEntry] = &[
{ENTRIES}
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Source-level registry names — NOT the cfg-filtered compiled registry:
    /// the classification map is complete at the source level, while
    /// BUILTIN_REGISTRY at runtime is feature-gated (video/voice/vec... are
    /// cfg'd out under default features). include_str! keeps the tests
    /// feature-independent (№316 coverage is a source-level contract).
    fn registry_source_names() -> HashSet<String> {
        let src = include_str!("builtins/registry.rs");
        let mut out = HashSet::new();
        let mut rest = src;
        while let Some(i) = rest.find("spec!(\"") {
            let after = &rest[i + 7..];
            if let Some(j) = after.find('"') {
                let line_start = rest[..i].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_end = rest[i..].find('\n').map(|p| i + p).unwrap_or(rest.len());
                let line = &rest[line_start..line_end];
                let code = line.split("//").next().unwrap_or(line);
                if code.contains("spec!(") {
                    out.insert(after[..j].to_string());
                }
            }
            rest = &rest[i + 7..];
        }
        out
    }

    fn registry_names() -> HashSet<String> {
        registry_source_names()
    }

    /// №316 «Сделано, когда» (а): every registered builtin is classified.
    #[test]
    fn coverage_every_registered_builtin_is_classified() {
        let map: HashSet<&str> = BUILTIN_CLASSES.iter().map(|e| e.name).collect();
        let missing: Vec<String> = registry_names()
            .into_iter()
            .filter(|n| !map.contains(n.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "builtins without classification ({}): {:?}",
            missing.len(),
            missing
        );
    }

    /// №316 «Сделано, когда» (а): the map has no extra names.
    #[test]
    fn no_extra_names() {
        let registry = registry_names();
        let extras: Vec<&str> = BUILTIN_CLASSES
            .iter()
            .map(|e| e.name)
            .filter(|n| !registry.contains(*n))
            .collect();
        assert!(extras.is_empty(), "classified names not in registry: {:?}", extras);
    }

    /// №316 «Сделано, когда» (а): rationale on every non-Pure entry.
    #[test]
    fn rationale_on_every_non_pure() {
        let bad: Vec<&str> = BUILTIN_CLASSES
            .iter()
            .filter(|e| e.class.role != Role::Pure && e.class.rationale.trim().is_empty())
            .map(|e| e.name)
            .collect();
        assert!(bad.is_empty(), "non-Pure without rationale: {:?}", bad);
    }

    /// №316: uniqueness of the map (one class per name).
    #[test]
    fn map_is_unique() {
        let mut seen = HashSet::new();
        let dupes: Vec<&str> = BUILTIN_CLASSES
            .iter()
            .map(|e| e.name)
            .filter(|n| !seen.insert(*n))
            .collect();
        assert!(dupes.is_empty(), "duplicate classifications: {:?}", dupes);
    }

    /// №316: the named minimum classes from the issue are present.
    #[test]
    fn issue_minimum_classes() {
        let expect_sink = [
            "http_post", "write_file", "send_message", "print", "db_execute", "tts_send",
        ];
        for n in expect_sink {
            let c = classify(n).unwrap_or_else(|| panic!("{}", n));
            assert_eq!(c.role, Role::Sink, "{} must be a Sink", n);
        }
        let expect_source = ["http_get", "env", "json_body", "form_data", "query_param"];
        for n in expect_source {
            let c = classify(n).unwrap_or_else(|| panic!("{}", n));
            assert_eq!(c.role, Role::Source, "{} must be a Source", n);
        }
        for n in ["exec", "git_push", "delete_file"] {
            let c = classify(n).unwrap_or_else(|| panic!("{}", n));
            assert_eq!(c.reversibility, Reversibility::Irreversible, "{}", n);
        }
        let redact = classify("redact").unwrap();
        assert_eq!(redact.role, Role::Lift, "redact — taint-sanitizer lift (ADR-0136)");
    }

    /// №316: Sink/Source/Lift/Pure distribution is sane (sanity counts,
    /// guards against a silent default Pure swallowing the risky surface).
    #[test]
    fn distribution_sanity() {
        let n = |r: Role| BUILTIN_CLASSES.iter().filter(|e| e.class.role == r).count();
        assert!(n(Role::Sink) >= 60, "sinks: {}", n(Role::Sink));
        assert!(n(Role::Source) >= 40, "sources: {}", n(Role::Source));
        assert!(n(Role::Lift) >= 5, "lifts: {}", n(Role::Lift));
        assert!(n(Role::Pure) >= 200, "pure: {}", n(Role::Pure));
    }

    /// №316 «Сделано, когда» (б): the REFERENCE.md classification block is
    /// regenerated from THIS map and matches it exactly (doc cannot drift).
    #[test]
    fn reference_classification_block_matches_map() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let path = std::path::Path::new(&manifest).join("REFERENCE.md");
        let reference = std::fs::read_to_string(&path).unwrap_or_default();
        let begin = "BEGIN GENERATED BUILTIN CLASSIFICATION";
        let end = "END GENERATED BUILTIN CLASSIFICATION";
        let (block, found) = match (reference.find(begin), reference.find(end)) {
            (Some(b), Some(e)) if b < e => (&reference[b..e], true),
            _ => ("", false),
        };
        assert!(found, "REFERENCE.md must contain the classification block markers");

        let mut expected = String::from(
            "| Builtin | Role | Default label | Reversibility |\n|---|---|---|---|\n",
        );
        for e in BUILTIN_CLASSES {
            let role = e.class.role.as_str();
            let label = e.class.default_label.as_str();
            let rev = e.class.reversibility.as_str();
            if e.class.rationale.is_empty() {
                expected.push_str(&format!(
                    "| `{}` | {} | {} | {} | — |\n",
                    e.name, role, label, rev
                ));
            } else {
                expected.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} |\n",
                    e.name, role, label, rev, e.class.rationale
                ));
            }
        }

        let block_rows: Vec<String> = block
            .lines()
            .filter(|l| l.starts_with("| `"))
            .map(|l| l.trim_end().to_string())
            .collect();
        let expected_rows: Vec<String> = expected
            .lines()
            .skip(2)
            .map(|l| l.trim_end().to_string())
            .collect();
        assert_eq!(
            block_rows.len(),
            expected_rows.len(),
            "REFERENCE block row count {} != map count {}",
            block_rows.len(),
            expected_rows.len()
        );
        for (got, want) in block_rows.iter().zip(expected_rows.iter()) {
            assert_eq!(got, want, "REFERENCE block row drift");
        }
    }
}
'''

# ── Curated overrides: name -> (role, label, reversibility, rationale) ──
OVERRIDES = {
    # ── io ──
    "print": ("Sink", "Public", "Irreversible", "prints to the public stdout channel — SECRET_LEAK semantics (№157), cannot be unsaid"),
    "read_file": ("Source", "Internal", "Pure", "ingests external file content into the program (input by provenance)"),
    "write_file": ("Sink", "Internal", "Reversible", "writes persistent local state (undoable by file deletion)"),
    "append_file": ("Sink", "Internal", "Reversible", "appends to persistent local state (undoable by truncation)"),
    "delete_file": ("Sink", "Internal", "Irreversible", "destroys a local file with no undo path (issue minimum list)"),
    "file_exists": ("Source", "Internal", "Pure", "reads filesystem metadata (state probe)"),
    "list_dir": ("Source", "Internal", "Pure", "reads filesystem directory state"),
    "exec": ("Sink", "Internal", "Irreversible", "arbitrary host command execution — external effect on the host that cannot be undone (issue minimum list)"),
    "exec_argv": ("Sink", "Internal", "Irreversible", "argv-form of exec — same irreversible host effect"),
    "git_push": ("Sink", "Network", "Irreversible", "pushes to a remote repository — external, non-undoable effect (issue minimum list)"),
    "mcp_call": ("Source", "Network", "Pure", "ingests untrusted MCP tool output — UserInput taint by ADR-0132 D3"),
    "mcp_list_tools": ("Source", "Network", "Pure", "ingests external tool metadata over MCP (not tainted per ADR-0132, still external ingress)"),
    # ── web ──
    "respond": ("Sink", "Public", "Irreversible", "writes the HTTP response — public channel, cannot be unsent"),
    "respond_html": ("Sink", "Public", "Irreversible", "writes the HTTP response as HTML — public channel (escaping contract)"),
    "form_data": ("Source", "Internal", "Pure", "ingests untrusted user form input — UserInput taint (№201 vocabulary)"),
    "json_body": ("Source", "Internal", "Pure", "ingests untrusted request body — UserInput taint"),
    "query_param": ("Source", "Internal", "Pure", "ingests untrusted request query parameter — UserInput taint"),
    "request_body": ("Source", "Internal", "Pure", "alias of json_body — untrusted request body ingress"),
    "redact": ("Lift", "Public", "Pure", "taint-sanitizer — removes Secret taint (mask before sink, ADR-0136; audit.rs redact_result_taint)"),
    "escape_html": ("Lift", "Public", "Pure", "HTML-escapes its input — taint-sanitizer per audit.rs (Sanitized)"),
    "escape_js": ("Lift", "Public", "Pure", "JS-escapes its input — sanitizer family of escape_html"),
    "escape_json": ("Lift", "Public", "Pure", "JSON-escapes its input — sanitizer family of escape_html"),
    "render": ("Lift", "Public", "Pure", "taint-sanitizing template render — output is public-safe (audit.rs sanitizer)"),
    "html_render": ("Lift", "Public", "Pure", "sanitizing HTML render — output is public-safe"),
    "template_render": ("Lift", "Public", "Pure", "auto-escaped template rendering — output is public-safe"),
    "http_get": ("Source", "Network", "Pure", "ingests external network data (SSRF-guarded, №130)"),
    "http_post": ("Sink", "Network", "Irreversible", "transmits program data to an external endpoint — cannot be unsent (issue minimum list)"),
    "http_post_multipart": ("Sink", "Network", "Irreversible", "multipart upload to an external endpoint — same egress as http_post"),
    "http_download": ("Source", "Network", "Reversible", "ingests remote bytes to a local file (network ingress with a disk side-effect)"),
    "require": ("Source", "Internal", "Pure", "reads and enforces request context (auth/rate precondition state)"),
    "server_path_param": ("Source", "Internal", "Pure", "ingests untrusted request path parameter — UserInput taint"),
    "web_search": ("Source", "Network", "Pure", "ingests external search results"),
    "geo_ip": ("Source", "Network", "Pure", "ingests external geolocation data"),
    "weather": ("Source", "Network", "Pure", "ingests external weather data"),
    "weather_forecast": ("Source", "Network", "Pure", "ingests external forecast data"),
    "geo_distance": ("Pure", "Public", "Pure", "pure spherical math on coordinates"),
    # ── db / event ──
    "query": ("Source", "Internal", "Pure", "reads the program's persistent DB (state input with provenance)"),
    "db_execute": ("Sink", "Internal", "Irreversible", "arbitrary SQL write against the persistent DB — destructive statements are non-undoable (issue minimum list)"),
    "db_insert": ("Sink", "Internal", "Irreversible", "intended DB row insert (registry-only stub) — persistent write"),
    "query_scalar": ("Source", "Internal", "Pure", "VM-native DB read — state input"),
    "query_row": ("Source", "Internal", "Pure", "VM-native DB read — state input"),
    "event_count": ("Source", "Internal", "Pure", "VM-native event-log read — state input"),
    "events_since": ("Source", "Internal", "Pure", "VM-native event-log read — state input"),
    "event_sum": ("Source", "Internal", "Pure", "VM-native event-log aggregation — state input"),
    # ── system / env ──
    "env": ("Source", "Secret", "Pure", "ingests environment secrets — Secret taint (audit.rs)"),
    "secret": ("Source", "Secret", "Pure", "materializes a Secret value — Secret taint (№172)"),
    "replay_snapshot": ("Source", "Internal", "Pure", "reads runtime snapshot state"),
    "policy_check": ("Source", "Internal", "Pure", "reads runtime policy state"),
    # ── crypto ──
    "hash_password": ("Lift", "Public", "Pure", "one-way de-identification of a password — output is safe for storage (argon2)"),
    "verify_password": ("Pure", "Secret", "Pure", "local credential comparison — handles secrets, no egress"),
    "encrypt": ("Lift", "Public", "Pure", "ciphertext is safe for untrusted channels — sensitivity lifted (AES-GCM)"),
    "decrypt": ("Pure", "Secret", "Pure", "produces Secret plaintext from ciphertext (label-raising transform)"),
    "generate_key": ("Source", "Secret", "Pure", "materializes a fresh Secret from CSPRNG entropy"),
    "sha256": ("Lift", "Public", "Pure", "one-way digest — de-identifies its input (used to hash secrets)"),
    "hmac_sha256": ("Lift", "Public", "Pure", "keyed digest — de-identifies its input"),
    "hex_encode": ("Pure", "Public", "Pure", "deterministic byte-to-text encoding"),
    "hex_decode": ("Pure", "Public", "Pure", "deterministic text-to-byte decoding"),
    "base64_encode": ("Pure", "Public", "Pure", "deterministic encoding — transport, NOT a sanitizer"),
    "base64_decode": ("Pure", "Public", "Pure", "deterministic decoding"),
    # ── llm (role = Source per ADR-0117 untrusted output; prompt-egress dual noted) ──
    "call_llm": ("Source", "Network", "Pure", "ingests untrusted model output (LlmOutput taint, ADR-0117); DUAL: the prompt is transmitted to an external provider — №317 corpus must cover prompt-egress"),
    "call_claude": ("Source", "Network", "Pure", "ingests untrusted model output (LlmOutput taint); DUAL: prompt egress to provider"),
    "call_llm_schema": ("Source", "Network", "Pure", "ingests schema-validated (still untrusted) model output; DUAL: prompt egress"),
    "llm_usage": ("Source", "Internal", "Pure", "reads LLM usage accounting state"),
    "llm_stream_open": ("Source", "Network", "Pure", "opens an external SSE stream — ingests untrusted model output; DUAL: prompt egress"),
    "llm_stream_next": ("Source", "Network", "Pure", "ingests the next untrusted model chunk from the external stream"),
    "llm_stream_close": ("Sink", "Network", "Reversible", "closes the external stream (cleanup effect, no data egress)"),
    "json_validate": ("Pure", "Public", "Pure", "deterministic validation of an in-program value"),
    # ── memory / kv / session persistence ──
    "kv_set": ("Sink", "Internal", "Reversible", "persists to the program KV store (redact-before-persist per ADR-0136 applies)"),
    "kv_get": ("Source", "Internal", "Pure", "reads the program KV store — state input"),
    "kv_delete": ("Sink", "Internal", "Irreversible", "destroys a persisted KV entry — no undo"),
    "kv_exists": ("Source", "Internal", "Pure", "reads KV store state"),
    "kv_list": ("Source", "Internal", "Pure", "reads KV store state"),
    "mem_set": ("Sink", "Internal", "Reversible", "persists to long-term memory store"),
    "mem_get": ("Source", "Internal", "Pure", "reads long-term memory store"),
    "mem_delete": ("Sink", "Internal", "Irreversible", "destroys a memory entry — no undo"),
    "memorize": ("Sink", "Internal", "Reversible", "alias of kv_set — persists to the memory store"),
    "session_set": ("Sink", "Internal", "Reversible", "persists web session state"),
    "session_get": ("Source", "Internal", "Pure", "reads web session state"),
    "session_clear": ("Sink", "Internal", "Irreversible", "wipes session state — no undo"),
    "session_login": ("Sink", "Internal", "Reversible", "creates a session (registry-only stub intent)"),
    "session_logout": ("Sink", "Internal", "Reversible", "destroys the current session (stub intent)"),
    "authenticate": ("Pure", "Secret", "Pure", "credential verification — handles secrets locally, no egress (stub intent)"),
    "vec_store": ("Sink", "Internal", "Reversible", "persists embeddings into the vector store (ADR-0134)"),
    "vec_search": ("Source", "Internal", "Pure", "reads the vector store (KNN state input)"),
    "embed": ("Pure", "Public", "Pure", "local embedding model compute on in-program text (№272/ADR-0134)"),
    "memory_decay": ("Sink", "Internal", "Reversible", "adjusts memory weights — undoable state change"),
    "memory_boost": ("Sink", "Internal", "Reversible", "adjusts memory weights — undoable state change"),
    "memory_prune": ("Sink", "Internal", "Irreversible", "destructively removes memory entries — no undo"),
    "memory_revise": ("Sink", "Internal", "Reversible", "revises memory entries — undoable state change"),
    "memory_forget": ("Sink", "Internal", "Irreversible", "destructively forgets memory (№280) — no undo"),
    "user_profile": ("Source", "Internal", "Pure", "reads persisted user profile (PII state input)"),
    "ref": ("Source", "Internal", "Pure", "creates a reference into the content store — state read"),
    "deref": ("Source", "Internal", "Pure", "reads content store state"),
    # ── graph / mtree ──
    "graph_query": ("Source", "Internal", "Pure", "reads the global memory graph — state input"),
    "graph_path": ("Source", "Internal", "Pure", "reads the global memory graph"),
    "graph_neighbors": ("Source", "Internal", "Pure", "reads the global memory graph"),
    "subgraph_extract": ("Source", "Internal", "Pure", "extracts a subgraph value from global graph state"),
    "subgraph_nodes": ("Pure", "Public", "Pure", "pure compute on a passed Subgraph value"),
    "subgraph_json": ("Pure", "Public", "Pure", "pure serialization of a passed Subgraph value"),
    "trace_start": ("Sink", "Internal", "Reversible", "mutates trace state"),
    "trace_end": ("Sink", "Internal", "Reversible", "mutates trace state"),
    "mtree_summarize": ("Source", "Internal", "Pure", "reads memory-tree state"),
    "mtree_retrieve": ("Source", "Internal", "Pure", "reads memory-tree state"),
    "mtree_store": ("Sink", "Internal", "Reversible", "persists to the memory tree"),
    "mtree_stats": ("Source", "Internal", "Pure", "reads memory-tree state"),
    "mtree_forget": ("Sink", "Internal", "Irreversible", "destructively forgets memory-tree entries"),
    # ── cron ──
    "cron_add": ("Sink", "Internal", "Reversible", "persists a schedule entry (undoable by cron_remove)"),
    "cron_remove": ("Sink", "Internal", "Irreversible", "removes a schedule entry — destructive"),
    "cron_list": ("Source", "Internal", "Pure", "reads schedule state"),
    "cron_mark_fired": ("Sink", "Internal", "Reversible", "mutates schedule firing state"),
    "cron_run": ("Sink", "Internal", "Irreversible", "fires scheduled flows — downstream external effects"),
    # ── recipe / vault ──
    "recipe_save": ("Sink", "Internal", "Reversible", "persists a recipe"),
    "recipe_search": ("Source", "Internal", "Pure", "reads recipe state"),
    "recipe_list": ("Source", "Internal", "Pure", "reads recipe state"),
    "semantic_search": ("Source", "Internal", "Pure", "reads the semantic vault (KNN state input)"),
    "config_load": ("Source", "Internal", "Pure", "ingests a config file from disk"),
    "vault_validate": ("Pure", "Public", "Pure", "deterministic validation of a loaded config"),
    # ── bot (assistant persistence / human channels) ──
    "send_message": ("Sink", "Network", "Irreversible", "delivers a message to an external chat — cannot be unsent (issue minimum list)"),
    "send_document": ("Sink", "Network", "Irreversible", "delivers a document externally — cannot be unsent"),
    "edit_message_text": ("Sink", "Network", "Reversible", "edits an already-delivered external message (reversible by further edits)"),
    "answer_callback_query": ("Sink", "Network", "Irreversible", "answers an external callback query"),
    "whisper_transcribe": ("Source", "Network", "Pure", "ingests external transcription of user audio; DUAL: uploads the audio to an external STT provider (№317 corpus)"),
    "tts_generate": ("Source", "Network", "Pure", "ingests an audio artifact from an external TTS provider; DUAL: transmits the text to the provider (№317 corpus)"),
    "tts_send": ("Sink", "Network", "Irreversible", "synthesizes AND delivers audio externally — cannot be unsent (issue minimum list)"),
    "todo_add": ("Sink", "Internal", "Reversible", "persists a todo entry"),
    "todo_list": ("Source", "Internal", "Pure", "reads todo state"),
    "todo_update": ("Sink", "Internal", "Reversible", "updates todo state — undoable"),
    "goal_get": ("Source", "Internal", "Pure", "reads goal state"),
    "goal_set": ("Sink", "Internal", "Reversible", "persists goal state"),
    "goals_add": ("Sink", "Internal", "Reversible", "persists goal state"),
    "goals_list": ("Source", "Internal", "Pure", "reads goal state"),
    "goal_complete": ("Sink", "Internal", "Reversible", "updates goal state"),
    "goals_reflect": ("Sink", "Internal", "Reversible", "updates goal state"),
    "remind": ("Sink", "Internal", "Reversible", "schedules a future external delivery (the reminder itself is undoable)"),
    "remind_recurring": ("Sink", "Internal", "Reversible", "schedules recurring future deliveries"),
    "cancel_remind": ("Sink", "Internal", "Reversible", "cancels a scheduled reminder"),
    "check_reminders": ("Source", "Internal", "Pure", "reads reminder state"),
    "list_reminders": ("Source", "Internal", "Pure", "reads reminder state"),
    "get_profile": ("Source", "Internal", "Pure", "reads persisted profile (PII state input)"),
    "human_mood": ("Source", "Internal", "Pure", "reads the persisted human-state model"),
    "ask_approval": ("Sink", "Network", "Irreversible", "sends an approval request to the human — external interaction"),
    "human_create": ("Sink", "Internal", "Reversible", "persists a human profile (PII)"),
    "human_delete": ("Sink", "Internal", "Irreversible", "destroys a human profile — no undo"),
    "human_forget": ("Sink", "Internal", "Irreversible", "destructively forgets human data (GDPR erasure semantics) — no undo"),
    "human_personas": ("Source", "Internal", "Pure", "reads persisted personas"),
    "human_recall": ("Source", "Internal", "Pure", "reads persisted human data"),
    "human_remember": ("Sink", "Internal", "Reversible", "persists human data (PII)"),
    "human_respond": ("Sink", "Network", "Irreversible", "delivers a response to the human — cannot be unsent"),
    "learn_preference": ("Sink", "Internal", "Reversible", "persists a learned preference (PII)"),
    "read_file_tokens": ("Source", "Internal", "Pure", "ingests file content (token-budgeted)"),
    "extract_entities": ("Pure", "Public", "Pure", "deterministic NER on in-program text"),
    "extract_param": ("Pure", "Public", "Pure", "deterministic parameter extraction"),
    "estimate_tokens": ("Pure", "Public", "Pure", "deterministic token estimate"),
    "compress_html": ("Pure", "Public", "Pure", "deterministic HTML compression"),
    "memory_score": ("Pure", "Public", "Pure", "deterministic scoring of passed values"),
    "fuzzy_find_best": ("Pure", "Public", "Pure", "deterministic fuzzy matching"),
    "squeeze": ("Pure", "Public", "Pure", "deterministic string compression"),
    # ── time ──
    "now": ("Source", "Public", "Pure", "wall-clock read — external (nondeterministic) input"),
    "time": ("Source", "Public", "Pure", "wall-clock read — external (nondeterministic) input"),
    "sleep": ("Sink", "Internal", "Irreversible", "temporal effect — suspends execution (no data flow)"),
    # ── reflex ──
    "reflex_train": ("Sink", "Internal", "Reversible", "persists trained weights in the Reflex registry (ADR-0114)"),
    "reflex_save": ("Sink", "Internal", "Reversible", "persists weights to SQLite (ADR-0116)"),
    "reflex_load": ("Source", "Internal", "Pure", "ingests persisted weights from SQLite"),
    "reflex_list": ("Source", "Internal", "Pure", "reads the Reflex registry state"),
    "reflex_predict": ("Pure", "Public", "Pure", "local model compute on passed values"),
    "reflex_metrics": ("Pure", "Public", "Pure", "local compute on a passed model handle"),
    "reflex_generate": ("Source", "Internal", "Pure", "ingests untrusted model output (LlmOutput-equivalent per №201)"),
    "reflex_bpe_save": ("Sink", "Internal", "Reversible", "persists the BPE vocab (№195)"),
    "reflex_bpe_load": ("Source", "Internal", "Pure", "ingests a persisted BPE vocab"),
    # ── vision (classification by intent; the current loud No-Go state is separate) ──
    "vision_generate": ("Sink", "Internal", "Reversible", "persists a generated artifact in the VisionRegistry (№210) — local compute, egress only at vision_export"),
    "vision_edit": ("Sink", "Internal", "Reversible", "persists an edited artifact in the VisionRegistry"),
    "vision_lora_generate": ("Sink", "Internal", "Reversible", "persists a LoRA-generated artifact in the VisionRegistry"),
    "vision_export": ("Sink", "Internal", "Reversible", "writes the signed image artifact to disk — egress point (gate VISION_UNSIGNED_EXPORT, ADR-0125)"),
    "vision_export_raw": ("Sink", "Internal", "Reversible", "explicit unsigned opt-out (ADR-0125); №320/ADR-0152: raw egress of synthetic or manifest-less artifacts is REFUSED — EU AI Act Art. 50 marking (static gate MEDIA_SYNTHETIC_UNMARKED + runtime backstop); legal only for synthetic: false"),
    "vision_fetch_weights": ("Source", "Network", "Reversible", "ingests external weights (allowlist+SSRF+SHA-pinned, №300); writes the local weight cache"),
    "vision_list": ("Source", "Internal", "Pure", "reads the VisionRegistry state"),
    "vision_save": ("Sink", "Internal", "Reversible", "persists a Vision artifact (№242)"),
    "vision_load": ("Source", "Internal", "Pure", "ingests a persisted Vision artifact"),
    "vision_lora_load": ("Source", "Internal", "Pure", "ingests a persisted LoRA adapter"),
    # ── voice ──
    "voice_enroll": ("Sink", "Secret", "Reversible", "persists a BIOMETRIC voiceprint (GDPR Art. 9 — Secret label, encrypted at rest per ADR-0145 D4)"),
    "tts_speak": ("Sink", "Internal", "Reversible", "persists a locally synthesized audio artifact (ADR-0143)"),
    "audio_export": ("Sink", "Internal", "Reversible", "writes the signed audio artifact to disk (gate AUDIO_UNSIGNED_EXPORT, ADR-0145)"),
    "voice_design": ("Sink", "Internal", "Reversible", "persists a designed voice artifact"),
    "voice_save": ("Sink", "Internal", "Reversible", "persists a Voice artifact"),
    "voice_load": ("Source", "Internal", "Pure", "ingests a persisted Voice artifact"),
    # ── registry (№333/№334/№335/№336 — Волна 2 backends/perception) ──
    "backend_list": ("Source", "Public", "Pure", "reads the static backend registry metadata (name/class/weights_id/pin/license — ADR-0163) — no weights bytes exist behind the entries"),
    "backend_select": ("Source", "Internal", "Pure", "backend try-chain over the №333 registry SSOT (№336, ADR-0165): picks the first available rung or returns Degraded(t) — a typed result, never a panic, never a silent mock; every attempt is an audit event"),
    "stt_transcribe": ("Source", "Internal", "Pure", "local STT backend call (№334, whisper-turbo canon): ingests the transcript into the flow; the audio stays local (no upload — unlike whisper_transcribe); real mode requires SHA-pinned weights (PARKED №294)"),
    "omni_ask": ("Source", "Internal", "Pure", "local omni backend call (№334, nemotron canon): ingests the model answer into the flow; no network egress; real mode requires SHA-pinned weights (PARKED №294)"),
    "vision_understand": ("Source", "Internal", "Pure", "local vision-understanding backend call (№334, molmoact2 canon): ingests the answer about an image into the flow; no upload, no egress; real mode requires SHA-pinned weights (PARKED №294)"),
    "consent_grant": ("Lift", "Public", "Pure", "records (subject, scope, TTL) in the consent ledger and passes the value through with the consent scope EXTENDED (semantic.rs label_source) — process-local bookkeeping, no egress"),
    "consent_revoke": ("Lift", "Public", "Pure", "records the revocation and returns the value under the QUARANTINE label — the flat cascade is lattice absorption (poison is absorbing, ADR-0154 §2.1); process-local bookkeeping"),
    "quarantine_write": ("Sink", "Internal", "Reversible", "THE quarantine sink — the only legal egress for poisoned values (№325 clearance exempts it); unconditional QUARANTINE_EGRESS audit event (№326 posture)"),
    "consent_ledger_export": ("Sink", "Internal", "Reversible", "dumps the consent ledger as JSON to a sandboxed path — FILE EGRESS with an audit event (grant/TTL/revoke records never leave the process silently)"),
    # ── media (№331, ADR-0162 / №332, ADR-0164) ──
    "media_store_image": ("Lift", "Internal", "Reversible", "wraps provided bytes into an opaque Image handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop"),
    "media_store_audio": ("Lift", "Internal", "Reversible", "wraps provided bytes into an opaque Audio handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop"),
    "media_store_video_frame": ("Lift", "Internal", "Reversible", "wraps provided bytes into an opaque VideoFrame handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop"),
    "media_store_video_segment": ("Lift", "Internal", "Reversible", "wraps provided bytes into an opaque VideoSegment handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop"),
    "media_save": ("Sink", "Internal", "Reversible", "the ONLY sanctioned materialization of media bytes — file egress through the io sandbox; №325 sink clearance (private-egress) + runtime backstop MEDIA_SEALED_EGRESS (ADR-0162 §2.5)"),
    "media_retain": ("Pure", "Public", "Pure", "refcount +1 on a media handle (ADR-0162 §2.4) — pure store bookkeeping, no byte movement"),
    "media_release": ("Pure", "Public", "Pure", "refcount −1 on a media handle; 0 evicts the entry (sealed bytes zeroized) — store bookkeeping, no external effect"),
    "media_meta": ("Source", "Public", "Pure", "reads media store METADATA only (kind/conf/refs/sealed/origin) — no bytes leave the store"),
    "media_source_capture": ("Source", "Internal", "Pure", "HandleSource runtime (№332/ADR-0164): captures a handle from a DECLARED origin (file-backed through the io sandbox; camera is a loud PARKED boundary) — the handle label is the origin's declared conf"),
    "media_bind_origin": ("Pure", "Public", "Pure", "ProvBind runtime (№332/ADR-0164): binds an entry's origin and joins the declared conf into the entry label (re-seals when public becomes non-public) — store bookkeeping, no byte movement"),
    "media_manifest": ("Source", "Public", "Pure", "reads the entry-level manifest facts (kind/origin/conf/synthetic/bytes_sha256 — ADR-0166 §2.4) WITHOUT materializing bytes — store metadata, no egress"),
    "media_manifest_read": ("Source", "Internal", "Pure", "ingests a provenance sidecar (<path>.manifest.json) from the sandbox (ADR-0166 §2.4): manifest content enters the flow; missing/empty/corrupt sidecars are loud refusals (№320 posture), synthetic reads conservatively true"),
    # ── video (№309, ADR-0151) ──
    "video_render": ("Sink", "Internal", "Reversible", "persists a generated video artifact in VIDEO_REGISTRY — local tiny pipeline; egress only at video_export"),
    "frame_interp": ("Sink", "Internal", "Reversible", "persists an interpolated artifact in VIDEO_REGISTRY (ADR-0151 D2)"),
    "video_extend": ("Sink", "Internal", "Reversible", "persists an extended artifact in VIDEO_REGISTRY (ADR-0151 D3)"),
    "av_mux": ("Sink", "Internal", "Reversible", "persists the A/V sidecar container in VIDEO_REGISTRY (ADR-0151 D4)"),
    "video_export": ("Sink", "Internal", "Reversible", "writes the signed .mlgv container to disk — egress point (gate VIDEO_UNSIGNED_EXPORT, ADR-0151 D5)"),
    "video_fetch_weights": ("Source", "Network", "Reversible", "intended external weights fetch (formal No-Go №294 class, ADR-0151 D7); covered by MODEL_WEIGHTS_UNSAFE"),
    # ── email ──
    "smtp_send": ("Sink", "Network", "Irreversible", "sends an email externally — cannot be unsent"),
    "smtp_send_html": ("Sink", "Network", "Irreversible", "sends an HTML email externally — cannot be unsent"),
    "imap_list": ("Source", "Network", "Pure", "ingests external mailbox listing"),
    "imap_read": ("Source", "Network", "Pure", "ingests external email content"),
    "imap_search": ("Source", "Network", "Pure", "ingests external mailbox search results"),
    "imap_mark_read": ("Sink", "Network", "Reversible", "mutates external mailbox flags (undoable by flag change)"),
    "imap_move": ("Sink", "Network", "Reversible", "moves an external email between folders (undoable by moving back)"),
    # ── calendar ──
    "cal_connect": ("Source", "Network", "Pure", "ingests external calendar connection state"),
    "cal_list": ("Source", "Network", "Pure", "ingests external calendar listings"),
    "cal_events": ("Source", "Network", "Pure", "ingests external calendar events"),
    "cal_read": ("Source", "Network", "Pure", "ingests an external calendar event"),
    "cal_create": ("Sink", "Network", "Irreversible", "creates an external calendar event — external state change"),
    "cal_update": ("Sink", "Network", "Reversible", "updates an external calendar event (undoable by update)"),
    "cal_delete": ("Sink", "Network", "Irreversible", "deletes an external calendar event — external state change"),
    "cal_freebusy": ("Source", "Network", "Pure", "ingests external free/busy data"),
    "ical_parse": ("Pure", "Public", "Pure", "deterministic iCalendar parsing of in-program text"),
    "ical_generate": ("Pure", "Public", "Pure", "deterministic iCalendar generation"),
    # ── contacts ──
    "card_connect": ("Source", "Network", "Pure", "ingests external CardDAV connection state"),
    "card_list": ("Source", "Network", "Pure", "ingests external address-book listings"),
    "card_contacts": ("Source", "Network", "Pure", "ingests external contacts (PII ingress)"),
    "card_read": ("Source", "Network", "Pure", "ingests an external contact (PII ingress)"),
    "card_create": ("Sink", "Network", "Irreversible", "creates an external contact — external state change"),
    "card_update": ("Sink", "Network", "Reversible", "updates an external contact (undoable by update)"),
    "card_delete": ("Sink", "Network", "Irreversible", "deletes an external contact — external state change"),
    "card_search": ("Source", "Network", "Pure", "ingests external contact search results"),
    "vcard_parse": ("Pure", "Public", "Pure", "deterministic vCard parsing of in-program text"),
    "vcard_generate": ("Pure", "Public", "Pure", "deterministic vCard generation"),
    # ── reflex (pure local compute) ──
    "reflex_tokenize": ("Pure", "Public", "Pure", "local BPE tokenization of in-program text"),
    "reflex_detokenize": ("Pure", "Public", "Pure", "local BPE detokenization"),
    "reflex_bpe_train": ("Pure", "Public", "Pure", "local BPE training on passed corpus (deterministic, №195)"),
    "reflex_bpe_encode": ("Pure", "Public", "Pure", "local BPE encoding"),
    "reflex_bpe_decode": ("Pure", "Public", "Pure", "local BPE decoding"),
    # ── security ──
    "canary_insert": ("Sink", "Internal", "Reversible", "plants canary markers into channels — security-instrumentation state write (№284)"),
    "canary_check": ("Source", "Internal", "Pure", "reads canary leak-detection state (№284)"),
    # ── stubs (classified by intent; VM-native per gen_reference MANUAL notes) ──
    "newline": ("Pure", "Public", "Pure", "produces a newline string (registry-only stub)"),
    "stdin": ("Source", "Internal", "Pure", "intended external stdin ingress (registry-only stub)"),
    "split_tokens": ("Pure", "Public", "Pure", "deterministic tokenization (registry-only stub)"),
    "if_eq": ("Pure", "Public", "Pure", "comparison helper (registry-only stub; `if` is an expression)"),
    "is_string_token": ("Pure", "Public", "Pure", "deterministic token check (registry-only stub)"),
    "recall": ("Source", "Internal", "Pure", "intended memory recall — state read"),
    "forget": ("Sink", "Internal", "Irreversible", "intended destructive memory removal"),
    "find": ("Source", "Internal", "Pure", "intended memory search — state read"),
    "inspect": ("Source", "Internal", "Pure", "intended runtime introspection — state read"),
    # №392: the DenyEvent surface — a read of the live runtime deny event,
    # available only inside an on_deny handler (the analyzer enforces the
    # scope at compile time, the backends at runtime).
    "deny_event": ("Source", "Internal", "Pure", "№392 DenyEvent read — handler-scoped runtime state, no egress"),
    "deny_reason": ("Source", "Internal", "Pure", "№392 deny reason word — handler-scoped runtime state, no egress"),
    "conv_start": ("Sink", "Internal", "Reversible", "intended conversation state creation"),
    "conv_add": ("Sink", "Internal", "Reversible", "intended conversation state append"),
    "conv_history": ("Source", "Internal", "Pure", "intended conversation state read"),
    "conv_context": ("Source", "Internal", "Pure", "intended conversation state read"),
    "conv_end": ("Sink", "Internal", "Reversible", "intended conversation state close"),
    "resolve_skill_index": ("Source", "Internal", "Pure", "VM-native skill resolver — state read"),
    "fit_to_budget": ("Pure", "Public", "Pure", "VM-native budget trimming — pure compute"),
    "map": ("Pure", "Public", "Pure", "intended mapping over values — pure compute"),
    # ── test / fluid / orchestration ──
    "assert_eq": ("Pure", "Public", "Pure", "assertion — control effect on failure, no data flow"),
    "assert_contains": ("Pure", "Public", "Pure", "assertion — control effect on failure, no data flow"),
    "confidence": ("Pure", "Public", "Pure", "fluid confidence wrapper — pure compute"),
    "budget_check": ("Pure", "Public", "Pure", "fluid budget check — pure compute"),
    "dag_phases": ("Pure", "Public", "Pure", "deterministic DAG phase computation"),
    "topo_sort": ("Pure", "Public", "Pure", "deterministic topological sort"),
}

# Categories where a silent Pure default is FORBIDDEN.
RISKY_CATEGORIES = {
    "io", "web", "db", "system", "email", "calendar", "contacts", "crypto",
    "llm", "voice", "vision", "video", "vault", "recipe", "cron", "mtree",
    "security", "memory", "bot", "reflex", "orchestration", "graph",
    "fluid", "stub", "test", "media", "registry",
}
# №337: "media" and "registry" join the risky set — their manual rows
# (№331/№333–№337) carry real rationales the Pure default would destroy
# on regeneration (found and prevented during №336/№337).


def parse_registry():
    entries = []
    seen = set()
    for line in REGISTRY.read_text(encoding="utf-8").splitlines():
        code = line.split("//")[0]
        m = re.search(r'spec!\(\s*"([^"]+)"\s*,\s*(\d+)\s*(?:,\s*(\d+))?\s*,\s*"([^"]+)"', code)
        if not m:
            continue
        name, cat = m.group(1), m.group(4)
        if name in seen:
            continue
        seen.add(name)
        entries.append((name, cat))
    return entries


def main():
    entries = parse_registry()
    print(f"registry names: {len(entries)}")

    rows = []
    missing_override = []
    for name, cat in entries:
        if name in OVERRIDES:
            role, label, rev, rationale = OVERRIDES[name]
        else:
            if cat in RISKY_CATEGORIES:
                missing_override.append((name, cat))
                continue
            role, label, rev, rationale = "Pure", "Public", "Pure", ""
        rows.append((name, cat, role, label, rev, rationale))

    if missing_override:
        print("ERROR: risky-category builtins without explicit classification:")
        for name, cat in missing_override:
            print(f"  {name} ({cat})")
        sys.exit(1)

    unknown = set(OVERRIDES) - {n for n, _ in entries}
    if unknown:
        print(f"ERROR: overrides for names not in registry: {sorted(unknown)}")
        sys.exit(1)

    non_pure = [(n, r, rat) for n, _, r, _, _, rat in rows if r != "Pure"]
    empty_rat = [(n, r) for n, r, rat in non_pure if not rat.strip()]
    if empty_rat:
        print(f"ERROR: non-Pure without rationale: {empty_rat}")
        sys.exit(1)

    entry_lines = []
    for name, cat, role, label, rev, rationale in rows:
        if role == "Pure":
            entry_lines.append(
                f'    BuiltClassEntry {{ name: "{name}", class: BuiltClass {{ role: Role::Pure, '
                f'default_label: Label::{label}, reversibility: Reversibility::{rev}, rationale: "" }} }},'
            )
        else:
            lit = rationale.replace('\\', '\\\\').replace('"', '\\"')
            entry_lines.append(
                f'    BuiltClassEntry {{ name: "{name}", class: BuiltClass {{ role: Role::{role}, '
                f'default_label: Label::{label}, reversibility: Reversibility::{rev}, rationale: "{lit}" }} }},'
            )
    rust = RUST_TEMPLATE.replace("{ENTRIES}", "\n".join(entry_lines))
    OUT_RS.write_text(rust, encoding="utf-8")
    print(f"wrote {OUT_RS} ({len(rows)} entries, {len(non_pure)} non-Pure)")

    # ── REFERENCE block: parsed back from the committed Rust map ──
    rs = OUT_RS.read_text(encoding="utf-8")
    entry_re = re.compile(
        r'BuiltClassEntry \{ name: "([^"]+)", class: BuiltClass \{ role: Role::(\w+), '
        r'default_label: Label::(\w+), reversibility: Reversibility::(\w+), rationale: "([^"]*)" \} \}'
    )
    md = ["| Builtin | Role | Default label | Reversibility |", "|---|---|---|---|"]
    for m in entry_re.finditer(rs):
        name, role, label, rev, rationale = m.groups()
        role_s = {"Pure": "pure", "Source": "source", "Lift": "lift", "Sink": "sink"}[role]
        label_s = {"Public": "public", "Internal": "internal", "Secret": "secret", "Network": "network"}[label]
        rev_s = {"Pure": "pure", "Reversible": "reversible", "Irreversible": "irreversible"}[rev]
        if rationale:
            md.append(f"| `{name}` | {role_s} | {label_s} | {rev_s} | {rationale} |")
        else:
            md.append(f"| `{name}` | {role_s} | {label_s} | {rev_s} | — |")
    block = BEGIN + "\n\n" + "\n".join(md) + "\n\n" + END + "\n"

    ref = REFERENCE.read_text(encoding="utf-8")
    if BEGIN in ref and END in ref:
        head, rest = ref.split(BEGIN, 1)
        _, tail = rest.split(END, 1)
        ref = head + block + tail
    else:
        ref = (
            ref.rstrip("\n")
            + "\n\n## 7. Builtin Classification — role × label × reversibility (№316)\n\n"
            + "> Generated from `src/builtins_classification.rs` (SSOT, №316) by `scripts/gen_classification.py`. "
            + "Role: pure / source / lift / sink; label: public / internal / secret / network; reversibility of the external effect: pure / reversible / irreversible.\n\n"
            + block
        )
    REFERENCE.write_text(ref, encoding="utf-8")
    print(f"updated {REFERENCE} classification block ({len(md) - 2} rows)")


if __name__ == "__main__":
    main()
