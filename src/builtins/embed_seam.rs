// ── Naryad №546 (issue #884; ADR-0178 §5 preconditions 4–5) ─────────
//
// The embedding seam of the generative contour — the ONE runtime
// boundary where mlog values flow INTO the contour's model surface —
// carries two fail-closed guards, both loud in the №514/№531 style:
//
// 1. THE SECRET CHECK (precondition 4). The secret family
//    (`Value::Secret` / `Value::Encrypted` / `Value::Hash`) is
//    refused at the seam with a seam-named error. Before this naryad
//    the refusal was an ACCIDENT of the String-only argument contract
//    (the generic "must be a String" error); now it is a named,
//    tested guarantee — the seam rejects the secret family even if
//    the surface contract ever widens.
//
//    HONEST SCOPE NOTE: the runtime cannot see a compile-time №322
//    label on a plain `Value::String` — the static lane is the
//    label-checker's job (stage 1, warn-only). What the seam
//    guarantees at RUNTIME is exactly the value-level fact: the
//    opaque secret family never enters the embedding contour.
//    The content of a `Secret` cannot become a plain `String` except
//    through `redact()` (the masking path — the masked text is by
//    construction not the secret), so the value-level fact is the
//    whole runtime surface today.
//
// 2. THE CALL BUDGET (precondition 5). One budget unit = one embedding
//    operation at the seam. The counter is thread-local (the same
//    mechanism as the №457 `ServeRouteExecGuard`), reset by an RAII
//    scope at the request/tick entry points (the serve route bodies
//    and the cron tick executor — the places that already define the
//    per-request / per-tick boundary). Over the limit the seam fails
//    LOUD (`[CONTOUR_BUDGET_EXCEEDED]`), never silently degrades.
//    The limit is configurable (`METALOGOS_CONTOUR_BUDGET`, units per
//    scope); the default is deliberately conservative.
//
// Both guards live at the seam's three entry points: the `embed`
// builtin, the shared `embed_text` helper (the learnable semantic
// probe's SSOT, №272/№273) and the vault `semantic_search` builtin
// (its own manager instance, office/text.rs). `vec_store`/`vec_search`
// take pre-computed vectors and do NOT embed internally — no seam, no
// budget unit.
use crate::interpreter::Value;

pub const CONTOUR_BUDGET_ENV: &str = "METALOGOS_CONTOUR_BUDGET";
/// The conservative default: 64 embedding operations per request/tick.
/// A single route that needs more is a design smell the loud error
/// surfaces (the №514 posture: the default fails loudly, the operator
/// opts in consciously).
pub const CONTOUR_BUDGET_DEFAULT: u64 = 64;

pub const SECRET_SEAM_ERROR: &str = "[EMBED_SECRET_REJECTED]";
pub const BUDGET_SEAM_ERROR: &str = "[CONTOUR_BUDGET_EXCEEDED]";

