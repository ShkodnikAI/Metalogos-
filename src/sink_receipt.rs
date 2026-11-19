// ── Sink receipts (Наряд №964, issue #1949 — the B7 line) ──
// The leaf module: no dependencies by design (the architecture C4 ratchet —
// tests/architecture_contract.rs — freezes the module cycle inventory; the
// receipt table must not pull its consumers into a cycle).

use crate::builtins_classification::{classify, Role};

/// Наряд №964 (issue #1949, the B7 line): the sinks whose RETURN is a
/// RECEIPT — an id, a path, a count, a status — and therefore does NOT
/// inherit the labels of the payload the sink consumed.
///
/// THE FALSE POSITIVE THIS CLOSES, measured on this tree: a legal
/// quarantine write passes (`quarantine_write("q", env("SP"))` — the
/// quarantine is THE sanctioned egress for a poisoned value, №325
/// exempts it from the clearance gate), but the moment the program uses
/// the receipt the label of the payload comes back out of the sink and
/// `print(r)` is refused as `SECRET_LEAK`. A program that quarantines a
/// secret correctly and logs the quarantine id is doing the right thing
/// and gets a refusal for it; `w1_consent_revoke` is that program.
///
/// WHY THIS IS A NAME TABLE AND NOT A RULE OVER RETURN TYPES. A blanket
/// «a sink's scalar return is bottom» would be a LAUNDERING path for any
/// sink that echoes its argument: `print(quarantine_write("q", secret))`
/// would pass while printing the secret, because the quarantine call is
/// legal and the echo would arrive at `print` unlabelled. The clearance
/// a payload has for ONE channel is not clearance for another, so the
/// receipt claim has to be made per name, with the reason, and checked
/// against the registry (`tests/naryad_964_sink_receipt.rs`: every name
/// here must be `Role::Sink` with a scalar declared return — a typo
/// would otherwise disable the rule silently, the №951 lesson).
///
/// The reason per name, each VERIFIED against the actual return read in
/// the bounds of the handler body (№977, issue #1986 — the finding B of
/// the consolidated audit 18.11: the old wording claimed «the key» for
/// three builtins that in fact return THE VALUE; the method is reading
/// the whole function body, never a fixed-size window, which bleeds into
/// the next function):
///   `quarantine_write` — returns the audit text `«[QUARANTINE_EGRESS]
///       … (reason: …)»`, which EMBEDS args[1] (the caller-supplied
///       reason) — `src/builtins/consent.rs`, the `builtin_quarantine_write`
///       body; the old «entry id» wording was false. The receipt claim
///       for this name rests on the №335 POLICY (the quarantine is THE
///       sanctioned egress with an audit event): the PAYLOAD (args[0])
///       must not ride the return. The ECHO is a different fact — a value
///       passed AS THE REASON laundered out as bottom (the №977
///       escalation) — and №980 (issue #1994) resolves it with the exact
///       class: the return carries the reason's label via
///       `RECEIPT_ECHO_ARGS` (`from=1`) below; the hand names its own
///       removal condition (№983);
///   `write_file`, `append_file` — return the constant status `"ok"` on
///       success and `""` on soft-failure (`src/builtins/io.rs`, the
///       handler bodies) — NOT the path the old wording claimed; a
///       constant carries no payload, so the claim holds;
///   `delete_file` — the same constant `"ok"` / `""` status
///       (`src/builtins/io.rs`);
///   `db_insert` — returns `Float(last_insert_rowid)` on the real lane
///       (`src/db_ops.rs::db_insert`, dispatched ahead of the registry on
///       the TW `src/interpreter/execution.rs` and the VM `src/vm.rs`),
///       while the registered fallback (`src/builtins/server.rs`, the
///       №820 no-db contract) only ever errs — a store-allocated rowid
///       echoes no argument (the old «affected-row count» wording was
///       wrong about the value);
///   `approval_revoke` — returns the count of revoked mandates (`Float`,
///       `src/builtins/approval.rs`);
///   `kv_set`, `kv_delete` — return `Unit` (`src/builtins/memory.rs`);
///       `memorize` returns `Unit` too, sharing the `builtin_kv_set`
///       handler (`src/builtins/registry.rs`) — a Unit cannot carry a
///       payload, so the claim holds whatever the old «returns the key»
///       wording said;
///   `cal_create`, `card_create` — return the EXTERNAL object uid
///       (`src/builtins/calendar.rs`, `src/builtins/contacts.rs`);
///   `cal_update`, `cal_delete` — return the constant `"ok"` status;
///       `card_update`, `card_delete` — the constant `{"ok":true}`
///       status (the handler bodies);
///   `remind`, `remind_recurring` — return the schedule id;
///       `cancel_remind` — the constant `"ok"` / `"not_found"` status
///       (`src/builtins/cron.rs`).
///
/// REMOVED BY №977 (they were in this table until the finding B):
/// `mem_set` returned args[1] — THE VALUE stored, `session_set` returned
/// args[2] — THE VALUE stored, `mem_delete` returned the value removed
/// FROM THE STORE (`src/builtins/memory.rs`, the handler bodies; the
/// store content is statically unknowable, so the echo is the only
/// sound approximation). Each is an echo sink — exactly the laundering
/// path the blanket-rule warning below describes — and on a non-maskable
/// probe (`query_param`: untrusted, not a secret) the `HTML_INJECTION`
/// refusal that fires on the bare control vanished on all three paths
/// (`mem_delete(k) → respond` did not refuse at all). The names are out;
/// the argument join covers their returns again
/// (`tests/naryad_977_echo_receipts.rs` pins the control and all three
/// paths).
///
/// NOT in this table, deliberately: `print`, `http_post`, `respond`,
/// `smtp_send`, `exec`, `git_push`, `tts_send`, `human_respond` and the
/// other delivery sinks. Their returns may be status text that carries
/// the request, nothing in the registry says otherwise, and the
/// conservative join costs those programs nothing — a delivery sink's
/// receipt is rarely read.
pub const SINK_RECEIPT_NAMES: &[&str] = &[
    "quarantine_write",
    "write_file",
    "append_file",
    "delete_file",
    "db_insert",
    "approval_revoke",
    "kv_set",
    "kv_delete",
    "memorize",
    "cal_create",
    "cal_update",
    "cal_delete",
    "card_create",
    "card_update",
    "card_delete",
    "remind",
    "remind_recurring",
    "cancel_remind",
];

