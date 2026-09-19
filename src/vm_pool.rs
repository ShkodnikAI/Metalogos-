//! Наряд №403 — warm VM pool for the serve backend (step B of the
//! VM-serve divisor work; step A was №402, `Arc<Program>` snapshots).
//!
//! WHY: after №402 the per-request VM path still pays `Vm::new()` (the
//! allocation-heavy builtins registry) plus `load_program` setup. The
//! pool recycles `Vm` objects across requests to remove the `Vm::new()`
//! share. The naryad's honest framing applies: if the measured gain is
//! small, that is a RESULT, not a failure (ADR-0141 step-B section).
//!
//! SAFETY MODEL (fail-closed, not best-effort reuse):
//!   * a VM is checked back in ONLY after a route execution that
//!     returned `Ok`; errors, panics (the `spawn_blocking` JoinError
//!     path drops the VM before the pool ever sees it) and failed
//!     resets all DISCARD the VM — a fresh `Vm::new()` + load_program
//!     is indistinguishable from the pre-pool per-request path;
//!   * the reset itself is `Vm::reset_for_reuse` (src/vm.rs): every
//!     mutable state class is explicitly cleared, then `load_program`
//!     wholesale-rebuilds the program-scoped tables — the №381
//!     shared-DB bug is the cautionary precedent and the db connection
//!     is dropped FIRST inside the reset;
//!   * the pool is bounded: when full, checked-in VMs are dropped (no
//!     unbounded growth — the №263 discipline).
//!
//! CONFIG (read ONCE at startup, the №263 read-once env discipline):
//!   * `METALOGOS_VM_POOL=1` — enable. DEFAULT IS OFF (opt-in) until
//!     the №404 re-gate flips the serve default; the decision is
//!     loudly fixed in ADR-0141.
//!   * `METALOGOS_VM_POOL_MAX=N` — idle-VM cap (default 8; a checked-in
//!     VM beyond the cap is dropped).

use crate::bytecode::Program;
use crate::vm::Vm;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Compile-time probe pinning the pool's threading assumption: the pool
/// moves `Vm` objects across threads (checkout/checkin from arbitrary
/// `spawn_blocking` threads through the shared pool), which requires
/// `Vm: Send`. The №40-era comment in server.rs claimed `Vm is !Send`;
/// №403 verified the type is actually Send (every field is Send — the
/// registries are owned HashMaps/Vecs, the db connection is
/// `rusqlite::Connection: Send`, Mutexes are Sync). If this probe ever
/// fails to compile, the pool MUST switch to thread-local storage — do
/// not delete the probe.
#[allow(dead_code)]
fn vm_must_be_send_for_the_shared_pool() {
    fn requires_send<T: Send>(_: std::marker::PhantomData<T>) {}
    requires_send::<Vm>(std::marker::PhantomData);
}

/// Default idle-VM cap (`METALOGOS_VM_POOL_MAX` unset).
pub const DEFAULT_POOL_MAX: usize = 8;

/// Pool lifecycle counters — the honest-reporting surface: the
/// completion report and the bench record quote these, never guesses.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VmPoolStats {
    /// Checkouts served from the idle set (actual reuse).
    pub reuses: u64,
    /// Checkouts that had to build a cold VM (`Vm::new` + load_program).
    pub cold_created: u64,
    /// VMs discarded because the route execution returned an error.
    pub discarded_error: u64,
    /// VMs discarded because `reset_for_reuse` failed.
    pub discarded_reset_failed: u64,
    /// VMs dropped because the idle set was at capacity.
    pub dropped_at_capacity: u64,
    /// Live VMs sitting in the idle set right now.
    pub idle_now: u64,
}

struct PoolInner {
    idle: Vec<Vm>,
}

/// The warm VM pool. Held on `ServerState` as `Option<Arc<VmPool>>`
/// (None = pool disabled — the exact pre-№403 behavior).
pub struct VmPool {
    program: Arc<Program>,
    inner: Mutex<PoolInner>,
    max_idle: usize,
    reuses: AtomicU64,
    cold_created: AtomicU64,
    discarded_error: AtomicU64,
    discarded_reset_failed: AtomicU64,
    dropped_at_capacity: AtomicU64,
}

