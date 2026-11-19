// ── №978 (issue #1987, the consolidated audit 18.11 — finding C): the
//    external sink returns are `public, untrusted` ──────────────────────
//
// THE FACT: one and the same remote body was trusted or untrusted
// depending on which ROLE the builtin answers. `http_get` is classified
// `Role::Source` (the №951 class), so its answer carries
// `public, untrusted` and `http_get(url) → respond` refuses. The SINK
// half of the same boundary — `http_post`, `exec`, and the rest of the
// delivery family — returned its remote content as the join of the call
// arguments: bottom for literal arguments, so the SAME body reached
// `respond` trusted and the refusal vanished.
//
//   http_get(url)  → respond = HTML_INJECTION   (the control, preserved)
//   http_post(url, b) → respond = PASS  → now HTML_INJECTION
//   exec("ls") → respond = PASS          → now HTML_INJECTION
//   pattern returns exec stdout → respond = PASS → now HTML_INJECTION
//
// THE CONTROL SITS IN THIS FILE, next to the pins. THE MUTATION TARGET:
// remove any name from `sink_receipt::EXTERNAL_RETURN_NAMES` (or widen
// the role predicate to be the mechanism) and the corresponding pin
// reddens.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins_classification::{classify, Role};
use metalogos::sink_receipt::{external_sink_return, EXTERNAL_RETURN_NAMES};

/// Every refusal class of a program (PARSE_ERROR / PASS yield empty).
fn classes(src: &str) -> Vec<String> {
    match metalogos::check_program(src) {
        Err(_) => vec!["PARSE_ERROR".to_string()],
        Ok(r) if r.errors.is_empty() => vec![],
        Ok(r) => {
            let joined = r
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            let mut found = Vec::new();
            for line in joined.lines() {
                if let Some(s) = line.find('[') {
                    if let Some(e) = line[s + 1..].find(']') {
                        let id = &line[s + 1..s + 1 + e];
                        if !id.is_empty()
                            && id
                                .chars()
                                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                            && !found.iter().any(|f: &String| f == id)
                        {
                            found.push(id.to_string());
                        }
                    }
                }
            }
            found
        }
    }
}

fn prog(body: &str) -> String {
    format!(
        "pattern P(d: String) -> String {{ {body} return \"ok\" }}\n\
         flow Main {{ input: String = \"x\" -> P -> output }}\n"
    )
}

/// THE CONTROL — `http_get` is a Source (№951): its answer was ALWAYS
/// untrusted and the respond refusal always fired. №978 must not touch
/// it; this pin is the interpretability anchor for everything below.
#[test]
fn n978_the_http_get_control_is_preserved() {
    let v = classes(&prog("let v = http_get(\"http://x\") let _ = respond(v)"));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "the bare http_get → respond control must keep refusing with \
         HTML_INJECTION — got {v:?}"
    );
}

/// The SINK half of the same boundary: http_post's return is the remote
/// response body (`src/builtins/http.rs`, `Ok(Value::String(resp_body))`)
/// — it arrives untrusted and respond refuses, exactly like the control.
#[test]
fn n978_http_post_return_refuses() {
    let v = classes(&prog(
        "let v = http_post(\"http://x\", \"b\") let _ = respond(v)",
    ));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "http_post's return is the remote response body — the same remote \
         data http_get carries — respond must refuse with HTML_INJECTION \
         (it PASSED on the pre-№978 tree) — got {v:?}"
    );
}

/// exec's return is the host command's stdout — remote/host data, not
/// the program's own.
#[test]
fn n978_exec_return_refuses() {
    let v = classes(&prog("let v = exec(\"ls\") let _ = respond(v)"));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "exec's return is the host stdout — respond must refuse with \
         HTML_INJECTION (it PASSED on the pre-№978 tree) — got {v:?}"
    );
}

/// THE FLOW-EDGE FORM (the audit's probe): the stdout crosses a pattern
/// return and a flow edge before reaching the sink — the label must
/// survive the edge, the №967 display gate answers on the terminator.
#[test]
fn n978_the_flow_edge_form_refuses() {
    let src = "pattern Fetch() -> String { return exec(\"ls\") }\n\
               pattern P(d: String) -> String { let v = Fetch() let _ = respond(v) return \"ok\" }\n\
               flow Main { input: String = \"x\" -> P -> output }\n";
    let v = classes(src);
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "exec stdout through a pattern return and a flow edge must arrive \
         untrusted at respond (it PASSED on the pre-№978 tree) — got {v:?}"
    );
}

