// ── LLM client abstraction for METALOGOS M3 ─────────────────────────
// Phase 7.1: Real LLM backends — Anthropic, OpenAI, Ollama.
// Mock mode for testing (METALOGOS_MOCK_LLM=true).
// Retry with exponential backoff (3 retries, 1s/2s/4s). Timeout 120s.

use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// Acquire a mutex lock, converting poison errors to a user-friendly message.
/// Used in functions that return `Result<_, String>` (Наряд №29 §3.4).
fn lock_or_err<'a, T>(
    guard: Result<
        std::sync::MutexGuard<'a, T>,
        std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
    >,
) -> Result<std::sync::MutexGuard<'a, T>, String> {
    guard.map_err(|e| format!("lock poisoned: {}", e))
}

/// A trait for LLM backends — allows swapping between real and mock.
pub trait LlmBackend: Send + Sync {
    /// Call the LLM with a prompt + input text. Returns the model's text response.
    fn call(&self, prompt: &str, input: &str) -> Result<String, String>;

    /// Call the LLM with an optional per-call model override (ADR-0048).
    /// Default implementation ignores the override and delegates to `call()`.
    /// Real backends use the override model in the API JSON body;
    /// MockLlm records it for contract tests.
    fn call_with_model(
        &self,
        prompt: &str,
        input: &str,
        _model: Option<&str>,
    ) -> Result<String, String> {
        self.call(prompt, input)
    }
}

/// Mock LLM backend for testing. Returns the prompt string as-is (deterministic).
/// This is what golden tests use — the "prompt" field IS the expected response.
///
/// ADR-0047: includes a static call counter for cache contract tests.
/// ADR-0048: records last model override for model-routing contract tests.
pub struct MockLlm;

/// Global call counter for MockLlm. Used by cache contract tests to verify
/// that identical LLM calls are served from cache (counter stays at 1 after
/// two identical invocations).
static MOCK_LLM_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

/// Global last-model tracker for MockLlm (ADR-0048).
/// Records the model name passed to call_with_model().
static MOCK_LLM_LAST_MODEL: Mutex<String> = Mutex::new(String::new());

impl MockLlm {
    /// Reset the global call counter to zero.
    /// Call this before each test that verifies call counts.
    pub fn reset_call_count() {
        MOCK_LLM_CALL_COUNT.store(0, Ordering::SeqCst);
    }

    /// Get the current global call count.
    pub fn call_count() -> u64 {
        MOCK_LLM_CALL_COUNT.load(Ordering::SeqCst)
    }

    /// Reset the last-model tracker to empty.
    pub fn reset_last_model() {
        *MOCK_LLM_LAST_MODEL
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = String::new();
    }

    /// Get the last model override passed to call_with_model().
    /// Empty string if no override was used or call() was called directly.
    pub fn last_model() -> String {
        MOCK_LLM_LAST_MODEL
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl LlmBackend for MockLlm {
    fn call(&self, prompt: &str, _input: &str) -> Result<String, String> {
        MOCK_LLM_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(prompt.to_string())
    }

    fn call_with_model(
        &self,
        prompt: &str,
        input: &str,
        model: Option<&str>,
    ) -> Result<String, String> {
        MOCK_LLM_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        // Record model for contract tests
        if let Some(m) = model {
            *lock_or_err(MOCK_LLM_LAST_MODEL.lock())? = m.to_string();
        } else {
            *lock_or_err(MOCK_LLM_LAST_MODEL.lock())? = String::new();
        }
        Ok(prompt.to_string())
    }
}

// ── Provider Configuration ──────────────────────────────────────────

/// Supported LLM providers.
#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    Anthropic,
    OpenAI,
    Ollama,
}

impl Provider {
    /// Parse provider from environment variable string.
    pub fn from_env() -> Self {
        match env::var("METALOGOS_LLM_PROVIDER")
            .unwrap_or_else(|_| "anthropic".to_string())
            .to_lowercase()
            .as_str()
        {
            "openai" => Provider::OpenAI,
            "ollama" => Provider::Ollama,
            _ => Provider::Anthropic,
        }
    }

    /// Default model for this provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::OpenAI => "gpt-4o",
            Provider::Ollama => "llama3",
        }
    }

    /// API endpoint URL.
    pub fn endpoint(&self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com/v1/messages",
            Provider::OpenAI => "https://api.openai.com/v1/chat/completions",
            Provider::Ollama => "http://localhost:11434/api/generate",
        }
    }

    /// Whether this provider requires an API key.
    pub fn requires_api_key(&self) -> bool {
        matches!(self, Provider::Anthropic | Provider::OpenAI)
    }
}

// ── Real LLM Backend ─────────────────────────────────────────────────

/// Maximum number of retries on transient errors (rate limit, server errors).
const MAX_RETRIES: u32 = 3;

/// Real LLM backend — HTTP POST to Anthropic, OpenAI, or Ollama API.
///
/// Features:
/// - 3 providers with provider-specific request/response formats
/// - Exponential backoff retry (3 retries: 1s, 2s, 4s delays)
/// - 30-second timeout per attempt, 10-second connect timeout
/// - JSON response parsing per provider format
/// - No retry on fatal client errors (400/401/403/404)
/// - ADR-0048: per-call model override via call_with_model()
/// - Наряд №12 Bug 4: METALOGOS_OPENAI_BASE_URL for custom base URL
/// - Наряд №32: deduplication of path suffix in resolve_endpoint

/// Extract the meaningful path suffix from a full endpoint URL.
/// Strips the versioned prefix (/v1/, /v1beta/, /api/) to enable deduplication
/// when a custom base_url already contains the versioned segment.
///
/// Examples:
/// - "https://api.openai.com/v1/chat/completions" → "/chat/completions"
/// - "https://api.anthropic.com/v1/messages" → "/messages"
/// - "http://localhost:11434/api/generate" → "/generate"
fn extract_endpoint_suffix(default_endpoint: &str) -> &str {
    if let Some(idx) = default_endpoint.find("://") {
        let after_scheme = &default_endpoint[idx + 3..];
        if let Some(slash_idx) = after_scheme.find('/') {
            let path = &after_scheme[slash_idx..];
            // Try to find and skip /v1/, /v1beta/, /api/ prefix
            for prefix in &["/v1beta/", "/v1/", "/api/"] {
                if let Some(pos) = path.find(prefix) {
                    return &path[pos + prefix.len() - 1..]; // keep the "/"
                }
            }
            return path;
        }
    }
    ""
}

