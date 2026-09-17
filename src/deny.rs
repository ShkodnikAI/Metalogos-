// ── Naryad #392 (P0, security/action): DenyEvent ─────────────────────
//
// The typed deny-event vocabulary shared by the static gate, the
// tree-walking interpreter and the bytecode VM. The reason words ARE the
// audit check_ids (`audit.rs::sink_check_id` is the SSOT that maps a
// sink call site to its reason class) — a deny event's `reason` always
// equals the diagnostic class the same verdict would print, so the
// static gate, the runtime twin and the event stream agree verbatim
// (the №328 agreement principle, extended from verdicts to events).
//
// Core classes (the №392 contract, the seven from audit.rs:3405-3422):
//   VOICE_EGRESS_UNCONSENTED  — voice egress without a consent scope;
//   IRREVERSIBLE_NO_GRANT     — destructive/irreversible action without
//                               a valid grant (grant refusals №390 fold
//                               here: the typed GRANT_* error is carried
//                               verbatim in the event's `human` field);
//   UNTRUSTED_EXEC_DECISION   — untrusted data drives exec/exec_argv;
//   SECRET_TO_EXEC            — a private label enters exec/exec_argv;
//   SECRET_EGRESS_VCS         — a private label enters git_push;
//   SECRET_EGRESS_NETWORK     — a private-URL marker in the address
//                               position of a network sink;
//   PII_EGRESS_NETWORK        — personal-data label in a network sink.
//
// Extending classes (same gate, same dictionary — audit.rs:3405-3422):
//   PII_EGRESS_OUTPUT         — personal-data label in a public output;
//   UNTRUSTED_EGRESS_NETWORK  — untrusted label in a network sink.
// Generic class:
//   SINK_CLEARANCE            — every other confidentiality excess.
// Legacy corpus classes emitted by the same gate (the leak-suite
// vocabulary keeps the historical names for output/memory/file sinks):
//   HTML_INJECTION, TAINT_PERSISTENCE, SECRET_LEAK.
//
// DenyEvent is a runtime-constructed `Value::Struct` (type_name
// "DenyEvent"); no language surface constructs it — the analyzer rejects
// `deny_event()`/`deny_reason()` outside an on_deny handler and the
// runtime refuses them when no event is live (double gate, mirroring the
// static+runtime posture of №325/№328).

use crate::interpreter::values::Value;
use std::collections::HashMap;

/// The canonical deny-reason vocabulary — exhaustive over every reason
/// the gates can emit. The seven core classes first, then the extending
/// pair, the generic class, then the legacy corpus classes.
pub const DENY_REASONS: [&str; 13] = [
    // ── the seven core classes (№392) ─────────────────────────────
    "VOICE_EGRESS_UNCONSENTED",
    "IRREVERSIBLE_NO_GRANT",
    "UNTRUSTED_EXEC_DECISION",
    "SECRET_TO_EXEC",
    "SECRET_EGRESS_VCS",
    "SECRET_EGRESS_NETWORK",
    "PII_EGRESS_NETWORK",
    // ── extending classes (same gate dictionary) ──────────────────
    "PII_EGRESS_OUTPUT",
    "UNTRUSTED_EGRESS_NETWORK",
    // ── the generic class ──────────────────────────────────────────
    "SINK_CLEARANCE",
    // ── legacy corpus classes (same gate, historical names) ───────
    "HTML_INJECTION",
    "TAINT_PERSISTENCE",
    "SECRET_LEAK",
];

/// The sink-class words the `on_deny(...)` selector accepts — the same
/// mapping `audit.rs::sink_kind` uses to classify sink builtins.
pub const SINK_CLASSES: [&str; 8] = [
    "voice", "exec", "vcs", "network", "output", "file", "memory", "db",
];

/// The wildcard class selector: every sink class at once.
pub const DENY_ALL: &str = "*";

/// True when `word` is a valid `on_deny(...)` class selector.
pub fn is_valid_class(word: &str) -> bool {
    word == DENY_ALL || SINK_CLASSES.contains(&word)
}

/// True when `word` is a known deny reason (the enum vocabulary).
pub fn is_known_reason(word: &str) -> bool {
    DENY_REASONS.contains(&word)
}

/// The deny reasons NOT covered by a match arm set — the analyzer prints
/// this list for a non-exhaustive match without an `else` arm (№392 §3).
pub fn uncovered_reasons(covered: &[String]) -> Vec<&'static str> {
    DENY_REASONS
        .iter()
        .copied()
        .filter(|r| !covered.iter().any(|c| c == r))
        .collect()
}

/// The argument bundle for firing an on_deny handler (№392). The fields
/// mirror the DenyEvent struct fields — grouped so the fire site stays
/// under the argument-count lint.
pub struct DenyEventArgs {
    pub reason: String,
    pub class: String,
    pub sink: String,
    pub argument: String,
    pub label: String,
    pub line: f64,
    pub human: String,
}

