// ── src/provider_contract.rs ─────────────────────────────────────────
// №879 (issue #1739, wave 78 — W-G, ADR-0208 §2.1): the provider-agnostic
// contract of the LLM layer. A call addresses THE CONTRACT — the
// capabilities (what the answering side can do), the limits (the
// ceilings a call is bound by) and the health semantics (the
// circuit-breaker posture the grammar's `circuit_breaker` field
// configures) — never a vendor name. `Provider` (the roster enum),
// `RealLlm` and `MockLlm` stay the IMPLEMENTATIONS; this module is the
// NAMED VIEW over them (ADR-0208 §6: the parents are consumed by
// reference — a drift between the view and a parent is a loud
// verification failure, never a silent overwrite).
//
// The dedup guard (№879): the contract NAME is `ProviderContract` —
// deliberately apart from the existing `enum Provider` (the
// implementation roster, llm.rs:240). The contract never routes a call
// (SmartRouter stays the loop's SSOT), never issues a right (ADR-0203
// stays the rights SSOT) and never grows the refusal vocabulary (№385:
// the roster refusal below rides the EXISTING `LLM_PROVIDER_UNAVAILABLE`
// class).

use crate::ast::LlmConfigDecl;
use crate::interpreter::values::{coded_error, CODE_LLM_PROVIDER_UNAVAILABLE};
use crate::llm::{env_max_tokens, Provider, DEFAULT_LLM_MAX_TOKENS, MAX_RETRIES};

/// The legacy RealLlm lane's wall-clock ceiling (llm.rs `RealLlm::call`
/// builds the blocking client with `Duration::from_secs(120)`). The
/// router lane's ceiling is the `llm { timeout }` field instead — see
/// `ProviderContract::from_config`.
pub(crate) const LEGACY_REAL_LLM_TIMEOUT_SECS: u32 = 120;

/// The circuit-breaker recovery window of `ProviderHealth` (llm.rs —
/// `circuit_recovery_secs: 60`, the half-open recovery rule).
pub(crate) const PROVIDER_HEALTH_RECOVERY_SECS: u64 = 60;

/// The health window length of `ProviderHealth` (llm.rs —
/// `max_window: 20`, the rolling sample the health score reads).
pub(crate) const PROVIDER_HEALTH_WINDOW: usize = 20;

/// The grammar default of `llm { circuit_breaker }` (the parser pins 3
/// when the field is absent — `test_parse_llm_config_empty`).
pub(crate) const DEFAULT_CIRCUIT_BREAKER: u32 = 3;

/// Which side of the contract answers: the local provider (the
/// deterministic, offline, non-echo backend — ADR-0208 §2.2's
/// local-first default) or an external one (the HTTP ladder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// `MockLlm` — the local provider: deterministic, offline, the
    /// visible `[mock-llm:<hex>]` marker in every answer (№454/№487).
    Local,
    /// The external roster (`RealLlm` over `Provider`: anthropic /
    /// openai / ollama) — the network optimization, never a
    /// precondition of the surface working.
    External,
}

impl ProviderKind {
    fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Local => "local",
            ProviderKind::External => "external",
        }
    }
}

/// The capabilities an implementation reports to the contract
/// (ADR-0208 §2.1 — what the answering side CAN DO).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    /// The answers are deterministic — same prompt, same bytes, no
    /// network (the local provider). An external provider answers
    /// through the network and makes no such guarantee.
    pub deterministic_answer: bool,
    /// The per-call model override is honored (ADR-0048 — the
    /// `call_with_model` seam every implementation carries).
    pub per_call_model: bool,
    /// The deadline-based cancellation is honored (№156/№248 — the
    /// `call_with_deadline` seam: RealLlm drops the TCP connection at
    /// the deadline, MockLlm sleeps `min(delay, deadline)` and fails
    /// loudly when the deadline is tighter).
    pub deadline_cancellation: bool,
}

/// The limits a call is bound by (ADR-0208 §2.1 — the ceilings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderLimits {
    /// The wall-clock ceiling of one call, seconds. `0` = no ceiling
    /// applies (the local provider's answer is immediate and
    /// deterministic — there is no latency to bound).
    pub timeout_secs: u32,
    /// The generation ceiling sent in the request body (№757). `0` =
    /// not applicable (the local marker is not token-bounded).
    pub max_tokens: u32,
    /// The retry ladder of ONE call to the implementation (the
    /// exponential backoff of the legacy RealLlm lane; the router
    /// lane's "retry" is the failover chain itself — reported as 0
    /// rung-internal retries).
    pub retries: u32,
}