/// The completeness-pass addition: git_push returns the git command's
/// stdout (`src/builtins/io.rs`) — the same class as exec, which the
/// audit's probe table never exercised.
#[test]
fn n978_git_push_stdout_refuses() {
    let v = classes(&prog(
        "let v = git_push(\"origin\", \"main\") let _ = respond(v)",
    ));
    assert!(
        v.iter().any(|c| c == "HTML_INJECTION"),
        "git_push's return is the git stdout — the exec class — respond \
         must refuse with HTML_INJECTION — got {v:?}"
    );
}

/// The return rule does not mask the ARGUMENT gates: a secret riding
/// http_post's arguments refuses AT THE CALL (the delivery-sink gate),
/// exactly as before №978.
#[test]
fn n978_the_call_gates_still_answer_for_the_args() {
    let v = classes(&prog(
        "let v = http_post(\"http://x\", env(\"SP\")) let _ = print(v)",
    ));
    assert!(
        !v.is_empty(),
        "a secret in http_post's arguments must refuse at the CALL — the \
         return rule never masks the argument gates"
    );
    assert!(
        v.iter()
            .any(|c| c == "SECRET_LEAK" || c == "PII_EGRESS_NETWORK"),
        "the call-side class for a secret argument must survive — got {v:?}"
    );
}

/// The structural mutation guard: the table holds exactly the verified
/// names, each still a live `Role::Sink`, and the predicate agrees. The
/// ROLE is deliberately NOT the mechanism (the №316 asymmetry is the bug
/// this closes) — every claim is per name, with a verified reason.
#[test]
fn n978_the_external_table_is_exactly_the_verified_names() {
    let expected = [
        "exec",
        "exec_argv",
        "git_push",
        "http_post",
        "http_post_multipart",
        "tts_send",
        "send_message",
        "send_photo",
        "send_document",
        "answer_callback_query",
        "edit_message_text",
        "human_respond",
    ];
    assert_eq!(
        EXTERNAL_RETURN_NAMES.len(),
        expected.len(),
        "the external-return table drifted — every name needs a verified \
         body reason before it joins (the №964/№977 discipline)"
    );
    for name in expected {
        assert!(
            EXTERNAL_RETURN_NAMES.contains(&name),
            "`{name}` is a verified external return — it must stand in the \
             table; removing it re-opens the trust asymmetry"
        );
        let c = classify(name).expect("every external-return name is classified");
        assert!(
            matches!(c.role, Role::Sink),
            "`{name}` must be a live Role::Sink — the table claims a fact \
             about SINKS"
        );
        assert!(
            external_sink_return(name),
            "the predicate must agree with the table for `{name}`"
        );
    }
}

/// The reviewed-NOT-external class holds: the email family returns the
/// constant typed Struct (a declared-constant field, not remote content),
/// the media/model family returns GENERATED artifacts — neither rides
/// this rule; the completeness pass recorded both in the table of the
/// PR description.
#[test]
fn n978_the_reviewed_not_external_names_stay_out() {
    for name in [
        "smtp_send",
        "smtp_send_html",
        "imap_mark_read",
        "imap_move",
        "vision_generate",
        "video_render",
        "audio_export",
        "http_get",
        "respond",
        "print",
        "db_insert",
        "kv_set",
        "mem_set",
    ] {
        assert!(
            !external_sink_return(name),
            "`{name}` must NOT be an external return — reviewed and excluded \
             with a recorded reason; widening the table beyond its reasons \
             is the laundering shape in reverse"
        );
    }
}

/// Both label lanes agree: on the literal-argument probes the integrity
/// lane refuses (HTML_INJECTION) and the typed secret lane does NOT call
/// the remote response a secret (it is untrusted NOT-secret data; a
/// secret that rode an argument refused at the call — the pin above).
#[test]
fn n978_the_two_label_lanes_agree_on_the_probes() {
    for (label, body) in [
        (
            "http_post",
            "let v = http_post(\"http://x\", \"b\") let _ = respond(v)",
        ),
        ("exec", "let v = exec(\"ls\") let _ = respond(v)"),
        (
            "human_respond",
            "let v = human_respond(\"q\", \"ctx\") let _ = respond(v)",
        ),
    ] {
        let v = classes(&prog(body));
        assert!(
            v.iter().any(|c| c == "HTML_INJECTION"),
            "{label}: the integrity lane must refuse the external return — \
             got {v:?}"
        );
        assert!(
            !v.iter().any(|c| c == "SECRET_LEAK"),
            "{label}: the remote response is untrusted but NOT a secret — \
             the typed secret lane must not raise it (a SECRET_LEAK here is \
             a lane disagreement, №476) — got {v:?}"
        );
    }
}