#[derive(Clone)]
pub struct RealLlm {
    provider: Provider,
    model: String,
    api_key: Option<String>,
    /// Custom base URL override (Наряд №12 Bug 4).
    /// When set, replaces the provider's default endpoint base.
    /// For OpenAI: "https://api.openai.com/v1/chat/completions" becomes "{base_url}/chat/completions"
    pub base_url: Option<String>,
}

impl RealLlm {
    /// Create a new RealLlm backend from environment configuration.
    ///
    /// Environment variables:
    /// - `METALOGOS_LLM_PROVIDER`: "anthropic" | "openai" | "ollama" (default: anthropic)
    /// - `METALOGOS_LLM_MODEL`: model name (default: provider's default model)
    /// - `METALOGOS_API_KEY`: API key for Anthropic/OpenAI (required for those providers)
    /// - `METALOGOS_OPENAI_BASE_URL`: custom base URL for OpenAI (Наряд №12 Bug 4)
    ///   e.g. "https://my-proxy.example.com/v1" — the path "/chat/completions" is appended automatically
    pub fn new() -> Self {
        let provider = Provider::from_env();
        let model = env::var("METALOGOS_LLM_MODEL")
            .unwrap_or_else(|_| provider.default_model().to_string());
        let api_key = env::var("METALOGOS_API_KEY").ok();
        // Наряд №12 Bug 4: Read custom base URL from env
        let base_url = env::var("METALOGOS_OPENAI_BASE_URL").ok();

        RealLlm {
            provider,
            model,
            api_key,
            base_url,
        }
    }

    /// Create a RealLlm with explicit configuration (for testing).
    pub fn with_config(provider: Provider, model: String, api_key: Option<String>) -> Self {
        RealLlm {
            provider,
            model,
            api_key,
            base_url: None,
        }
    }
}

impl LlmBackend for RealLlm {
    fn call(&self, prompt: &str, input: &str) -> Result<String, String> {
        // Build blocking HTTP client with 120s timeout
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client build error: {}", e))?;

        // Retry loop: first attempt + 3 retries = 4 total attempts
        // Delays between retries: 1s, 2s, 4s (exponential backoff)
        let mut last_error = String::new();
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = Duration::from_secs(1u64 << (attempt - 1));
                std::thread::sleep(delay);
            }

            match self.call_provider(&client, prompt, input) {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = e.clone();
                    // Don't retry on fatal client errors (4xx, excluding 429)
                    if is_client_error(&e) {
                        return Err(e);
                    }
                    // On last retry, give up
                    if attempt == MAX_RETRIES {
                        break;
                    }
                }
            }
        }

        Err(format!(
            "LLM call failed after {} retries: {}",
            MAX_RETRIES, last_error
        ))
    }

    /// ADR-0048: Call with per-pattern model override.
    /// If a model override is provided and differs from the global model,
    /// clone self with the overridden model and call through that.
    fn call_with_model(
        &self,
        prompt: &str,
        input: &str,
        model: Option<&str>,
    ) -> Result<String, String> {
        match model {
            Some(m) if m != self.model => {
                let mut backend = self.clone();
                backend.model = m.to_string();
                backend.call(prompt, input)
            }
            _ => self.call(prompt, input),
        }
    }
}

impl RealLlm {
    /// Dispatch to the correct provider implementation.
    fn call_provider(
        &self,
        client: &reqwest::blocking::Client,
        prompt: &str,
        input: &str,
    ) -> Result<String, String> {
        match &self.provider {
            Provider::Anthropic => self.call_anthropic(client, prompt, input),
            Provider::OpenAI => self.call_openai(client, prompt, input),
            Provider::Ollama => self.call_ollama(client, prompt, input),
        }
    }

    /// Resolve the effective endpoint URL, applying custom base_url override if set.
    /// Наряд №12 Bug 4: METALOGOS_OPENAI_BASE_URL support.
    /// Наряд №32: deduplicate path suffix when base_url already contains it.
    ///
    /// Strategy: strip the versioned prefix (/v1/ or /api/) from the default
    /// endpoint's path, take only the suffix (e.g. /chat/completions), then
    /// append it to base_url. If base_url already ends with that suffix, return
    /// base_url as-is.
    pub fn resolve_endpoint(&self) -> String {
        if let Some(ref base) = self.base_url {
            let base = base.trim_end_matches('/');
            let default = self.provider.endpoint();
            // Extract the path after the versioned segment from the default endpoint
            let suffix = extract_endpoint_suffix(&default);
            // Deduplicate
            if base.ends_with(suffix) {
                base.to_string()
            } else {
                format!("{}{}", base, suffix)
            }
        } else {
            self.provider.endpoint().to_string()
        }
    }

    // ── Anthropic Claude ────────────────────────────────────────────

