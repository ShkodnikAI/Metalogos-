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
///       sanctioned egress with an audit event), not on the mechanics of
///       the return, and the echo means a value passed AS THE REASON
///       launders out — kept in the table by scope (removing it re-breaks
///       the motivating false positive and is a design call over
///       №325/№335), escalated to the owner in the №977 report;
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