thread_local! {
    static CONTOUR_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// №607: whether a budget scope is ACTIVE on this thread. The scope is
    /// installed at the request/tick/program-run boundaries; the seam
    /// consumers that charge per-call (`embed` et al. — the №546 surface)
    /// charge unconditionally, the PURE-MATH consumers (the spectral
    /// contour, №607) charge ONLY inside a scope — outside one there is no
    /// request boundary the budget could mean, and a long-lived thread
    /// (a REPL, a lib-level statistical sweep) must not accumulate charges
    /// forever.
    static CONTOUR_SCOPE_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// RAII scope for one request/tick: resets the contour-call counter at
/// the boundary entry. Drop does nothing (the counter dies with the
/// thread or is reset by the next scope — the reset-on-entry form is
/// the one that cannot forget to run: a scope that starts, counts).
/// №607: the scope also marks the BUDGETED CONTEXT (see
/// `seam_budget_check_scoped`) and clears the mark on drop — a nested
/// scope resets the counter and keeps the mark until the OUTERMOST
/// scope drops (the mark is per-thread, the nesting depth is tracked by
/// the counter of scopes itself).
pub struct ContourBudgetScope;

thread_local! {
    static CONTOUR_SCOPE_DEPTH: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

impl ContourBudgetScope {
    pub fn new() -> Self {
        CONTOUR_CALLS.with(|c| c.set(0));
        CONTOUR_SCOPE_DEPTH.with(|d| d.set(d.get() + 1));
        CONTOUR_SCOPE_ACTIVE.with(|a| a.set(true));
        ContourBudgetScope
    }
}

impl Default for ContourBudgetScope {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ContourBudgetScope {
    fn drop(&mut self) {
        CONTOUR_SCOPE_DEPTH.with(|d| {
            let depth = d.get().saturating_sub(1);
            d.set(depth);
            if depth == 0 {
                // The outermost scope dropped — the thread leaves the
                // budgeted context.
                CONTOUR_SCOPE_ACTIVE.with(|a| a.set(false));
            }
        });
    }
}

/// №607: is a budget scope active on this thread?
pub fn seam_scope_active() -> bool {
    CONTOUR_SCOPE_ACTIVE.with(|a| a.get())
}

/// №607: the scoped variant of [`seam_budget_check`] for the PURE-MATH
/// contours (the spectral contour). Inside a request/tick scope the
/// charge is identical to the seam's; OUTSIDE one there is no request
/// boundary the budget could mean — the charge is skipped (the call's
/// own N × M work budget still bounds it). Never refuses outside a
/// scope, never refuses silently inside one.
pub fn seam_budget_check_scoped(units: u64) -> Result<(), String> {
    if !seam_scope_active() {
        return Ok(());
    }
    seam_budget_check(units)
}

fn budget_limit() -> u64 {
    match std::env::var(CONTOUR_BUDGET_ENV) {
        Ok(v) => v.trim().parse::<u64>().unwrap_or(CONTOUR_BUDGET_DEFAULT),
        Err(_) => CONTOUR_BUDGET_DEFAULT,
    }
}

/// Consume `units` budget units at the seam. Loud refusal over the
/// limit — the caller propagates the error, the contour call does not
/// happen (fail-closed, never a silent degradation).
pub fn seam_budget_check(units: u64) -> Result<(), String> {
    CONTOUR_CALLS.with(|c| {
        let next = c.get() + units.max(1);
        let limit = budget_limit();
        if next > limit {
            Err(format!(
                "{} embedding-seam budget exceeded: {} units used at this \
                 scope, refusing +{} more (limit {} per request/tick; raise \
                 {} consciously or restructure the caller)",
                BUDGET_SEAM_ERROR,
                c.get(),
                units.max(1),
                limit,
                CONTOUR_BUDGET_ENV
            ))
        } else {
            c.set(next);
            Ok(())
        }
    })
}

/// №607: the scoped charge — identical to [`seam_budget_check`] inside a
/// request/tick scope; a no-op outside one (no boundary, no budget).
/// The runtime secret-family check at the seam (precondition 4). The
/// opaque secret family (`Value::Secret`/`Encrypted`/`Hash`) NEVER
/// enters the embedding contour — refuse loudly, name the seam.
pub fn seam_secret_check(v: &Value) -> Result<(), String> {
    match v {
        Value::Secret(_) | Value::Encrypted(_) | Value::Hash(_) => Err(format!(
            "{} a value of the secret family ({}) never enters the \
             embedding contour (ADR-0178 §5.4, fail-closed); mask it via \
             redact() first if the masked form is intended",
            SECRET_SEAM_ERROR,
            v.type_name()
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::values::SecretString;
    // The budget tests mutate the process-global env var — serial only
    // (a parallel reader would see the wrong limit; the №272 convention).
    use serial_test::serial;

    #[test]
    fn secret_family_is_refused_with_the_seam_error() {
        for v in [
            Value::Secret(SecretString::new("pw".into())),
            Value::Encrypted(vec![1, 2, 3]),
            Value::Hash("h".into()),
        ] {
            let err = seam_secret_check(&v).unwrap_err();
            assert!(err.starts_with(SECRET_SEAM_ERROR), "got: {err}");
            assert!(
                err.contains("redact()"),
                "the error must name the legal path"
            );
        }
        // The non-secret values pass the value-level check (the String
        // contract still applies at the callers).
        assert!(seam_secret_check(&Value::String("s".into())).is_ok());
        assert!(seam_secret_check(&Value::Unit).is_ok());
    }

    #[test]
    #[serial]
    fn budget_counts_and_refuses_loudly() {
        ContourBudgetScope::new();
        std::env::remove_var(CONTOUR_BUDGET_ENV);
        // The default 64: consume 63, then one more fits, one more refuses.
        for _ in 0..63 {
            seam_budget_check(1).unwrap();
        }
        seam_budget_check(1).unwrap();
        let err = seam_budget_check(1).unwrap_err();
        assert!(err.starts_with(BUDGET_SEAM_ERROR), "got: {err}");
        assert!(err.contains("limit 64"), "the error must name the limit");
        // A multi-unit call that would overshoot is refused WHOLE.
        ContourBudgetScope::new();
        seam_budget_check(10).unwrap();
        let err = seam_budget_check(100).unwrap_err();
        assert!(err.starts_with(BUDGET_SEAM_ERROR));
        // And the refused call did NOT partially consume.
        seam_budget_check(54).unwrap(); // 10+54 = 64 = the limit
    }

    #[test]
    #[serial]
    fn budget_limit_is_configurable() {
        ContourBudgetScope::new();
        std::env::set_var(CONTOUR_BUDGET_ENV, "2");
        seam_budget_check(1).unwrap();
        seam_budget_check(1).unwrap();
        assert!(seam_budget_check(1).is_err());
        std::env::remove_var(CONTOUR_BUDGET_ENV);
    }

    #[test]
    #[serial]
    fn a_new_scope_resets_the_counter() {
        std::env::remove_var(CONTOUR_BUDGET_ENV);
        ContourBudgetScope::new();
        for _ in 0..64 {
            seam_budget_check(1).unwrap();
        }
        assert!(seam_budget_check(1).is_err());
        ContourBudgetScope::new();
        seam_budget_check(1).unwrap();
    }
}