/// `true` when the call's value is a sink RECEIPT and therefore carries
/// none of the payload's labels. Consulted by BOTH label lanes — the
/// lattice (`semantic::expr_label`) and the typed secret lane
/// (`secret_label::label_of_expr`) — so the two cannot disagree about
/// what a receipt is.
///
/// Two independent grounds, both checked against the live registry:
///   * the name stands in `SINK_RECEIPT_NAMES` (the reasoned table), or
///   * the declared return type is `Unit` or `Bool` — a value of that
///     type cannot carry a payload at all, so the receipt claim needs no
///     per-name argument. `Float` and `String` are NOT in this by-type
///     lane: a numeric or textual return can echo its argument, and the
///     two `Float` receipts that matter are named in the table instead.
///
/// A name that is not classified `Role::Sink` answers `false` even if it
/// stands in the table — the table claims a fact about SINKS, and a
/// reclassified builtin must lose the exemption rather than keep it.
pub fn is_sink_receipt(fn_name: &str) -> bool {
    if !matches!(classify(fn_name).map(|c| c.role), Some(Role::Sink)) {
        return false;
    }
    if SINK_RECEIPT_NAMES.contains(&fn_name) {
        return true;
    }
    crate::builtins::registry::BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == fn_name)
        .is_some_and(|s| matches!(s.return_type_raw, "Unit" | "Bool"))
}