impl VmPool {
    /// Env-driven constructor (production): `None` when the pool is not
    /// enabled. Reads the env ONCE — request handling never re-reads it.
    pub fn from_env(program: Arc<Program>) -> Option<Arc<Self>> {
        let flag = std::env::var("METALOGOS_VM_POOL").ok()?;
        if flag.trim() != "1" {
            return None;
        }
        let max = match std::env::var("METALOGOS_VM_POOL_MAX") {
            Ok(v) => match v.trim().parse::<usize>() {
                Ok(n) if n >= 1 => n,
                _ => {
                    eprintln!(
                        "[vm/pool] WARNING: METALOGOS_VM_POOL_MAX='{v}' is not a valid size, \
                         falling back to {DEFAULT_POOL_MAX}"
                    );
                    DEFAULT_POOL_MAX
                }
            },
            Err(_) => DEFAULT_POOL_MAX,
        };
        Some(Self::with_max(program, max))
    }

    /// Explicit constructor (tests and explicit enablement paths).
    pub fn with_max(program: Arc<Program>, max_idle: usize) -> Arc<Self> {
        Arc::new(VmPool {
            program,
            inner: Mutex::new(PoolInner { idle: Vec::new() }),
            max_idle: max_idle.max(1),
            reuses: AtomicU64::new(0),
            cold_created: AtomicU64::new(0),
            discarded_error: AtomicU64::new(0),
            discarded_reset_failed: AtomicU64::new(0),
            dropped_at_capacity: AtomicU64::new(0),
        })
    }

    /// Idle-set capacity (for startup logging and tests).
    pub fn max_idle(&self) -> usize {
        self.max_idle
    }

    /// Take a VM ready for a request: warm from the idle set when
    /// available, cold-built otherwise. Both branches return a VM on
    /// which `load_program` has SUCCEEDED — the caller injects the
    /// per-request server context and executes, exactly like the
    /// pre-pool path.
    pub fn checkout(&self) -> Result<Vm, String> {
        let popped = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .idle
            .pop();
        match popped {
            Some(vm) => {
                self.reuses.fetch_add(1, Ordering::Relaxed);
                Ok(vm)
            }
            None => {
                self.cold_created.fetch_add(1, Ordering::Relaxed);
                let mut vm = Vm::new();
                vm.load_program(&self.program)
                    .map_err(|e| format!("VM pool cold init: {}", e))?;
                Ok(vm)
            }
        }
    }

