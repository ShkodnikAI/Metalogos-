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
/// The reason per name, from the registry's own rationale:
///   `quarantine_write` — returns the quarantine ENTRY ID (the case above);
///   `write_file`, `append_file` — return the PATH written, which the
///       caller already holds as a literal or a built path;
///   `delete_file` — returns the path destroyed;
///   `db_insert` — returns the affected-row count (`Float`);
///   `approval_revoke` — returns the count of revoked mandates (`Float`);
///   `kv_set`, `mem_set`, `memorize`, `session_set` — return the KEY the
///       value was stored under, never the value;
///   `kv_delete`, `mem_delete` — return the key destroyed;
///   `cal_create`/`cal_update`/`cal_delete`,
///   `card_create`/`card_update`/`card_delete` — return the EXTERNAL
///       object id;
///   `remind`, `remind_recurring`, `cancel_remind` — return the schedule id.
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
    "mem_set",
    "mem_delete",
    "memorize",
    "session_set",
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