/// №978 (issue #1987, the consolidated audit 18.11 — finding C): the
/// sinks whose RETURN is EXTERNAL DATA — a remote response body, a host
/// command's stdout, a human's typed answer — and therefore arrives
/// `public, untrusted`, not as the join of the call's arguments. The
/// audit's probe: `http_get(url) → respond` refuses (`http_get` is a
/// `Role::Source`, the №951 class), while `http_post(url, …) → respond`
/// and `exec("ls") → respond` PASSED — the same remote body trusted or
/// untrusted depending on which ROLE the builtin answers, the №316
/// one-direction-per-builtin mechanism.
///
/// THE REASON PER NAME, each verified against the actual return read in
/// the bounds of the handler body (the №977 method):
///   `exec`, `exec_argv` — return the host command's STDOUT
///       (`src/builtins/io.rs`, `String::from_utf8_lossy(&output.stdout)`);
///   `git_push` — returns the git command's stdout too (`src/builtins/io.rs`)
///       — the completeness pass added it to the audit's six (the audit's
///       probe table never exercised it; the class is identical);
///   `http_post`, `http_post_multipart` — return the HTTP RESPONSE BODY
///       (`src/builtins/http.rs`, `Ok(Value::String(resp_body))`);
///   `tts_send` — returns the remote Telegram API response body
///       (`src/builtins/llm.rs`, `Ok(Value::String(tg_body))`);
///   `send_message`, `send_photo`, `send_document`,
///       `answer_callback_query`, `edit_message_text` — return the remote
///       messaging-API response body (`src/builtins/server.rs`,
///       `src/builtins/pdf.rs` — `resp_body` / `last_body` / `resp`) — the
///       completeness pass added the messaging family to the audit's six;
///   `human_respond` — returns the HUMAN'S ANSWER (`src/builtins/office/
///       human.rs`, `Ok(Value::String(response))`) — untrusted user input,
///       not the program's own data.
///
/// Reviewed and deliberately NOT in this table (the completeness pass
/// over every `Role::Sink` classification row, 120 names):
///   the email family — `smtp_send`, `smtp_send_html`, `imap_mark_read`,
///       `imap_move` return the constant typed Struct `…Result{ok: true}`
///       (`src/builtins/email.rs`) — a declared-constant field, not remote
///       content;
///   the media/model generation family (`vision_generate`, `video_render`,
///       `audio_export`, …) returns GENERATED artifacts — model output,
///       not ingested remote data (a different boundary, owned by the
///       flow-edge and runtime gates, not by this class);
///   the store-content return class (`memory_export` and kin return the
///       STORE's content through a sink return) is recorded as the №978-a
///       residual in `docs/limitations.md` — whether it carries the store's
///       channel class is the owner's call, not this naryad's.
///
/// The rule lives HERE, one table, and is consulted by BOTH label lanes —
/// the lattice (`semantic::expr_label`) and the typed secret lane
/// (`secret_label::label_of_expr`) — exactly like the receipts above, so
/// the two lanes cannot disagree about what an external return is. The
/// ROLE predicate is deliberately NOT the mechanism (it is the source of
/// the №316 asymmetry): the claim is made per name, with a verified
/// reason, and a reclassified builtin must lose the marking rather than
/// keep it.
pub const EXTERNAL_RETURN_NAMES: &[&str] = &[
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

/// `true` when the call's value is a sink's EXTERNAL return — remote
/// response content, host stdout, or a human's answer — and therefore
/// carries `public, untrusted` in the lattice and the plain declared
/// base type in the typed secret lane (it is untrusted NOT-secret data;
/// a secret that rode an argument refuses at the CALL, in the argument
/// gates, before any return is read).
pub fn external_sink_return(fn_name: &str) -> bool {
    if !matches!(classify(fn_name).map(|c| c.role), Some(Role::Sink)) {
        return false;
    }
    EXTERNAL_RETURN_NAMES.contains(&fn_name)
}

/// №980 (issue #1994, the escalations line 19.11 — escalation 1): the
/// receipts whose return EMBEDS one caller-supplied argument — the return
/// is not a pure receipt (bottom) and not the payload join, but a
/// constant machinery wrapper around EXACTLY ONE argument. The honest
/// class is `from=<i>`: the return carries the label of that argument and
/// nothing else.
///
/// THE REASON PER NAME, read in the bounds of the handler body (the №977
/// method — the whole function, never a fixed-size window):
///   `quarantine_write` — returns the audit text `«[QUARANTINE_EGRESS]
///       poisoned value reached the quarantine sink (legal egress with
///       audit event, №335; reason: …)»` which interpolates args[1] (the
///       caller-supplied reason) — `src/builtins/consent.rs`,
///       `builtin_quarantine_write`. The name keeps its place in
///       `SINK_RECEIPT_NAMES`: the PAYLOAD (args[0]) must not ride the
///       return — quarantining a secret and logging the receipt stays
///       legal (№964/№325, `w1_consent_revoke`). But the echo means a
///       value passed AS THE REASON arrived at every display sink as
///       `Label::bottom()` — the laundering path the №977 escalation
///       named — closed here by the exact class instead of the coarse
///       two (removing the name re-breaks the motivating false positive;
///       joining the payload back poisons it).
///
/// REMOVAL CONDITION (№983): when the product fix lands — the return
/// becomes the machine quarantine id, the reason moves into the №335
/// audit event — this hand is removed BY THAT naryad's PR and the name
/// re-enters the pure receipt claim. Keeping `from=1` past the fix would
/// make the rule stricter than the code — the §1 drift, the reverse
/// direction. The pins in `tests/naryad_980_receipt_echo.rs` name this
/// condition.
///
/// Consulted by FOUR lanes, in the same order everywhere — after the
/// private-field bridge, before `is_sink_receipt`: the static lattice
/// (`semantic::expr_label`), the TW runtime (`interpreter/labels.rs`),
/// the VM runtime (`vm.rs::builtin_result_label`) and the typed secret
/// lane (`secret_label::label_of_expr`). One table, no island (§2): the
/// same consult order is the same verdict.
pub const RECEIPT_ECHO_ARGS: &[(&str, usize)] = &[("quarantine_write", 1)];

/// The index of the argument the receipt ECHOES, or `None` when the
/// return is a pure receipt. Guarded by the live registry like
/// `is_sink_receipt`: a reclassified builtin must lose the echo rather
/// than keep it. An index beyond the call's arity answers `None` — a
/// 1-arg `quarantine_write(v)` interpolates the constant «unspecified»,
/// pure machinery, `bottom`.
pub fn receipt_echo_arg(fn_name: &str) -> Option<usize> {
    if !matches!(classify(fn_name).map(|c| c.role), Some(Role::Sink)) {
        return None;
    }
    RECEIPT_ECHO_ARGS
        .iter()
        .find(|(n, _)| *n == fn_name)
        .map(|(_, i)| *i)
}