/// The health semantics of the contract (ADR-0208 §2.1/§2.3 — the
/// circuit-breaker posture; `ProviderHealth` stays the tracker SSOT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderHealthSemantics {
    /// The `llm { circuit_breaker }` threshold — the consecutive-failure
    /// count that opens the circuit (grammar.pest:40; honored BY the
    /// contract view — see `from_config`).
    pub circuit_threshold: u32,
    /// The half-open recovery window, seconds (`ProviderHealth`).
    pub recovery_secs: u64,
    /// The rolling health window length (`ProviderHealth`).
    pub window: usize,
    /// The `llm { failover: auto }` posture — the chain is a CONTRACT
    /// behavior (ADR-0208 §2.3): every implementation is bound by the
    /// same loop and the same exhaustion refusal classes.
    pub failover_auto: bool,
}

/// The provider-agnostic contract (ADR-0208 §2.1). The address a call
/// speaks to; the providers are interchangeable implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderContract {
    /// The roster name the contract answers for: `local` (the
    /// local-first provider), `anthropic` / `openai` / `ollama` (the
    /// external roster), `router` (the `llm {}` block view).
    pub name: &'static str,
    /// Which side answers (local / external).
    pub kind: ProviderKind,
    /// What the answering side can do.
    pub capabilities: ProviderCapabilities,
    /// The ceilings a call is bound by.
    pub limits: ProviderLimits,
    /// The circuit-breaker posture.
    pub health: ProviderHealthSemantics,
}

impl ProviderContract {
    /// The contract of the LOCAL provider (`MockLlm` — the
    /// deterministic, offline, non-echo backend of №454/№487). The
    /// local provider is a FULL-RIGHT member of the implementation
    /// roster (ADR-0208 §2.1), not a test stub: it answers with the
    /// visible frozen marker, honors the per-call model override and
    /// the deadline, needs no network and no credentials.
    pub fn local() -> Self {
        ProviderContract {
            name: "local",
            kind: ProviderKind::Local,
            capabilities: ProviderCapabilities {
                deterministic_answer: true,
                per_call_model: true,
                deadline_cancellation: true,
            },
            limits: ProviderLimits {
                // No wall-clock ceiling and no token ceiling apply: the
                // marker is computed, not generated (honest 0s, never
                // invented numbers).
                timeout_secs: 0,
                max_tokens: 0,
                retries: 0,
            },
            health: ProviderHealthSemantics {
                circuit_threshold: DEFAULT_CIRCUIT_BREAKER,
                recovery_secs: PROVIDER_HEALTH_RECOVERY_SECS,
                window: PROVIDER_HEALTH_WINDOW,
                // The local provider is a single deterministic rung —
                // there is no chain to walk.
                failover_auto: false,
            },
        }
    }