    /// Call Anthropic Claude API.
    /// POST https://api.anthropic.com/v1/messages
    /// Headers: x-api-key, anthropic-version
    fn call_anthropic(
        &self,
        client: &reqwest::blocking::Client,
        prompt: &str,
        input: &str,
    ) -> Result<String, String> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            "Anthropic requires METALOGOS_API_KEY. \
             Set it or use METALOGOS_MOCK_LLM=true for testing."
                .to_string()
        })?;

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": format!("{}\n\nInput: {}", prompt, input)
            }]
        });

        let response = client
            .post(Provider::Anthropic.endpoint())
            .header("x-api-key", api_key.as_str())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("Anthropic request failed: {}", e))?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e| format!("Anthropic response read error: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "Anthropic API error ({}): {}",
                status.as_u16(),
                truncate(&body_text, 500)
            ));
        }

        parse_anthropic_response(&body_text)
    }

    // ── OpenAI GPT ──────────────────────────────────────────────────

    /// Call OpenAI GPT API.
    /// POST https://api.openai.com/v1/chat/completions
    /// Header: Authorization: Bearer
    fn call_openai(
        &self,
        client: &reqwest::blocking::Client,
        prompt: &str,
        input: &str,
    ) -> Result<String, String> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            "OpenAI requires METALOGOS_API_KEY. \
             Set it or use METALOGOS_MOCK_LLM=true for testing."
                .to_string()
        })?;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": format!("{}\n\nInput: {}", prompt, input)
            }],
            "max_tokens": 1024,
            "temperature": 0.0
        });

        let response = client
            .post(self.resolve_endpoint())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("OpenAI request failed: {}", e))?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e| format!("OpenAI response read error: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "OpenAI API error ({}): {}",
                status.as_u16(),
                truncate(&body_text, 500)
            ));
        }

        parse_openai_response(&body_text)
    }

    // ── Ollama (local) ─────────────────────────────────────────────

    /// Call Ollama local model API.
    /// POST http://localhost:11434/api/generate
    /// No API key required.
    fn call_ollama(
        &self,
        client: &reqwest::blocking::Client,
        prompt: &str,
        input: &str,
    ) -> Result<String, String> {
        let body = serde_json::json!({
            "model": self.model,
            "prompt": format!("{}\n\nInput: {}", prompt, input),
            "stream": false
        });

        let response = client
            .post(Provider::Ollama.endpoint())
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| {
                format!(
                    "Ollama request failed (is Ollama running at localhost:11434?): {}",
                    e
                )
            })?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e| format!("Ollama response read error: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "Ollama API error ({}): {}",
                status.as_u16(),
                truncate(&body_text, 500)
            ));
        }

        parse_ollama_response(&body_text)
    }
}

// ── Response Parsing ───────────────────────────────────────────────

/// Parse Anthropic response: `{ "content": [{ "type": "text", "text": "..." }] }`
fn parse_anthropic_response(raw: &str) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("Failed to parse Anthropic JSON: {}", e))?;

    json.get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
        })
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .map(|t| t.trim().to_string())
        .ok_or_else(|| {
            format!(
                "Unexpected Anthropic response format: {}",
                truncate(raw, 300)
            )
        })
}

/// Parse OpenAI response: `{ "choices": [{ "message": { "content": "..." } }] }`
fn parse_openai_response(raw: &str) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("Failed to parse OpenAI JSON: {}", e))?;

    json.get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(|c| c.as_str())
        .map(|t| t.trim().to_string())
        .ok_or_else(|| format!("Unexpected OpenAI response format: {}", truncate(raw, 300)))
}

/// Parse Ollama response: `{ "response": "..." }`
fn parse_ollama_response(raw: &str) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("Failed to parse Ollama JSON: {}", e))?;

    json.get("response")
        .and_then(|r| r.as_str())
        .map(|t| t.trim().to_string())
        .ok_or_else(|| format!("Unexpected Ollama response format: {}", truncate(raw, 300)))
}

// ── Retry Helpers ──────────────────────────────────────────────────

/// Check if an error string indicates a fatal client-side error (4xx, excluding 429).
fn is_client_error(error: &str) -> bool {
    // 429 is rate limit — should be retried, not treated as fatal
    if is_rate_limit(error) {
        return false;
    }
    let code = extract_status_hundreds(error);
    code == 4
}

/// Check if an error string indicates a rate limit error (429).
fn is_rate_limit(error: &str) -> bool {
    error.contains("429") || error.to_lowercase().contains("rate limit")
}

/// Extract HTTP status code hundreds digit from error string.
/// Looks for pattern "(NNN):" and returns NNN/100.
fn extract_status_hundreds(error: &str) -> u32 {
    for part in error.split(' ') {
        if part.starts_with('(') && part.ends_with("):") {
            let inner = &part[1..part.len() - 2];
            if let Ok(code) = inner.parse::<u32>() {
                return code / 100;
            }
        }
    }
    0
}

/// Truncate a string to max_len bytes, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .take_while(|(i, _)| *i < max_len)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(max_len);
        format!("{}...", &s[..end])
    }
}

// ── Factory ─────────────────────────────────────────────────────────

/// Resolve a model alias to an actual model name using environment variables.
///
/// Lookup order:
/// 1. If `METALOGOS_LLM_MODEL_{alias}` exists → use its value
/// 2. Otherwise → return the alias as-is (treated as a direct model name)
///
/// # Examples
/// ```ignore
/// // METALOGOS_LLM_MODEL_fast=claude-haiku-4-5-20251001
/// resolve_model("fast")       → "claude-haiku-4-5-20251001"
/// resolve_model("claude-sonnet-4-20250514") → "claude-sonnet-4-20250514"
/// resolve_model("unknown")    → "unknown"
/// ```
pub fn resolve_model(alias: &str) -> String {
    let env_key = format!("METALOGOS_LLM_MODEL_{}", alias);
    env::var(&env_key).unwrap_or_else(|_| alias.to_string())
}

/// Create an LLM backend based on environment configuration.
///
/// - If `METALOGOS_MOCK_LLM=1` or `METALOGOS_MOCK_LLM=true`: returns MockLlm (for tests)
/// - Otherwise: returns RealLlm configured from env vars
///
/// **Defaults to MockLlm for safety** — no accidental API calls in tests or CI.
pub fn create_llm_backend() -> Box<dyn LlmBackend> {
    let use_mock = env::var("METALOGOS_MOCK_LLM")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(true); // Default to mock for safety

    if use_mock {
        Box::new(MockLlm)
    } else {
        Box::new(RealLlm::new())
    }
}

/// Global LLM usage tracker (Наряд №4).
/// Shared across all SmartRouter instances. Written by SmartRouter::call(),
/// read by the llm_usage() builtin.
pub static GLOBAL_LLM_USAGE: once_cell::sync::Lazy<StdMutex<LlmUsageTracker>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(LlmUsageTracker::new_empty()));

/// Reset the global LLM usage tracker (for tests).
pub fn reset_global_llm_usage() {
    if let Ok(mut tracker) = GLOBAL_LLM_USAGE.lock() {
        *tracker = LlmUsageTracker::new_empty();
    }
}

