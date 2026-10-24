// ── Naryad №782 (audit-23.10 A3) — the resource-limit SSOT ──────────────
//
//! ONE home for the executor resource limits. The old state had TWO local
//! copies of `MAX_STRING_LENGTH` (vm.rs and interpreter/execution.rs) and
//! NO recursion-depth cap at all (the native CallPattern → execute_code
//! recursion could overflow the thread stack — in serve the route bodies
//! run inside spawn_blocking with a 2 MiB stack, the overflow is an ABORT
//! of the whole serve process, not a catchable panic).
//!
//! - `MAX_STRING_LENGTH` — the string capacity: string-producing builtins
//!   with a multiplicative shape (repeat, pad_left, pad_right, ...) MUST
//!   check it BEFORE the allocation (`check_string_capacity`); the
//!   post-hoc concat checks of both executors read the same constant.
//! - `max_call_depth()` — the native-recursion cap shared by BOTH
//!   executors (VM `execute_code` entries; TW pattern invocations);
//!   `METALOGOS_VM_MAX_DEPTH` is the deployment knob (default 1024).
//!
//! Coverage boundary (docs/limitations.md): the cap covers the MLOG-LEVEL
//! recursion and the multiplicative string builtins — it does NOT cover
//! allocations inside opaque subsystems (svg/pdf/image generators); those
//! keep their own loud refusals.

/// The maximum length (bytes) of a single builtin-produced string.
/// The 1 MB figure is the historical TW/Vm concat limit — unchanged.
pub const MAX_STRING_LENGTH: usize = 1_000_000;

/// The default native-recursion depth cap (both executors). The lower
/// bound of the audit's 512–1024 range: in the debug build profile the
/// `execute_code` frames are large (the unoptimized instruction match),
/// so 512 is the default that reaches its LOUD refusal before the stack
/// exhaustion on every profile (release frames are far smaller; the
/// serve blocking pool runs 8 MiB stacks since №782 — see the mlog.rs
/// runtime builder).
pub const DEFAULT_MAX_CALL_DEPTH: usize = 512;

/// The effective depth cap: `METALOGOS_VM_MAX_DEPTH` overrides the
/// default; an unparseable/zero value falls back to the default loudly
/// ignored (the knob must never brick a deployment — the honest note
/// lives in docs/limitations.md).
pub fn max_call_depth() -> usize {
    match std::env::var("METALOGOS_VM_MAX_DEPTH") {
        Ok(v) => match v.parse::<usize>() {
            Ok(n) if n >= 1 => n,
            _ => DEFAULT_MAX_CALL_DEPTH,
        },
        Err(_) => DEFAULT_MAX_CALL_DEPTH,
    }
}

/// The pre-allocation capacity guard for multiplicative string builtins:
/// `unit_len * count` is refused BEFORE the allocation. The old behavior
/// attempted `s.repeat(n)` first — a 1 TB request died in the allocator
/// (abort, not panic) and took the serve process down.
pub fn check_string_capacity(op: &str, unit_len: usize, count: usize) -> Result<(), String> {
    let need = unit_len.saturating_mul(count);
    if need > MAX_STRING_LENGTH {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_STRING_TOO_LARGE,
            format!(
                "{op}: requested length {need} exceeds the {MAX_STRING_LENGTH}-byte limit — refused BEFORE the allocation"
            ),
        ));
    }
    Ok(())
}