    /// The contract of an EXTERNAL roster entry (`RealLlm` over
    /// `Provider` — the legacy lane's view: the 120s client ceiling,
    /// the 3-rung backoff, the №757 generation ceiling).
    pub fn for_provider(provider: &Provider) -> Self {
        let name: &'static str = match provider {
            Provider::Anthropic => "anthropic",
            Provider::OpenAI => "openai",
            Provider::Ollama => "ollama",
        };
        ProviderContract {
            name,
            kind: ProviderKind::External,
            capabilities: ProviderCapabilities {
                deterministic_answer: false,
                per_call_model: true,
                deadline_cancellation: true,
            },
            limits: ProviderLimits {
                timeout_secs: LEGACY_REAL_LLM_TIMEOUT_SECS,
                max_tokens: DEFAULT_LLM_MAX_TOKENS,
                retries: MAX_RETRIES,
            },
            health: ProviderHealthSemantics {
                circuit_threshold: DEFAULT_CIRCUIT_BREAKER,
                recovery_secs: PROVIDER_HEALTH_RECOVERY_SECS,
                window: PROVIDER_HEALTH_WINDOW,
                // The legacy lane is a single backend — the failover
                // chain belongs to the router view (`from_config`).
                failover_auto: false,
            },
        }
    }

    /// The contract view of ONE `llm {}` block — the router-level
    /// limits and health posture the block configures (the `timeout`,
    /// `max_tokens` and `circuit_breaker` fields; the `failover: auto`
    /// chain posture). The per-provider capabilities stay the roster's
    /// (`for_provider` / `local`) — this view names what the CHAIN is
    /// bound by. An empty `providers` list is the local-first default
    /// (ADR-0208 §2.2): the contract reads Local — the block that
    /// declares nothing external runs on the local provider.
    pub fn from_config(config: &LlmConfigDecl) -> Self {
        let kind = if config.providers.is_empty() {
            ProviderKind::Local
        } else {
            ProviderKind::External
        };
        ProviderContract {
            name: "router",
            kind,
            capabilities: ProviderCapabilities {
                // The chain's determinism is the WEAKEST rung's: an
                // external rung answers through the network. Only the
                // local-first chain (empty providers) is deterministic.
                deterministic_answer: config.providers.is_empty(),
                per_call_model: true,
                deadline_cancellation: true,
            },
            limits: ProviderLimits {
                timeout_secs: config.timeout,
                // The same resolution ladder SmartRouter::from_config
                // walks: block value → env → the crate default (№757).
                max_tokens: config
                    .max_tokens
                    .or_else(env_max_tokens)
                    .unwrap_or(DEFAULT_LLM_MAX_TOKENS),
                // The router's "retry" IS the failover chain — a rung
                // is tried once (honest 0 rung-internal retries).
                retries: 0,
            },
            health: ProviderHealthSemantics {
                // The circuit_breaker threshold the grammar carries is
                // HONORED BY THE CONTRACT (№879's negative control):
                // the value here is the block's own, never a constant.
                circuit_threshold: config.circuit_breaker,
                recovery_secs: PROVIDER_HEALTH_RECOVERY_SECS,
                window: PROVIDER_HEALTH_WINDOW,
                failover_auto: config.failover.as_deref() == Some("auto"),
            },
        }
    }

    /// The deterministic report — the canonical serialization of the
    /// contract (fixed field order, no timestamps, no addresses). The
    /// double-run is byte-identical BY CONSTRUCTION (№879's negative
    /// control: the configuration serialization must not drift between
    /// runs — the same contract always reports the same bytes).
    pub fn report(&self) -> String {
        let c = &self.capabilities;
        let l = &self.limits;
        let h = &self.health;
        format!(
            "provider-contract: name={} kind={}\n\
             capabilities: deterministic_answer={} per_call_model={} deadline_cancellation={}\n\
             limits: timeout_secs={} max_tokens={} retries={}\n\
             health: circuit_threshold={} recovery_secs={} window={} failover_auto={}",
            self.name,
            self.kind.as_str(),
            c.deterministic_answer,
            c.per_call_model,
            c.deadline_cancellation,
            l.timeout_secs,
            l.max_tokens,
            l.retries,
            h.circuit_threshold,
            h.recovery_secs,
            h.window,
            h.failover_auto,
        )
    }
}