/// Get a snapshot of the global LLM usage tracker report.
pub fn global_llm_usage_report() -> LlmUsageReport {
    if let Ok(tracker) = GLOBAL_LLM_USAGE.lock() {
        tracker.report()
    } else {
        LlmUsageReport {
            total_calls: 0.0,
            total_tokens: 0.0,
            total_errors: 0.0,
            providers: Vec::new(),
        }
    }
}

// ── Smart Router (Наряд №4: LLM Routing with Failover + Circuit Breaker) ──

use std::sync::Mutex as StdMutex;
use std::time::Instant;

/// Per-provider health tracking entry.
#[derive(Debug, Clone)]
struct ProviderHealth {
    /// Timestamped results: (Instant, success: bool)
    window: Vec<(Instant, bool)>,
    /// Max entries in the health window.
    max_window: usize,
    /// Circuit breaker threshold: opens after N consecutive failures.
    circuit_threshold: u32,
    /// Current consecutive failure count.
    consecutive_failures: u32,
    /// Whether circuit is open (provider temporarily skipped).
    circuit_open: bool,
    /// When circuit was opened (for half-open recovery).
    circuit_opened_at: Option<Instant>,
    /// Circuit breaker recovery time in seconds.
    circuit_recovery_secs: u64,
}

impl ProviderHealth {
    fn new(circuit_threshold: u32) -> Self {
        ProviderHealth {
            window: Vec::new(),
            max_window: 20,
            circuit_threshold,
            consecutive_failures: 0,
            circuit_open: false,
            circuit_opened_at: None,
            circuit_recovery_secs: 60,
        }
    }

    /// Record a call result. Returns true if the circuit should trip open.
    fn record(&mut self, success: bool) {
        self.window.push((Instant::now(), success));
        if self.window.len() > self.max_window {
            self.window.remove(0);
        }
        if success {
            self.consecutive_failures = 0;
            // Close circuit on success (half-open state)
            self.circuit_open = false;
            self.circuit_opened_at = None;
        } else {
            self.consecutive_failures += 1;
            if self.consecutive_failures >= self.circuit_threshold && !self.circuit_open {
                self.circuit_open = true;
                self.circuit_opened_at = Some(Instant::now());
            }
        }
    }

    /// Check if this provider should be skipped (circuit open and not yet recovered).
    fn is_available(&mut self) -> bool {
        if !self.circuit_open {
            return true;
        }
        // Half-open: check if recovery time has elapsed
        if let Some(opened) = self.circuit_opened_at {
            if opened.elapsed().as_secs() >= self.circuit_recovery_secs {
                self.circuit_open = false;
                self.circuit_opened_at = None;
                return true;
            }
        }
        false
    }

    /// Health score: success_count / total_count in window.
    fn health_score(&self) -> f64 {
        if self.window.is_empty() {
            return 1.0;
        }
        let successes = self.window.iter().filter(|(_, ok)| *ok).count();
        successes as f64 / self.window.len() as f64
    }
}

/// Per-provider usage statistics.
#[derive(Debug, Clone)]
pub struct ProviderUsage {
    pub alias: String,
    pub calls: u64,
    pub tokens: u64,
    pub errors: u64,
    pub avg_latency_ms: f64,
    pub health_score: f64,
}

/// Global LLM usage tracker (thread-safe).
pub struct LlmUsageTracker {
    total_calls: StdMutex<u64>,
    total_tokens: StdMutex<u64>,
    total_errors: StdMutex<u64>,
    providers: StdMutex<Vec<ProviderHealth>>,
    provider_names: Vec<String>,
    /// Per-provider: calls, tokens, errors, latencies for avg
    provider_calls: StdMutex<Vec<u64>>,
    provider_tokens: StdMutex<Vec<u64>>,
    provider_errors: StdMutex<Vec<u64>>,
    provider_latencies: StdMutex<Vec<u64>>,
}

impl LlmUsageTracker {
    /// Create an empty tracker (no providers).
    pub fn new_empty() -> Self {
        LlmUsageTracker {
            total_calls: StdMutex::new(0),
            total_tokens: StdMutex::new(0),
            total_errors: StdMutex::new(0),
            providers: StdMutex::new(Vec::new()),
            provider_names: Vec::new(),
            provider_calls: StdMutex::new(Vec::new()),
            provider_tokens: StdMutex::new(Vec::new()),
            provider_errors: StdMutex::new(Vec::new()),
            provider_latencies: StdMutex::new(Vec::new()),
        }
    }

    pub fn new(provider_names: Vec<String>, circuit_threshold: u32) -> Self {
        let n = provider_names.len();
        LlmUsageTracker {
            total_calls: StdMutex::new(0),
            total_tokens: StdMutex::new(0),
            total_errors: StdMutex::new(0),
            providers: StdMutex::new(
                provider_names
                    .iter()
                    .map(|_| ProviderHealth::new(circuit_threshold))
                    .collect(),
            ),
            provider_names,
            provider_calls: StdMutex::new(vec![0; n]),
            provider_tokens: StdMutex::new(vec![0; n]),
            provider_errors: StdMutex::new(vec![0; n]),
            provider_latencies: StdMutex::new(vec![0; n]),
        }
    }

    pub fn provider_count(&self) -> usize {
        self.provider_names.len()
    }

    pub fn record_call(
        &self,
        provider_idx: usize,
        success: bool,
        prompt_chars: usize,
        latency_ms: u64,
    ) {
        if let Ok(mut providers) = self.providers.lock() {
            if provider_idx < providers.len() {
                providers[provider_idx].record(success);
            }
        }
        // Estimate tokens: chars / 4
        let tokens = (prompt_chars / 4) as u64;
        if let Ok(mut total_calls) = self.total_calls.lock() {
            *total_calls += 1;
        }
        if let Ok(mut total_tokens) = self.total_tokens.lock() {
            *total_tokens += tokens;
        }
        if !success {
            if let Ok(mut total_errors) = self.total_errors.lock() {
                *total_errors += 1;
            }
        }
        if let Ok(mut pc) = self.provider_calls.lock() {
            if provider_idx < pc.len() {
                pc[provider_idx] += 1;
            }
        }
        if let Ok(mut pt) = self.provider_tokens.lock() {
            if provider_idx < pt.len() {
                pt[provider_idx] += tokens;
            }
        }
        if !success {
            if let Ok(mut pe) = self.provider_errors.lock() {
                if provider_idx < pe.len() {
                    pe[provider_idx] += 1;
                }
            }
        }
        if let Ok(mut pl) = self.provider_latencies.lock() {
            if provider_idx < pl.len() {
                pl[provider_idx] = pl[provider_idx].saturating_add(latency_ms);
            }
        }
    }