/// Build the runtime `DenyEvent` struct value. Fields (№392 §1):
/// `reason` (the enum word), `sink` (the sink builtin name), `class`
/// (the sink-class word the handler matched on), `argument` (the
/// refused argument, best-effort name), `label` (the runtime label that
/// failed clearance — "bottom" when the gate is content-driven),
/// `line` (source line, 0 when unknown), `human` (the full sentence a
/// human reads — carries the typed GRANT_* detail for grant refusals).
#[allow(clippy::too_many_arguments)]
pub fn make_event(
    reason: &str,
    sink: &str,
    class: &str,
    argument: &str,
    label: &str,
    line: f64,
    human: &str,
) -> Value {
    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("reason".to_string(), Value::String(reason.to_string()));
    fields.insert("sink".to_string(), Value::String(sink.to_string()));
    fields.insert("class".to_string(), Value::String(class.to_string()));
    fields.insert("argument".to_string(), Value::String(argument.to_string()));
    fields.insert("label".to_string(), Value::String(label.to_string()));
    fields.insert("line".to_string(), Value::Float(line));
    fields.insert("human".to_string(), Value::String(human.to_string()));
    Value::Struct {
        type_name: "DenyEvent".to_string(),
        fields,
    }
}

/// Most-specific handler selection (№392 §2): an exact sink-class match
/// wins over the wildcard; within the covered classes the LAST declared
/// handler wins (later declarations refine earlier ones — the same
/// resolution the rule engine uses). Returns the handler's position in
/// the handler list, or None when the refusal is unhandled (loud default
/// error — unchanged behavior).
pub fn select_handler<T>(handlers: &[(String, T)], class: &str) -> Option<usize> {
    let exact = handlers
        .iter()
        .rposition(|(c, _)| c == class)
        .or_else(|| handlers.iter().rposition(|(c, _)| c == DENY_ALL));
    let _ = handlers.len();
    exact
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_reason_vocabulary_is_exhaustive_over_unique_words() {
        // 13 distinct words, every one uppercase snake case (the audit
        // check_id convention).
        let mut seen = std::collections::HashSet::new();
        for r in DENY_REASONS {
            assert!(seen.insert(r), "duplicate deny reason: {r}");
            assert!(
                r.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "reason words are check_id-style: {r}"
            );
        }
        // The seven core classes of the №392 contract are present.
        for core in [
            "VOICE_EGRESS_UNCONSENTED",
            "IRREVERSIBLE_NO_GRANT",
            "UNTRUSTED_EXEC_DECISION",
            "SECRET_TO_EXEC",
            "SECRET_EGRESS_VCS",
            "SECRET_EGRESS_NETWORK",
            "PII_EGRESS_NETWORK",
        ] {
            assert!(is_known_reason(core), "core class missing: {core}");
        }
    }

    #[test]
    fn class_validation_accepts_sink_classes_and_wildcard() {
        for c in SINK_CLASSES {
            assert!(is_valid_class(c), "{c} must be a valid class");
        }
        assert!(is_valid_class("*"));
        assert!(!is_valid_class("everything"));
        assert!(!is_valid_class(""));
    }

    #[test]
    fn uncovered_reasons_lists_exactly_the_missing_set() {
        let covered = vec!["SINK_CLEARANCE".to_string()];
        let uncovered = uncovered_reasons(&covered);
        assert_eq!(uncovered.len(), DENY_REASONS.len() - 1);
        assert!(!uncovered.contains(&"SINK_CLEARANCE"));
        assert!(uncovered.contains(&"SECRET_TO_EXEC"));

        let all: Vec<String> = DENY_REASONS.iter().map(|s| s.to_string()).collect();
        assert!(uncovered_reasons(&all).is_empty());
    }

    #[test]
    fn select_handler_prefers_exact_class_over_wildcard() {
        let handlers = vec![("*".to_string(), 0usize), ("network".to_string(), 1usize)];
        assert_eq!(select_handler(&handlers, "network"), Some(1));
        assert_eq!(select_handler(&handlers, "db"), Some(0));
        assert_eq!(select_handler(&handlers, "voice"), Some(0));

        let none: Vec<(String, usize)> = vec![];
        assert_eq!(select_handler(&none, "db"), None);
    }

    #[test]
    fn make_event_carries_the_full_contract() {
        let ev = make_event(
            "IRREVERSIBLE_NO_GRANT",
            "db_execute_with_grant",
            "db",
            "sql",
            "bottom",
            7.0,
            "GRANT_EXHAUSTED: grant g1 is exhausted",
        );
        match &ev {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "DenyEvent");
                for key in [
                    "reason", "sink", "class", "argument", "label", "line", "human",
                ] {
                    assert!(fields.contains_key(key), "field {key} missing");
                }
            }
            other => panic!("expected a DenyEvent struct, got {other:?}"),
        }
    }
}