/// The roster lookup: the contract for a NAME. The roster is CLOSED —
/// `local` plus the `Provider` enum (`anthropic` / `openai` / `ollama`);
/// a new provider is an explicit future transaction (ADR-0208 §5 — the
/// contract makes the EXISTING implementations interchangeable, it does
/// not invite new integrations). A name outside the roster is
/// UNADDRESSABLE: the refusal is loud, rides the EXISTING
/// `LLM_PROVIDER_UNAVAILABLE` class (№385/ADR-0169 — the vocabulary
/// does not grow) and fires BEFORE any network is touched.
pub fn contract_for_name(name: &str) -> Result<ProviderContract, String> {
    match name {
        "local" => Ok(ProviderContract::local()),
        "anthropic" => Ok(ProviderContract::for_provider(&Provider::Anthropic)),
        "openai" => Ok(ProviderContract::for_provider(&Provider::OpenAI)),
        "ollama" => Ok(ProviderContract::for_provider(&Provider::Ollama)),
        other => Err(coded_error(
            CODE_LLM_PROVIDER_UNAVAILABLE,
            format!(
                "provider contract: the name '{}' is not in the closed roster \
                 (local | anthropic | openai | ollama) — the call refuses before \
                 any network is touched (ADR-0208 §2.1, №879)",
                other
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// №879: the report is deterministic — the double-run is
    /// byte-identical for every contract view.
    #[test]
    fn n879_report_double_run_byte_identical() {
        let views = vec![
            ProviderContract::local(),
            ProviderContract::for_provider(&Provider::Anthropic),
            ProviderContract::for_provider(&Provider::OpenAI),
            ProviderContract::for_provider(&Provider::Ollama),
        ];
        for contract in &views {
            let first = contract.report();
            let second = contract.report();
            assert_eq!(
                first.as_bytes(),
                second.as_bytes(),
                "the report must be byte-identical across runs — got:\n{}",
                first
            );
        }
    }

    /// №879: the local provider reports the local kind and the honest
    /// zero ceilings (no invented numbers — the marker is computed).
    #[test]
    fn n879_local_contract_shape() {
        let local = ProviderContract::local();
        assert_eq!(local.name, "local");
        assert_eq!(local.kind, ProviderKind::Local);
        assert!(local.capabilities.deterministic_answer);
        assert!(local.capabilities.per_call_model);
        assert!(local.capabilities.deadline_cancellation);
        assert_eq!(local.limits.timeout_secs, 0);
        assert_eq!(local.limits.max_tokens, 0);
        assert_eq!(local.limits.retries, 0);
        assert!(local.report().contains("kind=local"));
    }

    /// №879: the external roster entries report the external kind with
    /// the legacy lane's honest ceilings (120s / 4096 / 3 retries) and
    /// no determinism guarantee.
    #[test]
    fn n879_external_roster_shape() {
        for provider in [Provider::Anthropic, Provider::OpenAI, Provider::Ollama] {
            let contract = ProviderContract::for_provider(&provider);
            assert_eq!(contract.kind, ProviderKind::External);
            assert!(!contract.capabilities.deterministic_answer);
            assert!(contract.capabilities.deadline_cancellation);
            assert_eq!(contract.limits.timeout_secs, 120);
            assert_eq!(contract.limits.max_tokens, DEFAULT_LLM_MAX_TOKENS);
            assert_eq!(contract.limits.retries, MAX_RETRIES);
        }
    }

    /// №879 (the negative control): the `circuit_breaker` threshold the
    /// grammar carries is HONORED BY THE CONTRACT — `from_config`
    /// reports the block's own value, never a constant; the same for
    /// `timeout`, `max_tokens` and the `failover: auto` posture.
    #[test]
    fn n879_config_contract_honors_the_block() {
        let src = "llm { providers: [{alias: p, provider: openai, url: \"http://127.0.0.1:9\"}], \
                   default_model: \"m\", failover: auto, circuit_breaker: 7, timeout: 45, \
                   max_tokens: 900 }";
        let decls = crate::parser::parse(src).expect("the llm{} block must parse");
        let config = match &decls[0] {
            crate::ast::Declaration::LlmConfig(c) => c.clone(),
            _ => panic!("expected LlmConfig"),
        };
        let view = ProviderContract::from_config(&config);
        assert_eq!(view.health.circuit_threshold, 7, "the block's threshold");
        assert_eq!(view.limits.timeout_secs, 45, "the block's timeout");
        assert_eq!(view.limits.max_tokens, 900, "the block's max_tokens");
        assert!(
            view.health.failover_auto,
            "failover: auto is a contract posture"
        );
        assert_eq!(view.kind, ProviderKind::External);
        // Deterministic double-run over the config view too.
        assert_eq!(
            ProviderContract::from_config(&config).report(),
            view.report()
        );

        // The empty block is the local-first default (ADR-0208 §2.2):
        // the router view reads Local and stays deterministic.
        let empty = "llm { }";
        let decls = crate::parser::parse(empty).expect("the empty llm{} must parse");
        let config = match &decls[0] {
            crate::ast::Declaration::LlmConfig(c) => c.clone(),
            _ => panic!("expected LlmConfig"),
        };
        let empty_view = ProviderContract::from_config(&config);
        assert_eq!(empty_view.kind, ProviderKind::Local);
        assert!(empty_view.capabilities.deterministic_answer);
        assert!(!empty_view.health.failover_auto);
    }

    /// №879 (the negative control): an unaddressable provider name —
    /// one outside the closed roster — refuses LOUDLY through the
    /// EXISTING class (LLM_PROVIDER_UNAVAILABLE, №385: the vocabulary
    /// does not grow), before any network is touched.
    #[test]
    fn n879_unaddressable_name_refuses_loudly() {
        for forged in ["gibberish", "Anthropic", "open_ai", ""] {
            let err = contract_for_name(forged).expect_err("a name outside the roster must refuse");
            assert!(
                err.starts_with("[LLM_PROVIDER_UNAVAILABLE]"),
                "the refusal must ride the EXISTING class at the front — got: {}",
                err
            );
            assert!(
                err.contains(forged),
                "the refusal must name the forged name — got: {}",
                err
            );
        }
        // The roster itself answers.
        for ok in ["local", "anthropic", "openai", "ollama"] {
            assert!(contract_for_name(ok).is_ok(), "{} must resolve", ok);
        }
    }
}