    /// Check if a specific provider is available (circuit breaker).
    pub fn is_provider_available(&self, idx: usize) -> bool {
        if let Ok(mut providers) = self.providers.lock() {
            if idx < providers.len() {
                return providers[idx].is_available();
            }
        }
        true
    }

    /// Get health score for a specific provider.
    pub fn health_score(&self, idx: usize) -> f64 {
        if let Ok(providers) = self.providers.lock() {
            if idx < providers.len() {
                return providers[idx].health_score();
            }
        }
        1.0
    }

    /// Build usage report as Value-compatible data.
    pub fn report(&self) -> LlmUsageReport {
        let total_calls = self.total_calls.lock().map(|g| *g).unwrap_or(0);
        let total_tokens = self.total_tokens.lock().map(|g| *g).unwrap_or(0);
        let total_errors = self.total_errors.lock().map(|g| *g).unwrap_or(0);

        let mut provider_reports = Vec::new();
        for i in 0..self.provider_names.len() {
            let calls = self
                .provider_calls
                .lock()
                .map(|g| g.get(i).copied().unwrap_or(0))
                .unwrap_or(0);
            let tokens = self
                .provider_tokens
                .lock()
                .map(|g| g.get(i).copied().unwrap_or(0))
                .unwrap_or(0);
            let errors = self
                .provider_errors
                .lock()
                .map(|g| g.get(i).copied().unwrap_or(0))
                .unwrap_or(0);
            let total_lat = self
                .provider_latencies
                .lock()
                .map(|g| g.get(i).copied().unwrap_or(0))
                .unwrap_or(0);
            let avg_lat = if calls > 0 {
                total_lat as f64 / calls as f64
            } else {
                0.0
            };
            let health = self.health_score(i);

            provider_reports.push(ProviderUsage {
                alias: self.provider_names[i].clone(),
                calls,
                tokens,
                errors,
                avg_latency_ms: avg_lat,
                health_score: health,
            });
        }

        LlmUsageReport {
            total_calls: total_calls as f64,
            total_tokens: total_tokens as f64,
            total_errors: total_errors as f64,
            providers: provider_reports,
        }
    }
}

/// Usage report returned by llm_usage() builtin.
#[derive(Debug, Clone)]
pub struct LlmUsageReport {
    pub total_calls: f64,
    pub total_tokens: f64,
    pub total_errors: f64,
    pub providers: Vec<ProviderUsage>,
}

/// Smart LLM router: wraps multiple providers with failover, circuit breaker, health tracking.
pub struct SmartRouter {
    /// Provider configurations (alias, provider_type, api_key, url).
    providers: Vec<(String, String, Option<String>, Option<String>)>,
    /// Default model name/alias.
    default_model: Option<String>,
    /// Failover mode: "auto" or None.
    failover: bool,
    /// Timeout in seconds per provider call.
    timeout: u32,
    /// Health and usage tracker.
    tracker: LlmUsageTracker,
}

impl SmartRouter {
    /// Create a SmartRouter from an LlmConfigDecl.
    pub fn from_config(config: &crate::ast::LlmConfigDecl) -> Self {
        let provider_names: Vec<String> =
            config.providers.iter().map(|p| p.alias.clone()).collect();
        let circuit_threshold = config.circuit_breaker;
        let providers: Vec<(String, String, Option<String>, Option<String>)> = config
            .providers
            .iter()
            .map(|p| {
                // Evaluate key expression: if it's env("KEY"), resolve at runtime
                let key = p.key.as_ref().and_then(|expr| match expr {
                    crate::ast::Expr::FnCall(name, args) if name == "env" => {
                        args.first().and_then(|a| {
                            if let crate::ast::Expr::StringLit(s) = a {
                                std::env::var(s).ok()
                            } else {
                                None
                            }
                        })
                    }
                    crate::ast::Expr::StringLit(s) => Some(s.clone()),
                    _ => None,
                });
                (p.alias.clone(), p.provider.clone(), key, p.url.clone())
            })
            .collect();

        SmartRouter {
            providers,
            default_model: config.default_model.clone(),
            failover: config.failover.as_deref() == Some("auto"),
            timeout: config.timeout,
            tracker: LlmUsageTracker::new(provider_names, circuit_threshold),
        }
    }