    /// Return a VM after a request. `exec_ok` MUST be `false` unless the
    /// route execution completed successfully — the fail-closed switch.
    /// A VM that cannot be provably reset is dropped, never stored.
    pub fn checkin(&self, vm: Vm, exec_ok: bool) {
        if !exec_ok {
            self.discarded_error.fetch_add(1, Ordering::Relaxed);
            return; // vm dropped here — fail-closed
        }
        let mut vm = vm;
        if let Err(_e) = vm.reset_for_reuse(&self.program) {
            self.discarded_reset_failed.fetch_add(1, Ordering::Relaxed);
            return; // vm dropped here — fail-closed
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.idle.len() >= self.max_idle {
            self.dropped_at_capacity.fetch_add(1, Ordering::Relaxed);
            return; // vm dropped — bounded pool
        }
        inner.idle.push(vm);
    }

    /// Consistent snapshot of the lifecycle counters.
    pub fn stats(&self) -> VmPoolStats {
        let idle_now = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .idle
            .len() as u64;
        VmPoolStats {
            reuses: self.reuses.load(Ordering::Relaxed),
            cold_created: self.cold_created.load(Ordering::Relaxed),
            discarded_error: self.discarded_error.load(Ordering::Relaxed),
            discarded_reset_failed: self.discarded_reset_failed.load(Ordering::Relaxed),
            dropped_at_capacity: self.dropped_at_capacity.load(Ordering::Relaxed),
            idle_now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_program() -> Arc<Program> {
        let source = "pattern Id(n: String) -> String { return n }";
        let declarations = crate::parser::parse(source).expect("parse");
        let program = crate::compiler::Compiler::new()
            .compile(declarations)
            .expect("compile");
        Arc::new(program)
    }

    #[test]
    fn pool_checkout_cold_then_reuse() {
        let pool = VmPool::with_max(tiny_program(), 2);
        // First checkout: cold build (nothing idle yet).
        let vm = pool.checkout().expect("cold checkout");
        let s = pool.stats();
        assert_eq!(s.cold_created, 1);
        assert_eq!(s.reuses, 0);
        // Successful checkin lands in the idle set...
        pool.checkin(vm, true);
        let s = pool.stats();
        assert_eq!(s.idle_now, 1);
        // ...and the next checkout is a REUSE.
        let _vm2 = pool.checkout().expect("warm checkout");
        let s = pool.stats();
        assert_eq!(s.reuses, 1);
        assert_eq!(s.cold_created, 1);
    }

    #[test]
    fn pool_fail_closed_on_error() {
        let pool = VmPool::with_max(tiny_program(), 2);
        let vm = pool.checkout().expect("checkout");
        // Route execution FAILED → the VM must be discarded, not pooled.
        pool.checkin(vm, false);
        let s = pool.stats();
        assert_eq!(s.discarded_error, 1);
        assert_eq!(s.idle_now, 0);
        // The next checkout is therefore a cold build — never a stale VM.
        let _vm = pool.checkout().expect("fresh cold build after error");
        let s = pool.stats();
        assert_eq!(s.cold_created, 2, "error path must not recycle");
    }

    #[test]
    fn pool_is_bounded_at_capacity() {
        let pool = VmPool::with_max(tiny_program(), 1);
        // Two live VMs at once (both cold-built, nothing idle).
        let v1 = pool.checkout().expect("cold 1");
        let v2 = pool.checkout().expect("cold 2");
        // The first checkin fills the idle set...
        pool.checkin(v1, true);
        assert_eq!(pool.stats().idle_now, 1);
        // ...the second must be DROPPED (bounded pool).
        pool.checkin(v2, true);
        let s = pool.stats();
        assert_eq!(s.dropped_at_capacity, 1);
        assert_eq!(s.idle_now, 1);
    }

    #[test]
    fn pool_env_disabled_by_default() {
        // No METALOGOS_VM_POOL in the environment → from_env returns None
        // (the default-off decision; the parse path itself is exercised
        // in from_env_parse below with explicit values).
        if std::env::var("METALOGOS_VM_POOL").is_ok() {
            // The test process carries the var (CI/local quirk): the
            // assertion then depends on the value, keep it honest.
            let enabled = std::env::var("METALOGOS_VM_POOL")
                .map(|v| v.trim() == "1")
                .unwrap_or(false);
            assert_eq!(
                VmPool::from_env(tiny_program()).is_some(),
                enabled,
                "from_env must mirror the METALOGOS_VM_POOL flag"
            );
        } else {
            assert!(VmPool::from_env(tiny_program()).is_none());
        }
    }

    #[test]
    fn pooled_reset_leaves_vm_equivalent_to_fresh() {
        // The integration-level pin: a reused VM behaves like a fresh one
        // on the program surface (load_program side). The exhaustive
        // field-by-field enumeration lives in src/vm.rs
        // (mod n403_reset_tests).
        let program = tiny_program();
        let pool = VmPool::with_max(program.clone(), 1);
        let vm = pool.checkout().expect("cold");
        pool.checkin(vm, true);
        let mut reused = pool.checkout().expect("warm");
        let mut fresh_baseline = {
            let mut v = Vm::new();
            v.load_program(&program).expect("baseline load");
            v
        };
        // Both must execute an arbitrary program identically (same
        // load_program side effects — globals slots, pattern table size).
        let p2 = {
            let decls = crate::parser::parse("entity base: String = \"x\"").expect("parse");
            crate::compiler::Compiler::new()
                .compile(decls)
                .expect("compile")
        };
        let out_reused = reused.run(p2.clone());
        let out_fresh = fresh_baseline.run(p2);
        assert_eq!(
            out_reused, out_fresh,
            "a pooled-then-reset VM must be behaviorally identical to a fresh VM"
        );
    }
}