    /// Call the LLM with smart routing.
    /// 1. Pick best available provider (by health_score)
    /// 2. Try it; on failure, try next available provider (failover)
    /// 3. Track usage for each attempt
    pub fn call(
        &self,
        prompt: &str,
        input: &str,
        model_override: Option<&str>,
    ) -> Result<String, String> {
        if self.providers.is_empty() {
            // No providers configured — fall back to legacy behavior
            let backend = create_llm_backend();
            return backend.call(prompt, input);
        }

        let effective_prompt_len = prompt.len() + input.len();

        // Build ordered list of provider indices, sorted by health_score desc
        let mut candidates: Vec<usize> = (0..self.providers.len()).collect();
        candidates.sort_by(|&a, &b| {
            let sa = self.tracker.health_score(a);
            let sb = self.tracker.health_score(b);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut last_error = String::new();

        for &idx in &candidates {
            if !self.tracker.is_provider_available(idx) {
                continue; // circuit breaker open — skip
            }

            let (ref alias, ref provider_type, ref api_key, ref url) = self.providers[idx];
            let start = Instant::now();

            let result = self.call_provider(
                provider_type,
                api_key.as_deref(),
                url.as_deref(),
                prompt,
                input,
                model_override,
            );

            let latency_ms = start.elapsed().as_millis() as u64;
            let success = result.is_ok();
            self.tracker
                .record_call(idx, success, effective_prompt_len, latency_ms);
            // Also record to global tracker for llm_usage() builtin
            if let Ok(global) = GLOBAL_LLM_USAGE.lock() {
                global.record_call(idx, success, effective_prompt_len, latency_ms);
            }

            match result {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = e.clone();
                    if !self.failover {
                        break; // manual mode — don't try next provider
                    }
                    // Continue to next provider (failover)
                }
            }
        }

        // All providers exhausted — soft failure
        Err(format!(
            "All LLM providers failed. Last error: {}",
            truncate(&last_error, 200)
        ))
    }

    /// Make a single provider call using the appropriate format.
    fn call_provider(
        &self,
        provider_type: &str,
        api_key: Option<&str>,
        url: Option<&str>,
        prompt: &str,
        input: &str,
        model_override: Option<&str>,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(self.timeout.max(5) as u64))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client build error: {}", e))?;

        let resolved_model = model_override
            .or(self.default_model.as_deref())
            .unwrap_or("default");

        let body_text = format!("{}\n\nInput: {}", prompt, input);
        let body = serde_json::json!({
            "model": resolved_model,
            "messages": [{ "role": "user", "content": body_text }],
            "max_tokens": 1024,
            "temperature": 0.0
        });

        let endpoint = self.resolve_endpoint(provider_type, url);

        match provider_type {
            "anthropic" => {
                let key = api_key.ok_or_else(|| "anthropic requires an API key".to_string())?;
                // Anthropic uses a different format
                let anth_body = serde_json::json!({
                    "model": resolved_model,
                    "max_tokens": 1024,
                    "messages": [{ "role": "user", "content": body_text }]
                });
                let resp = client
                    .post(&endpoint)
                    .header("x-api-key", key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&anth_body)
                    .send()
                    .map_err(|e| format!("Anthropic request failed: {}", e))?;
                let status = resp.status();
                let text = resp
                    .text()
                    .map_err(|e| format!("Response read error: {}", e))?;
                if !status.is_success() {
                    return Err(format!(
                        "Anthropic API error ({}): {}",
                        status.as_u16(),
                        truncate(&text, 500)
                    ));
                }
                parse_anthropic_response(&text)
            }
            "ollama" => {
                let ollama_body = serde_json::json!({
                    "model": resolved_model,
                    "prompt": body_text,
                    "stream": false
                });
                let resp = client
                    .post(&endpoint)
                    .header("content-type", "application/json")
                    .json(&ollama_body)
                    .send()
                    .map_err(|e| format!("Ollama request failed: {}", e))?;
                let status = resp.status();
                let text = resp
                    .text()
                    .map_err(|e| format!("Response read error: {}", e))?;
                if !status.is_success() {
                    return Err(format!(
                        "Ollama API error ({}): {}",
                        status.as_u16(),
                        truncate(&text, 500)
                    ));
                }
                parse_ollama_response(&text)
            }
            _ => {
                // OpenAI-compatible: openai, groq, cerebras, nvidia, openrouter, custom
                let key = api_key;
                let mut req = client
                    .post(&endpoint)
                    .header("content-type", "application/json")
                    .json(&body);
                if let Some(k) = key {
                    req = req.header("Authorization", format!("Bearer {}", k));
                }
                let resp = req
                    .send()
                    .map_err(|e| format!("{} request failed: {}", provider_type, e))?;
                let status = resp.status();
                let text = resp
                    .text()
                    .map_err(|e| format!("Response read error: {}", e))?;
                if !status.is_success() {
                    return Err(format!(
                        "{} API error ({}): {}",
                        provider_type,
                        status.as_u16(),
                        truncate(&text, 500)
                    ));
                }
                parse_openai_response(&text)
            }
        }
    }

    /// Resolve the endpoint URL for a provider type.
    fn resolve_endpoint(&self, provider_type: &str, url: Option<&str>) -> String {
        if let Some(u) = url {
            // Custom URL: append /v1/chat/completions for OpenAI-compatible, or /api/generate for ollama
            if provider_type == "ollama" {
                format!("{}/api/generate", u.trim_end_matches('/'))
            } else if provider_type == "anthropic" {
                format!("{}/v1/messages", u.trim_end_matches('/'))
            } else {
                format!("{}/v1/chat/completions", u.trim_end_matches('/'))
            }
        } else {
            match provider_type {
                "anthropic" => "https://api.anthropic.com/v1/messages".to_string(),
                "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
                "ollama" => "http://localhost:11434/api/generate".to_string(),
                "groq" => "https://api.groq.com/openai/v1/chat/completions".to_string(),
                "cerebras" => "https://api.cerebras.ai/v1/chat/completions".to_string(),
                "nvidia" => "https://integrate.api.nvidia.com/v1/chat/completions".to_string(),
                "openrouter" => "https://openrouter.ai/api/v1/chat/completions".to_string(),
                "google" => {
                    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
                        .to_string()
                }
                other => format!("https://{}/v1/chat/completions", other),
            }
        }
    }

    /// Get a usage report for the llm_usage() builtin.
    pub fn usage_report(&self) -> LlmUsageReport {
        self.tracker.report()
    }

    /// Get the number of providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Get provider alias by index.
    pub fn provider_alias(&self, idx: usize) -> Option<&str> {
        self.providers.get(idx).map(|(alias, ..)| alias.as_str())
    }
}

/// Resolve a model alias to an actual model name, considering the llm config.
/// 1. If `METALOGOS_LLM_MODEL_{alias}` env exists → use it
/// 2. If alias matches a provider alias → use that provider (pass through)
/// 3. Otherwise → return as-is (direct model name)
pub fn resolve_model_smart(alias: &str, config: Option<&crate::ast::LlmConfigDecl>) -> String {
    // Check env override first
    let env_key = format!("METALOGOS_LLM_MODEL_{}", alias);
    if let Ok(val) = env::var(&env_key) {
        return val;
    }
    // If no env override, return as-is
    alias.to_string()
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutex to serialize tests that mutate process-wide environment
    /// variables (set_var / remove_var). Without this, parallel test
    /// threads overwrite each other's env, causing flaky failures.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // ── Mock LLM ───────────────────────────────────────────────────

    #[test]
    fn test_mock_llm_returns_prompt() {
        let backend = MockLlm;
        let result = backend.call("classify this", "input text");
        assert_eq!(result.unwrap(), "classify this");
    }

    #[test]
    fn test_mock_llm_ignores_input() {
        let backend = MockLlm;
        let result = backend.call("expected", "ignored");
        assert_eq!(result.unwrap(), "expected");
    }

    // ── Provider ────────────────────────────────────────────────────

    #[test]
    fn test_provider_from_env_default() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("METALOGOS_LLM_PROVIDER");
        assert_eq!(Provider::from_env(), Provider::Anthropic);
    }

    #[test]
    fn test_provider_from_env_openai() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_LLM_PROVIDER", "openai");
        assert_eq!(Provider::from_env(), Provider::OpenAI);
        env::remove_var("METALOGOS_LLM_PROVIDER");
    }

    #[test]
    fn test_provider_from_env_ollama() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_LLM_PROVIDER", "ollama");
        assert_eq!(Provider::from_env(), Provider::Ollama);
        env::remove_var("METALOGOS_LLM_PROVIDER");
    }

    #[test]
    fn test_provider_from_env_case_insensitive() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_LLM_PROVIDER", "OpenAI");
        assert_eq!(Provider::from_env(), Provider::OpenAI);
        env::remove_var("METALOGOS_LLM_PROVIDER");
    }

    #[test]
    fn test_default_models() {
        assert_eq!(
            Provider::Anthropic.default_model(),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(Provider::OpenAI.default_model(), "gpt-4o");
        assert_eq!(Provider::Ollama.default_model(), "llama3");
    }

    #[test]
    fn test_endpoint_urls() {
        assert_eq!(
            Provider::Anthropic.endpoint(),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            Provider::OpenAI.endpoint(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            Provider::Ollama.endpoint(),
            "http://localhost:11434/api/generate"
        );
    }

    #[test]
    fn test_requires_api_key() {
        assert!(Provider::Anthropic.requires_api_key());
        assert!(Provider::OpenAI.requires_api_key());
        assert!(!Provider::Ollama.requires_api_key());
    }

    // ── Response Parsing ────────────────────────────────────────────

    #[test]
    fn test_parse_openai_response_simple() {
        let raw = r#"{"choices":[{"message":{"content":"complaint"}}]}"#;
        assert_eq!(parse_openai_response(raw).unwrap(), "complaint");
    }

    #[test]
    fn test_parse_openai_response_with_usage() {
        let raw = r#"{"choices":[{"message":{"content":"question","role":"assistant"}}],"usage":{"prompt_tokens":10}}"#;
        assert_eq!(parse_openai_response(raw).unwrap(), "question");
    }

    #[test]
    fn test_parse_openai_response_empty_choices() {
        let result = parse_openai_response(r#"{"choices":[]}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_anthropic_response_simple() {
        let raw = r#"{"content":[{"type":"text","text":"greeting"}]}"#;
        assert_eq!(parse_anthropic_response(raw).unwrap(), "greeting");
    }

    #[test]
    fn test_parse_anthropic_response_multiple_blocks() {
        let raw = r#"{"content":[{"type":"text","text":"hello"},{"type":"text","text":" world"}]}"#;
        assert_eq!(parse_anthropic_response(raw).unwrap(), "hello");
    }

    #[test]
    fn test_parse_ollama_response_simple() {
        let raw = r#"{"response":"urgent"}"#;
        assert_eq!(parse_ollama_response(raw).unwrap(), "urgent");
    }

    #[test]
    fn test_parse_ollama_response_with_done() {
        let raw = r#"{"response":"complaint","done":true,"total_duration":12345678}"#;
        assert_eq!(parse_ollama_response(raw).unwrap(), "complaint");
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_openai_response("not json");
        assert!(result.is_err());
    }

    // ── Error Classification ────────────────────────────────────────

    #[test]
    fn test_is_client_error() {
        assert!(is_client_error("OpenAI API error (400): Bad Request"));
        assert!(is_client_error("Anthropic API error (401): Unauthorized"));
        assert!(!is_client_error("OpenAI API error (429): Rate limit"));
        assert!(!is_client_error(
            "OpenAI API error (500): Internal Server Error"
        ));
    }

    #[test]
    fn test_is_rate_limit() {
        assert!(is_rate_limit("OpenAI API error (429): Rate limit exceeded"));
        assert!(is_rate_limit("Rate limit exceeded, retry after 60s"));
        assert!(!is_rate_limit("OpenAI API error (400): Bad Request"));
    }

    // ── Truncate ───────────────────────────────────────────────────

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    // ── RealLlm Construction ────────────────────────────────────────

    #[test]
    fn test_real_llm_new_default_provider() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("METALOGOS_LLM_PROVIDER");
        let llm = RealLlm::new();
        assert_eq!(llm.provider, Provider::Anthropic);
        assert_eq!(llm.model, "claude-sonnet-4-20250514");
        assert!(llm.api_key.is_none());
    }

    #[test]
    fn test_real_llm_with_config() {
        let llm = RealLlm::with_config(
            Provider::OpenAI,
            "gpt-4o-mini".to_string(),
            Some("sk-test".to_string()),
        );
        assert_eq!(llm.provider, Provider::OpenAI);
        assert_eq!(llm.model, "gpt-4o-mini");
        assert_eq!(llm.api_key, Some("sk-test".to_string()));
    }

    #[test]
    fn test_real_llm_anthropic_requires_key() {
        let llm = RealLlm::with_config(
            Provider::Anthropic,
            "claude-sonnet-4-20250514".to_string(),
            None,
        );
        let result = llm.call("test", "input");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("METALOGOS_API_KEY"));
    }

    #[test]
    fn test_real_llm_openai_requires_key() {
        let llm = RealLlm::with_config(Provider::OpenAI, "gpt-4o".to_string(), None);
        let result = llm.call("test", "input");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("METALOGOS_API_KEY"));
    }

    // ── Factory ────────────────────────────────────────────────────

    #[test]
    fn test_create_llm_backend_default_is_mock() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("METALOGOS_MOCK_LLM");
        let backend = create_llm_backend();
        assert_eq!(backend.call("prompt", "input").unwrap(), "prompt");
    }

    #[test]
    fn test_create_llm_backend_explicit_mock_true() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_MOCK_LLM", "true");
        let backend = create_llm_backend();
        assert_eq!(backend.call("prompt", "input").unwrap(), "prompt");
        env::remove_var("METALOGOS_MOCK_LLM");
    }

    #[test]
    fn test_create_llm_backend_explicit_mock_1() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_MOCK_LLM", "1");
        let backend = create_llm_backend();
        assert_eq!(backend.call("prompt", "input").unwrap(), "prompt");
        env::remove_var("METALOGOS_MOCK_LLM");
    }

    // ── resolve_model unit tests (ADR-0048) ──────────────────────────

    #[test]
    fn test_resolve_model_with_env_alias() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_LLM_MODEL_fast", "claude-haiku-4-5-20251001");
        assert_eq!(resolve_model("fast"), "claude-haiku-4-5-20251001");
        env::remove_var("METALOGOS_LLM_MODEL_fast");
    }

    #[test]
    fn test_resolve_model_without_env_passthrough() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("METALOGOS_LLM_MODEL_unknown");
        assert_eq!(resolve_model("unknown"), "unknown");
    }

    #[test]
    fn test_resolve_model_direct_model_name() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // "claude-sonnet-4-20250514" is a real model name, not an alias
        env::remove_var("METALOGOS_LLM_MODEL_claude-sonnet-4-20250514");
        assert_eq!(
            resolve_model("claude-sonnet-4-20250514"),
            "claude-sonnet-4-20250514"
        );
    }

    #[test]
    fn test_resolve_model_custom_user_alias() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_LLM_MODEL_cheap", "gpt-4o-mini");
        assert_eq!(resolve_model("cheap"), "gpt-4o-mini");
        env::remove_var("METALOGOS_LLM_MODEL_cheap");
    }

    #[test]
    fn test_resolve_model_env_changes_are_reflected() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_LLM_MODEL_volatile", "model-v1");
        assert_eq!(resolve_model("volatile"), "model-v1");
        env::set_var("METALOGOS_LLM_MODEL_volatile", "model-v2");
        assert_eq!(resolve_model("volatile"), "model-v2");
        env::remove_var("METALOGOS_LLM_MODEL_volatile");
    }

    // ── resolve_endpoint deduplication (Наряд №32) ────────────────────

    #[test]
    fn test_resolve_endpoint_base_without_trailing_path() {
        let mut llm = RealLlm::with_config(Provider::OpenAI, "gpt-4o".to_string(), None);
        // base_url = "https://myproxy.com/v1" (no /chat/completions)
        // default endpoint = "https://api.openai.com/v1/chat/completions"
        // path = "/chat/completions"
        // result should be "https://myproxy.com/v1/chat/completions"
        llm.base_url = Some("https://myproxy.com/v1".to_string());
        assert_eq!(
            llm.resolve_endpoint(),
            "https://myproxy.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_resolve_endpoint_base_with_full_path_no_dup() {
        let mut llm = RealLlm::with_config(Provider::OpenAI, "gpt-4o".to_string(), None);
        // base_url already contains /v1/chat/completions — should NOT double
        llm.base_url = Some("https://myproxy.com/v1/chat/completions".to_string());
        assert_eq!(
            llm.resolve_endpoint(),
            "https://myproxy.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_resolve_endpoint_no_base_url() {
        let mut llm = RealLlm::with_config(Provider::OpenAI, "gpt-4o".to_string(), None);
        llm.base_url = None;
        assert_eq!(
            llm.resolve_endpoint(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    // ── Integration Tests (require real API keys) ──────────────────

    #[test]
    #[ignore] // METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=openai METALOGOS_API_KEY=sk-xxx cargo test -- --ignored
    fn test_real_llm_openai_classify() {
        let api_key = env::var("METALOGOS_API_KEY").expect("METALOGOS_API_KEY must be set");
        let llm = RealLlm::with_config(
            Provider::OpenAI,
            env::var("METALOGOS_LLM_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            Some(api_key),
        );
        let result = llm.call(
            "Classify this message as one of: question | complaint | greeting | urgent. Return ONLY the category name.",
            "ваш сервис ужасен",
        );
        let response = result.expect("OpenAI LLM call should succeed");
        assert!(
            response.to_lowercase().contains("complaint"),
            "Expected 'complaint', got: {}",
            response
        );
    }

    #[test]
    #[ignore] // METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=anthropic METALOGOS_API_KEY=sk-ant-xxx cargo test -- --ignored
    fn test_real_llm_anthropic_classify() {
        let api_key = env::var("METALOGOS_API_KEY").expect("METALOGOS_API_KEY must be set");
        let llm = RealLlm::with_config(
            Provider::Anthropic,
            env::var("METALOGOS_LLM_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string()),
            Some(api_key),
        );
        let result = llm.call(
            "Classify this message as one of: question | complaint | greeting | urgent. Return ONLY the category name.",
            "ваш сервис ужасен",
        );
        let response = result.expect("Anthropic LLM call should succeed");
        assert!(
            response.to_lowercase().contains("complaint"),
            "Expected 'complaint', got: {}",
            response
        );
    }

    #[test]
    #[ignore] // METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=ollama cargo test -- --ignored
    fn test_real_llm_ollama_classify() {
        let llm = RealLlm::with_config(
            Provider::Ollama,
            env::var("METALOGOS_LLM_MODEL").unwrap_or_else(|_| "llama3".to_string()),
            None,
        );
        let result = llm.call(
            "Classify this message as one of: question | complaint | greeting | urgent. Return ONLY the category name.",
            "ваш сервис ужасен",
        );
        let response = result.expect("Ollama LLM call should succeed");
        assert!(
            response.to_lowercase().contains("complaint"),
            "Expected 'complaint', got: {}",
            response
        );
    }
}
