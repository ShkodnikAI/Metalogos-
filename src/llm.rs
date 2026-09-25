// ── LLM client abstraction for METALOGOS M3 ─────────────────────────
// Phase 7.1: Real LLM backends — Anthropic, OpenAI, Ollama.
// Mock mode for testing — ONLY when METALOGOS_MOCK_LLM=1|true is set
// explicitly (Наряд №454: the default is the real backend, which fails
// loudly without credentials; a silent mock-by-default leaked prompts).
// Retry with exponential backoff (3 retries, 1s/2s/4s). Timeout 120s.

use std::env;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

    /// Наряд №248: deadline-based request cancellation — the backend must
    /// either finish within `deadline` or fail loudly. Default delegates to
    /// `call_with_model` so existing backends stay compatible unchanged;
    /// backends that CAN honor a deadline override this (RealLlm drops the
    /// TCP connection at min(deadline, 120s); MockLlm sleeps
    /// min(delay, deadline) and errors loudly when the deadline is tighter
    /// than its artificial delay). This closes the README/REFERENCE
    /// "full request cancellation" promise on every call path:
    /// SmartRouter via client timeout (№156), legacy via this method.
    fn call_with_deadline(
        &self,
        prompt: &str,
        input: &str,
        model: Option<&str>,
        _deadline: Duration,
    ) -> Result<String, String> {
        self.call_with_model(prompt, input, model)
    }
}

/// Mock LLM backend for testing. Does NOT echo the prompt (Наряд №454):
/// the answer is the deterministic marker `"[mock-llm:<8 hex of prompt hash>]"`
/// — stable for tests, reveals nothing about the prompt (which may carry
/// instructions, recalled memory and conversation history).
///
/// ADR-0047: includes a static call counter for cache contract tests.
/// ADR-0048: records last model override for model-routing contract tests.
pub struct MockLlm;

/// Deterministic non-echo response of the mock backend (Наряд №454):
/// `[mock-llm:<8 hex from the hash of the prompt>]`.
/// `DefaultHasher::new()` has fixed keys — the hash is stable across runs.
pub fn mock_response(prompt: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    prompt.hash(&mut h);
    format!("[mock-llm:{:08x}]", h.finish() as u32)
}

/// Single source of truth for mock-LLM mode (Наряд №454): mock is active
/// ONLY when `METALOGOS_MOCK_LLM` is explicitly `1` or `true` (case
/// insensitive). Unset or any other value — the real backend, fail-loud.
/// Every site that used to re-read the variable with the old default-on
/// semantics must go through this predicate so the backend choice and the
/// trace/accuracy labeling can never disagree.
pub fn mock_llm_requested() -> bool {
    env::var("METALOGOS_MOCK_LLM")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Global call counter for MockLlm. Used by cache contract tests to verify
/// that identical LLM calls are served from cache (counter stays at 1 after
/// two identical invocations).
static MOCK_LLM_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

/// Global artificial delay for MockLlm (milliseconds). Used by timeout
/// contract tests (Наряд №126) to simulate a slow/hung LLM provider.
/// Default 0 = no delay.
static MOCK_LLM_DELAY_MS: AtomicU64 = AtomicU64::new(0);

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

    /// Set an artificial delay for all MockLlm calls (Наряд №126).
    /// Used by timeout contract tests to simulate a slow/hung LLM provider.
    pub fn set_delay_ms(ms: u64) {
        MOCK_LLM_DELAY_MS.store(ms, Ordering::SeqCst);
    }

    /// Reset the artificial delay to zero.
    pub fn reset_delay() {
        MOCK_LLM_DELAY_MS.store(0, Ordering::SeqCst);
    }
}

impl LlmBackend for MockLlm {
    fn call(&self, prompt: &str, _input: &str) -> Result<String, String> {
        // Наряд №126: apply artificial delay if set (for timeout tests)
        let delay_ms = MOCK_LLM_DELAY_MS.load(Ordering::SeqCst);
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
        MOCK_LLM_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        // Наряд №454: never echo the prompt back to the caller.
        Ok(mock_response(prompt))
    }

    fn call_with_model(
        &self,
        prompt: &str,
        _input: &str,
        model: Option<&str>,
    ) -> Result<String, String> {
        // Наряд №126: apply artificial delay if set (for timeout tests)
        let delay_ms = MOCK_LLM_DELAY_MS.load(Ordering::SeqCst);
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
        MOCK_LLM_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        // Record model for contract tests
        if let Some(m) = model {
            *lock_or_err(MOCK_LLM_LAST_MODEL.lock())? = m.to_string();
        } else {
            *lock_or_err(MOCK_LLM_LAST_MODEL.lock())? = String::new();
        }
        // Наряд №454: never echo the prompt back to the caller.
        Ok(mock_response(prompt))
    }

    /// Наряд №248: deadline-aware mock. Sleeps min(delay, deadline) so the
    /// caller never waits past the deadline; when the deadline is tighter
    /// than (or equal to) the artificial delay the call fails loudly with
    /// the legacy timeout wording. Existing tests keep calling
    /// `call`/`call_with_model`, which sleep the FULL delay — their n126
    /// semantics are preserved 1:1 (verified unchanged in naryad_126).
    /// NOTE (honesty, §3.5): MockLlm is a test-only backend — there is no
    /// real request to cancel, the sleep only SIMULATES provider latency;
    /// real on-the-wire cancellation is RealLlm's contract (TCP drop).
    fn call_with_deadline(
        &self,
        prompt: &str,
        input: &str,
        model: Option<&str>,
        deadline: Duration,
    ) -> Result<String, String> {
        let delay = Duration::from_millis(MOCK_LLM_DELAY_MS.load(Ordering::SeqCst));
        if deadline <= delay {
            // Deadline is tighter than the simulated latency — loud timeout,
            // same wording the legacy thread-wrapper produced (learnable.rs).
            // №385: stamped at the origin — the deadline DID fire.
            return Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_LLM_TIMEOUT,
                format!("LLM call timed out after {:?}", deadline),
            ));
        }
        // delay < deadline here: sleeping the full delay IS min(delay, deadline).
        if delay > Duration::ZERO {
            std::thread::sleep(delay);
        }
        self.call_with_model(prompt, input, model)
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
///
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

impl Default for RealLlm {
    fn default() -> Self {
        Self::new()
    }
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

    /// Наряд №248: deadline-based request cancellation (legacy path).
    /// Builds a one-shot client with timeout = min(deadline, 120s) plus a
    /// 10s connect timeout — the n156 pattern (SmartRouter path: effective
    /// timeout = min(override, config); llm.rs call_provider). reqwest
    /// performs real HTTP-level cancellation (drops the TCP connection)
    /// when the deadline fires — the request does NOT stay in flight.
    /// No retry loop here: the 1s/2s/4s backoff of `call` would inflate
    /// the total wait beyond the caller's deadline; a single attempt is
    /// the honest deadline contract (same as the n156 path).
    fn call_with_deadline(
        &self,
        prompt: &str,
        input: &str,
        model: Option<&str>,
        deadline: Duration,
    ) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(deadline.min(Duration::from_secs(120)))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client build error: {}", e))?;

        // Honor a per-call model override the same way call_with_model does.
        let target = match model {
            Some(m) if m != self.model => {
                let mut backend = self.clone();
                backend.model = m.to_string();
                backend
            }
            _ => self.clone(),
        };

        target.call_provider(&client, prompt, input).map_err(|e| {
            // №385: preserve the inner origin stamp (a stamped provider
            // failure keeps its code at the FRONT); an unstamped inner
            // error keeps the legacy timeout wording unstamped.
            crate::interpreter::values::wrap_error_preserving_code(
                &format!("LLM call timed out after {:?}", deadline),
                &e,
            )
        })
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
            let suffix = extract_endpoint_suffix(default);
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
            .map_err(|e| llm_send_error("Anthropic request failed", &e))?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e| llm_send_error("Anthropic response read error", &e))?;

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
            .map_err(|e| llm_send_error("OpenAI request failed", &e))?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e| llm_send_error("OpenAI response read error", &e))?;

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
                llm_send_error(
                    "Ollama request failed (is Ollama running at localhost:11434?)",
                    &e,
                )
            })?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e| llm_send_error("Ollama response read error", &e))?;

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
/// **Defaults to RealLlm, fail-loud** (Наряд №454, audit 25.09 finding 3.1):
/// the old mock-by-default silently answered every learnable/conversation
/// call with the echoed prompt (instructions + memory + history) and made
/// production deployments confidently wrong. Without credentials the real
/// backend now fails loudly at the first call. Tests and CI that need the
/// mock must set `METALOGOS_MOCK_LLM=1|true` explicitly — see
/// `mock_llm_requested()`.
pub fn create_llm_backend() -> Box<dyn LlmBackend> {
    if mock_llm_requested() {
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
            cache_hits_semantic: 0.0,
            canary_leaks: 0.0,
            providers: Vec::new(),
        }
    }
}

/// Global SmartRouter bridge (Наряд №4).
/// Set by Interpreter when it processes Declaration::LlmConfig.
/// Read by builtin_call_llm() to route through SmartRouter instead of legacy create_llm_backend().
/// SmartRouter is not Clone (contains Mutex<Instant> fields), so we wrap in Option.
pub static GLOBAL_SMART_ROUTER: once_cell::sync::Lazy<StdMutex<Option<SmartRouter>>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(None));

/// Install a SmartRouter into the global bridge.
/// Called from interpreter/execution.rs when Declaration::LlmConfig is processed.
pub fn set_global_smart_router(router: SmartRouter) {
    if let Ok(mut g) = GLOBAL_SMART_ROUTER.lock() {
        *g = Some(router);
    }
}

/// Remove the global SmartRouter (tests).
/// Without this, a router installed by one test leaks into every other
/// test in the same binary (process-global state).
pub fn clear_global_smart_router() {
    if let Ok(mut g) = GLOBAL_SMART_ROUTER.lock() {
        *g = None;
    }
}

/// Call LLM through the global SmartRouter if available, else return None.
/// The caller should fall back to legacy create_llm_backend() if this returns None.
/// Наряд #156: `timeout_override` passed through for real HTTP cancellation.
/// №385 (ADR-0169): stamp a reqwest send/read failure with the stable LLM
/// code AT THE ORIGIN, by the TYPED reqwest error kind — never by message
/// text: `is_timeout()` → `LLM_TIMEOUT` (deadline / provider timeout),
/// `is_connect()` → `LLM_PROVIDER_UNAVAILABLE` (connect failure). Any other
/// transport failure stays UNSTAMPED — the provider state is unknown, so the
/// honest classification is the `RUNTIME_ERROR` fallback, not a guess.
/// The message text after the stamp is unchanged.
pub(crate) fn llm_send_error(ctx: &str, e: &reqwest::Error) -> String {
    use crate::interpreter::values::{
        coded_error, CODE_LLM_PROVIDER_UNAVAILABLE, CODE_LLM_TIMEOUT,
    };
    let text = format!("{}: {}", ctx, e);
    if e.is_timeout() {
        coded_error(CODE_LLM_TIMEOUT, text)
    } else if e.is_connect() {
        coded_error(CODE_LLM_PROVIDER_UNAVAILABLE, text)
    } else {
        text
    }
}

pub fn call_via_smart_router(
    prompt: &str,
    input: &str,
    model_override: Option<&str>,
    timeout_override: Option<Duration>,
) -> Option<Result<String, String>> {
    if let Ok(g) = GLOBAL_SMART_ROUTER.lock() {
        if let Some(ref router) = *g {
            return Some(router.call(prompt, input, model_override, timeout_override));
        }
    }
    None
}

// ── Per-call LLM traces (Наряд №276, ADR-0138) ────────────────────
// METALOGOS_LLM_TRACE=<path>: every LLM call appends ONE JSONL line with
// OpenTelemetry GenAI semantic-conventions field names, verified against the
// live spec (open-telemetry/semantic-conventions-genai,
// docs/gen-ai/gen-ai-spans.md, base semantic-conventions v1.44.0,
// checked 2026-09-12):
//   `gen_ai.provider.name`  — NOTE: the issue text said `gen_ai.system`;
//       the upstream spec has RENAMED it — the current attribute is
//       `gen_ai.provider.name` (Required). We follow the live spec: the
//       whole point is that a future exporter reads the file WITHOUT
//       renames.
//   `gen_ai.request.model`, `gen_ai.usage.input_tokens`,
//   `gen_ai.usage.output_tokens` — unchanged in the live spec.
// Honest data: a field the provider did not report is OMITTED, never
// invented (e.g. mock and legacy-backend calls carry no token counts).

// Per-thread backend tag for the `backend` trace field.
// "tw" (tree-walking interpreter) by default; VM entry points set "vm"
// for the duration of a program run and restore the previous value.
// (thread_local! — rustdoc does not render docs on macro invocations, hence
// the plain comments instead of ///.)
thread_local! {
    static BACKEND_TAG: std::cell::Cell<&'static str> = const { std::cell::Cell::new("tw") };
}

/// Set the backend tag for LLM traces on the current thread ("tw" | "vm").
/// Returns the previous tag so the caller (VM entry points) can restore it.
pub fn set_llm_backend_tag(tag: &'static str) -> &'static str {
    BACKEND_TAG.with(|b| b.replace(tag))
}

/// One LLM call event for the JSONL trace (Наряд №276).
/// `None` fields are omitted from the line — honest data, not defaults.
pub struct LlmTraceEvent<'a> {
    /// `gen_ai.provider.name` (live-spec name; see module comment).
    pub provider_name: Option<&'a str>,
    /// `gen_ai.request.model`.
    pub model: Option<&'a str>,
    /// `gen_ai.usage.input_tokens` — only when the provider reported it.
    pub input_tokens: Option<u64>,
    /// `gen_ai.usage.output_tokens` — only when the provider reported it.
    pub output_tokens: Option<u64>,
    pub latency_ms: u64,
    /// "ok" | "error".
    pub status: &'static str,
    /// "exact" (ADR-0047 cache hit) | "semantic" (№273, future) | "miss".
    pub cache: &'static str,
    /// SmartRouter provider alias, when the call went through a router.
    pub provider_alias: Option<&'a str>,
}

static TRACE_WARNED: AtomicBool = AtomicBool::new(false);

fn trace_warn_once(msg: &str) {
    if !TRACE_WARNED.swap(true, Ordering::SeqCst) {
        eprintln!(
            "[llm-trace] warning: {} (further trace-write warnings suppressed)",
            msg
        );
    }
}

/// Append one LLM call event to the METALOGOS_LLM_TRACE file (JSONL).
/// Overhead when tracing is off: exactly ONE env-check per call (the
/// documented contract — ADR-0138). Append+flush per line: a crash loses
/// nothing already written (speed traded for survivability, on purpose).
/// Write errors NEVER fail the LLM call — one warning, then silence.
/// No rotation in v1: the file grows; the operator rotates it (documented).
pub fn trace_llm_call(evt: &LlmTraceEvent) {
    let Ok(path) = env::var("METALOGOS_LLM_TRACE") else {
        return;
    };
    if path.is_empty() {
        return;
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    // v1 operation: every current call site is a chat-style completion.
    // Span-name convention of the spec ("{gen_ai.operation.name}") maps
    // to the `name` field as "gen_ai.<operation>" = "gen_ai.chat".
    let mut line = serde_json::json!({
        "ts": ts,
        "name": "gen_ai.chat",
        "latency_ms": evt.latency_ms,
        "status": evt.status,
        "cache": evt.cache,
        "backend": BACKEND_TAG.with(|b| b.get()),
    });
    if let Some(p) = evt.provider_name {
        line["gen_ai.provider.name"] = serde_json::json!(p);
    }
    if let Some(m) = evt.model {
        line["gen_ai.request.model"] = serde_json::json!(m);
    }
    if let Some(t) = evt.input_tokens {
        line["gen_ai.usage.input_tokens"] = serde_json::json!(t);
    }
    if let Some(t) = evt.output_tokens {
        line["gen_ai.usage.output_tokens"] = serde_json::json!(t);
    }
    if let Some(a) = evt.provider_alias {
        line["provider_alias"] = serde_json::json!(a);
    }
    let mut out = line.to_string();
    out.push('\n');
    let res = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(out.as_bytes()).and_then(|()| f.flush())
        });
    if let Err(e) = res {
        trace_warn_once(&format!("cannot append LLM trace to {}: {}", path, e));
    }
}

/// Provider name from env for legacy (non-router) traces: the value the
/// live spec expects for `gen_ai.provider.name` ("anthropic" / "openai" /
/// "ollama" — well-known values; "ollama" is an honest custom value).
pub fn provider_env_name() -> &'static str {
    match Provider::from_env() {
        Provider::OpenAI => "openai",
        Provider::Ollama => "ollama",
        Provider::Anthropic => "anthropic",
    }
}

/// Token usage extracted from a raw provider response (Наряд №276).
/// Best-effort by design: parse failures yield None — an unparseable
/// response must not break the call, and an absent usage must not be
/// invented (honest data).
#[derive(Debug, Clone, Copy, Default)]
pub struct ProviderTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

fn extract_provider_usage(provider_type: &str, raw: &str) -> Option<ProviderTokenUsage> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let u = v.get("usage")?;
    let (i, o) = match provider_type {
        "anthropic" => (u.get("input_tokens")?, u.get("output_tokens")?),
        "ollama" => (u.get("prompt_eval_count")?, u.get("eval_count")?),
        // OpenAI-compatible: openai, groq, cerebras, nvidia, openrouter, custom
        _ => (u.get("prompt_tokens")?, u.get("completion_tokens")?),
    };
    Some(ProviderTokenUsage {
        input_tokens: i.as_u64()?,
        output_tokens: o.as_u64()?,
    })
}

/// Usage from an ALREADY-parsed Anthropic response body (call_claude reuses
/// the one parse it already did for content extraction — №276).
pub(crate) fn extract_usage_from_anthropic_body(
    parsed: &serde_json::Value,
) -> Option<ProviderTokenUsage> {
    let u = parsed.get("usage")?;
    Some(ProviderTokenUsage {
        input_tokens: u.get("input_tokens")?.as_u64()?,
        output_tokens: u.get("output_tokens")?.as_u64()?,
    })
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
            cache_hits_semantic: CACHE_HITS_SEMANTIC.load(std::sync::atomic::Ordering::Relaxed)
                as f64,
            canary_leaks: CANARY_LEAKS.load(std::sync::atomic::Ordering::Relaxed) as f64,
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
    /// Наряд №273: semantic cache hits (cosine ≥ threshold, ADR-0135).
    pub cache_hits_semantic: f64,
    /// Наряд №284: confirmed canary leaks (compromised LLM channel).
    pub canary_leaks: f64,
    pub providers: Vec<ProviderUsage>,
}

/// Наряд №273 (ADR-0135): global semantic-cache hit counter — observed
/// via llm_usage().cache_hits_semantic (exact hits are counted by the
/// existing total_calls-exempt exact path of ADR-0047; the semantic hit
/// gets its own counter for honest observability).
pub static CACHE_HITS_SEMANTIC: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Record one semantic cache hit (learnable cache_semantic contour).
pub fn record_cache_hit_semantic() {
    CACHE_HITS_SEMANTIC.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Наряд №284 (P1, M1): global canary-leak counter — observed via
/// llm_usage().canary_leaks (runtime detector of untrusted-content
/// exfiltration through the LLM channel; the static half — the
/// compromised-channel taint label in the leak branch — lives in
/// src/audit.rs, check_canary_leak). Detector, not gate.
pub static CANARY_LEAKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Record one canary leak (canary_check detected the marker in the response).
pub fn record_canary_leak() {
    CANARY_LEAKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                    crate::ast::Expr::FnCall { name, args, .. } if name == "env" => {
                        args.first().and_then(|a| {
                            if let crate::ast::Expr::StringLit { value: s, .. } = a {
                                std::env::var(s).ok()
                            } else {
                                None
                            }
                        })
                    }
                    crate::ast::Expr::StringLit { value: s, .. } => Some(s.clone()),
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
    ///
    /// Наряд #156 Block 1 (real cancellation):
    /// `timeout_override` allows passing a per-call timeout (e.g. sandbox
    /// timeout). Uses `min(timeout_override, self.timeout)` — the
    /// effective value is fed to `reqwest::blocking::Client::timeout()`
    /// which performs real HTTP-level cancellation (drops the TCP
    /// connection). This replaces the former thread-based approach
    /// (Наряд №126) for the SmartRouter path.
    pub fn call(
        &self,
        prompt: &str,
        input: &str,
        model_override: Option<&str>,
        timeout_override: Option<Duration>,
    ) -> Result<String, String> {
        let call_start = Instant::now();
        if self.providers.is_empty() {
            // No providers configured — fall back to legacy behavior.
            // The legacy backend's identity is not visible here (type-erased
            // trait object), so the trace honestly carries no provider fields.
            let backend = create_llm_backend();
            let res = backend.call(prompt, input);
            trace_llm_call(&LlmTraceEvent {
                provider_name: None,
                model: None,
                input_tokens: None,
                output_tokens: None,
                latency_ms: call_start.elapsed().as_millis() as u64,
                status: if res.is_ok() { "ok" } else { "error" },
                cache: "miss",
                provider_alias: None,
            });
            return res;
        }

        let effective_prompt_len = prompt.len() + input.len();
        // Resolved once here (was inside call_provider) so the trace can
        // name the requested model without re-deriving it.
        let resolved_model = model_override
            .or(self.default_model.as_deref())
            .unwrap_or("default");

        // Build ordered list of provider indices, sorted by health_score desc
        let mut candidates: Vec<usize> = (0..self.providers.len()).collect();
        candidates.sort_by(|&a, &b| {
            let sa = self.tracker.health_score(a);
            let sb = self.tracker.health_score(b);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut last_error = String::new();
        // Last attempted provider type (for the final error trace).
        let mut last_attempted: Option<String> = None;

        for &idx in &candidates {
            if !self.tracker.is_provider_available(idx) {
                continue; // circuit breaker open — skip
            }

            let (ref _alias, ref provider_type, ref api_key, ref url) = self.providers[idx];
            let start = Instant::now();
            last_attempted = Some(provider_type.clone());

            let result = self.call_provider(
                provider_type,
                api_key.as_deref(),
                url.as_deref(),
                prompt,
                input,
                resolved_model,
                timeout_override,
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
                Ok((response, usage)) => {
                    trace_llm_call(&LlmTraceEvent {
                        provider_name: Some(provider_type.as_str()),
                        model: Some(resolved_model),
                        input_tokens: usage.map(|u| u.input_tokens),
                        output_tokens: usage.map(|u| u.output_tokens),
                        latency_ms: call_start.elapsed().as_millis() as u64,
                        status: "ok",
                        cache: "miss",
                        provider_alias: Some(_alias.as_str()),
                    });
                    return Ok(response);
                }
                Err(e) => {
                    last_error = e.clone();
                    if !self.failover {
                        break; // manual mode — don't try next provider
                    }
                    // Continue to next provider (failover)
                }
            }
        }

        // All providers exhausted — soft failure.
        // The trace names the LAST attempted provider (the one that
        // produced the final error) — honest about what was tried last.
        trace_llm_call(&LlmTraceEvent {
            provider_name: last_attempted.as_deref(),
            model: Some(resolved_model),
            input_tokens: None,
            output_tokens: None,
            latency_ms: call_start.elapsed().as_millis() as u64,
            status: "error",
            cache: "miss",
            provider_alias: None,
        });
        // №385 (ADR-0169): classify the exhaustion by what ACTUALLY happened.
        // No rung attempted (every circuit open) → the provider surface is
        // unavailable: stamp LLM_PROVIDER_UNAVAILABLE at the origin. Rungs
        // attempted → the last error keeps its own origin stamp (promoted to
        // the front — wrapper text must not bury the code); an unstamped last
        // error (e.g. an HTTP status refusal) stays unstamped → the honest
        // RUNTIME_ERROR fallback, not a guess.
        if last_attempted.is_none() {
            return Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_LLM_PROVIDER_UNAVAILABLE,
                format!(
                    "All LLM providers failed. Last error: {}",
                    truncate(&last_error, 200)
                ),
            ));
        }
        if let Some((code, rest)) = crate::interpreter::values::split_origin_stamp(&last_error) {
            return Err(crate::interpreter::values::coded_error(
                code,
                format!(
                    "All LLM providers failed. Last error: {}",
                    truncate(rest, 200)
                ),
            ));
        }
        Err(format!(
            "All LLM providers failed. Last error: {}",
            truncate(&last_error, 200)
        ))
    }

    /// Make a single provider call using the appropriate format.
    /// Наряд #156: effective timeout = min(timeout_override, self.timeout).
    /// `reqwest::blocking::Client::timeout()` performs real HTTP-level
    /// cancellation (drops TCP connection) when it fires.
    /// Наряд №276: returns `(text, usage)` — usage extracted from the raw
    /// response when the provider reported it (honest: None otherwise).
    #[allow(clippy::too_many_arguments)]
    fn call_provider(
        &self,
        provider_type: &str,
        api_key: Option<&str>,
        url: Option<&str>,
        prompt: &str,
        input: &str,
        resolved_model: &str,
        timeout_override: Option<Duration>,
    ) -> Result<(String, Option<ProviderTokenUsage>), String> {
        let effective_timeout = match timeout_override {
            Some(override_dur) => {
                let config_dur = Duration::from_secs(self.timeout.max(5) as u64);
                override_dur.min(config_dur)
            }
            None => Duration::from_secs(self.timeout.max(5) as u64),
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(effective_timeout)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client build error: {}", e))?;

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
                    .map_err(|e| llm_send_error("Anthropic request failed", &e))?;
                let status = resp.status();
                let text = resp
                    .text()
                    .map_err(|e| llm_send_error("Response read error", &e))?;
                if !status.is_success() {
                    return Err(format!(
                        "Anthropic API error ({}): {}",
                        status.as_u16(),
                        truncate(&text, 500)
                    ));
                }
                let usage = extract_provider_usage(provider_type, &text);
                parse_anthropic_response(&text).map(|s| (s, usage))
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
                    .map_err(|e| llm_send_error("Ollama request failed", &e))?;
                let status = resp.status();
                let text = resp
                    .text()
                    .map_err(|e| llm_send_error("Response read error", &e))?;
                if !status.is_success() {
                    return Err(format!(
                        "Ollama API error ({}): {}",
                        status.as_u16(),
                        truncate(&text, 500)
                    ));
                }
                let usage = extract_provider_usage(provider_type, &text);
                parse_ollama_response(&text).map(|s| (s, usage))
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
                let resp = req.send().map_err(|e| {
                    llm_send_error(&format!("{} request failed", provider_type), &e)
                })?;
                let status = resp.status();
                let text = resp
                    .text()
                    .map_err(|e| llm_send_error("Response read error", &e))?;
                if !status.is_success() {
                    return Err(format!(
                        "{} API error ({}): {}",
                        provider_type,
                        status.as_u16(),
                        truncate(&text, 500)
                    ));
                }
                let usage = extract_provider_usage(provider_type, &text);
                parse_openai_response(&text).map(|s| (s, usage))
            }
        }
    }

    /// Resolve the endpoint URL for a provider type.
    /// Наряд №4 fix: reuse RealLlm's deduplication logic (Наряд №32).
    /// Uses extract_endpoint_suffix() to avoid doubling path segments
    /// when the custom URL already contains the full endpoint path.
    fn resolve_endpoint(&self, provider_type: &str, url: Option<&str>) -> String {
        if let Some(u) = url {
            let u = u.trim_end_matches('/');
            // Get the default endpoint for this provider type
            let default_endpoint = match provider_type {
                "anthropic" => "https://api.anthropic.com/v1/messages",
                "openai" => "https://api.openai.com/v1/chat/completions",
                "ollama" => "http://localhost:11434/api/generate",
                "groq" => "https://api.groq.com/openai/v1/chat/completions",
                "cerebras" => "https://api.cerebras.ai/v1/chat/completions",
                "nvidia" => "https://integrate.api.nvidia.com/v1/chat/completions",
                "openrouter" => "https://openrouter.ai/api/v1/chat/completions",
                "google" => {
                    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
                }
                _ => "https://api.openai.com/v1/chat/completions",
            };
            // Наряд №32 dedup: strip versioned prefix, get suffix only
            let suffix = extract_endpoint_suffix(default_endpoint);
            if u.ends_with(suffix) {
                u.to_string()
            } else {
                format!("{}{}", u, suffix)
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
pub fn resolve_model_smart(alias: &str, _config: Option<&crate::ast::LlmConfigDecl>) -> String {
    // Check env override first
    let env_key = format!("METALOGOS_LLM_MODEL_{}", alias);
    if let Ok(val) = env::var(&env_key) {
        return val;
    }
    // If no env override, return as-is
    alias.to_string()
}

// ── LLM streaming (Наряд №275, ADR-0137) ──────────────────────────────
//
// Streaming sits OVER SmartRouter (ADR-0048) — `stream_open` reuses the
// same candidate-selection / circuit-breaker / resolved-model machinery
// as `SmartRouter::call`, but issues the POST with `"stream": true` and
// returns an opaque handle (`LlmStreamId`) into `LLM_STREAM_REGISTRY`.
//
// Spike verdict (docs/research/naryad-275-streaming-spike.md, ADR-0137 §D3):
// `reqwest::blocking::Response` has NO `chunk()` method (that is the
// async `reqwest::Response` API). Instead, `Response: std::io::Read`
// gives incremental blocking reads — `read(&mut buf)` returns as soon as
// the TCP buffer yields any bytes, then we parse one SSE delta from the
// line buffer. Semantically equivalent to "incremental chunked SSE
// reading", satisfies the spirit of the Go-criterion ("without rewriting
// backends, without async / tasks / callbacks", ADR-0096-compatible).
//
// Trace contract (ADR-0138 §D4): one line per COMPLETED stream — never
// per chunk. `llm_stream_close` writes a single `trace_llm_call` line
// with aggregated usage (final SSE event) + full-stream latency.

/// Opaque handle to an open LLM stream — `u32` index into
/// `LLM_STREAM_REGISTRY` (ADR-0137 §D2, leкало `ReflexId`/`VisionId`).
/// Weights, response body, and SSE-internal state never enter `Value` —
/// the handle carries only the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LlmStreamId(pub u32);

impl std::fmt::Display for LlmStreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[LlmStream#{}]", self.0)
    }
}

/// End-of-stream sentinel returned by `llm_stream_next` when the stream
/// has been fully consumed. The caller should call `llm_stream_close`
/// upon receiving this marker (ADR-0137 §D1). Keep-alive pings from
/// the provider are returned as empty strings — they are NOT the
/// end-of-stream marker.
pub const LLM_STREAM_END_MARKER: &str = "__end__";

/// Default upper bound on simultaneously-open streams (ADR-0137 §D6,
/// lesson from №263 — maps without bounds leak). `METALOGOS_LLM_STREAM_MAX`
/// env var overrides; invalid values fall back to the default.
pub const LLM_STREAM_DEFAULT_MAX: u32 = 64;

fn llm_stream_max() -> u32 {
    env::var("METALOGOS_LLM_STREAM_MAX")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(LLM_STREAM_DEFAULT_MAX)
}

/// One open LLM stream. Held in `LLM_STREAM_REGISTRY` behind a `Mutex`.
///
/// `LlmStreamState` owns:
/// - the `reqwest::blocking::Response` (so `Read::read` can pull more
///   bytes from the TCP buffer on each `next`),
/// - an incremental SSE line buffer + parser,
/// - provenance (provider_type, alias, resolved_model) for trace +
///   final-metadata return,
/// - aggregated usage (filled by the final SSE event),
/// - bookkeeping (started_at, status, ended flag).
pub struct LlmStreamState {
    /// Provider type (anthropic/openai/groq/.../ollama). Drives the
    /// SSE-event-shape parser.
    provider_type: String,
    /// SmartRouter provider alias (for the trace `provider_alias` field).
    provider_alias: String,
    /// Resolved model name (for the trace `gen_ai.request.model` field).
    resolved_model: String,
    /// The blocking HTTP response — we read from it incrementally.
    response: reqwest::blocking::Response,
    /// Incremental line buffer: bytes pulled from `Read::read` that
    /// haven't yet formed a complete SSE event.
    line_buf: Vec<u8>,
    /// Aggregated delta text (for equivalence test against `call_llm`).
    aggregated_text: String,
    /// Aggregated usage from the final SSE event (Anthropic `message_delta`,
    /// OpenAI final chunk with `stream_options.include_usage`, Ollama
    /// per-chunk `eval_count`). `None` until the final event arrives.
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    /// When `stream_open` was called — for `latency_ms` in the trace.
    started_at: Instant,
    /// Set to true once the SSE stream reported end-of-stream (final
    /// `data: [DONE]` / `message_stop` / ollama `"done": true`). Subsequent
    /// `next` calls return `LLM_STREAM_END_MARKER` without touching the
    /// network.
    ended: bool,
    /// Final status written to the trace: "ok" if the stream completed
    /// cleanly, "error" if the network broke or a non-2xx was returned.
    /// Initial value "ok" — flipped to "error" on read failure.
    status: &'static str,
}

impl std::fmt::Debug for LlmStreamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Mirror Reflex/Vision Debug discipline: provider + model only,
        // never the response body (leaks user prompts / PII).
        f.debug_struct("LlmStreamState")
            .field("provider", &self.provider_type)
            .field("model", &self.resolved_model)
            .field("ended", &self.ended)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

/// Final metadata returned by `llm_stream_close` (ADR-0137 §D1).
pub struct LlmStreamFinal {
    pub provider: String,
    pub model: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub latency_ms: u64,
    pub status: &'static str,
    /// Aggregated delta text — equivalent to the single-shot `call_llm`
    /// response. The equivalence test compares this against the same
    /// prompt+input fed to non-streaming `call_llm`.
    pub aggregated_text: String,
}

/// Process-global registry of open LLM streams (ADR-0137 §D2 / §D6).
///
/// Bounded by `llm_stream_max()` (default 64). Insertion when full →
/// `STREAM_LIMIT_REACHED` (loud; lesson №263). `stream_close` removes
/// the entry; `stream_next` borrows mutably through the registry mutex.
pub static LLM_STREAM_REGISTRY: once_cell::sync::Lazy<StdMutex<LlmStreamRegistry>> =
    once_cell::sync::Lazy::new(|| StdMutex::new(LlmStreamRegistry::new()));

pub struct LlmStreamRegistry {
    streams: std::collections::HashMap<u32, LlmStreamState>,
    next_id: u32,
}

impl LlmStreamRegistry {
    pub fn new() -> Self {
        Self {
            streams: std::collections::HashMap::new(),
            next_id: 0,
        }
    }

    /// Insert a new stream, return its handle. Loud error if the bound
    /// (ADR-0137 §D6) is exceeded.
    fn insert(&mut self, state: LlmStreamState) -> Result<LlmStreamId, String> {
        let max = llm_stream_max();
        if (self.streams.len() as u32) >= max {
            return Err(format!(
                "llm_stream_open(): STREAM_LIMIT_REACHED — {} streams open (max={}, override via METALOGOS_LLM_STREAM_MAX)",
                self.streams.len(),
                max
            ));
        }
        let id = LlmStreamId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        self.streams.insert(id.0, state);
        Ok(id)
    }

    /// Borrow a stream mutably for `next` — keeps ownership in registry.
    fn with_mut<R>(
        &mut self,
        id: LlmStreamId,
        f: impl FnOnce(&mut LlmStreamState) -> R,
    ) -> Option<R> {
        self.streams.get_mut(&id.0).map(f)
    }

    /// Take a stream out for `close` — drops the response, returns the
    /// aggregated final metadata.
    fn take(&mut self, id: LlmStreamId) -> Option<LlmStreamState> {
        self.streams.remove(&id.0)
    }

    /// Number of open streams (for tests + diagnostics).
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }
}

impl Default for LlmStreamRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Test-only: drop ALL open streams (used by `clear_global_smart_router`'s
/// tests so a leak in one test does not poison the next). Production code
/// must call `stream_close` per stream.
///
/// Not gated by `#[cfg(test)]`: integration tests (a separate crate) would
/// not see the item otherwise — Rust's `cfg(test)` is per-crate.
pub fn clear_llm_stream_registry_for_tests() {
    if let Ok(mut g) = LLM_STREAM_REGISTRY.lock() {
        g.streams.clear();
    }
}

// ── SmartRouter streaming surface (ADR-0137 §D4) ────────────────────

impl SmartRouter {
    /// Open a streaming LLM call. Reuses the same candidate-selection
    /// logic as `SmartRouter::call` (circuit-breaker, failover on open,
    /// health-score sorting), but issues the POST with `"stream": true`
    /// (OpenAI/Anthropic) or `"stream": true` (Ollama default).
    ///
    /// Failover is **on open only** (ADR-0137 §D5) — once a stream is
    /// open and the provider dies mid-stream, `next` returns an error
    /// and the user must `close` + `open` again. The circuit breaker
    /// will mark the provider sick so the next `open` skips it.
    ///
    /// `stream: true` in the body — JSON shape identical to `call_provider`
    /// except `stream: true`. Anthropic native format; OpenAI-compatible
    /// (`/v1/chat/completions`) takes `stream: true`; ollama native
    /// (`/api/generate`) defaults to `stream: true` already, but we
    /// send it explicitly for parity.
    pub fn stream_open(
        &self,
        prompt: &str,
        input: &str,
        model_override: Option<&str>,
        timeout_override: Option<Duration>,
    ) -> Result<LlmStreamState, String> {
        if self.providers.is_empty() {
            // No providers configured — cannot stream (mock / legacy path
            // does not support streaming). Issue contract: STREAM_UNSUPPORTED,
            // not silent full-answer.
            return Err(
                "llm_stream_open(): STREAM_UNSUPPORTED — no llm {} providers configured; \
                 the mock does not stream — configure llm { providers: [...] } for real streaming"
                    .to_string(),
            );
        }

        let resolved_model = model_override
            .or(self.default_model.as_deref())
            .unwrap_or("default");

        // Build ordered candidate list, sorted by health_score desc —
        // identical logic to `SmartRouter::call`.
        let mut candidates: Vec<usize> = (0..self.providers.len()).collect();
        candidates.sort_by(|&a, &b| {
            let sa = self.tracker.health_score(a);
            let sb = self.tracker.health_score(b);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut last_error = String::new();
        let body_text = format!("{}\n\nInput: {}", prompt, input);

        for &idx in &candidates {
            if !self.tracker.is_provider_available(idx) {
                continue; // circuit breaker open — skip
            }
            let (ref alias, ref provider_type, ref api_key, ref url) = self.providers[idx];
            match self.stream_open_provider(
                provider_type,
                api_key.as_deref(),
                url.as_deref(),
                &body_text,
                resolved_model,
                timeout_override,
            ) {
                Ok(state) => {
                    // Record a successful open — health tracking sees this
                    // as a call that started; if the user later closes
                    // with an error, the next `call`/`stream_open` will
                    // record the failure. For now, optimistic.
                    return Ok(LlmStreamState {
                        provider_type: provider_type.clone(),
                        provider_alias: alias.clone(),
                        resolved_model: resolved_model.to_string(),
                        response: state,
                        line_buf: Vec::with_capacity(8192),
                        aggregated_text: String::with_capacity(8192),
                        input_tokens: None,
                        output_tokens: None,
                        started_at: Instant::now(),
                        ended: false,
                        status: "ok",
                    });
                }
                Err(e) => {
                    last_error = e;
                    if !self.failover {
                        break; // manual mode — don't try next provider
                    }
                    // failover=auto: try next provider on open error
                }
            }
        }

        // All providers failed on open.
        Err(format!(
            "llm_stream_open(): all providers failed on open. Last error: {}",
            truncate(&last_error, 200)
        ))
    }

    /// Open one provider's stream — issues the POST, returns the blocking
    /// Response on 2xx. The body is shaped per provider, identical to
    /// `SmartRouter::call_provider` but with `stream: true`.
    #[allow(clippy::too_many_arguments)]
    fn stream_open_provider(
        &self,
        provider_type: &str,
        api_key: Option<&str>,
        url: Option<&str>,
        body_text: &str,
        resolved_model: &str,
        timeout_override: Option<Duration>,
    ) -> Result<reqwest::blocking::Response, String> {
        let effective_timeout = match timeout_override {
            Some(override_dur) => {
                let config_dur = Duration::from_secs(self.timeout.max(5) as u64);
                override_dur.min(config_dur)
            }
            None => Duration::from_secs(self.timeout.max(5) as u64),
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(effective_timeout)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client build error: {}", e))?;

        let endpoint = self.resolve_endpoint(provider_type, url);

        match provider_type {
            "anthropic" => {
                let key = api_key.ok_or_else(|| "anthropic requires an API key".to_string())?;
                let body = serde_json::json!({
                    "model": resolved_model,
                    "max_tokens": 1024,
                    "stream": true,
                    "messages": [{ "role": "user", "content": body_text }]
                });
                let resp = client
                    .post(&endpoint)
                    .header("x-api-key", key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .header("accept", "text/event-stream")
                    .json(&body)
                    .send()
                    .map_err(|e| llm_send_error("Anthropic stream open failed", &e))?;
                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    return Err(format!(
                        "Anthropic stream open error ({}): {}",
                        status,
                        truncate(&text, 500)
                    ));
                }
                Ok(resp)
            }
            "ollama" => {
                let body = serde_json::json!({
                    "model": resolved_model,
                    "prompt": body_text,
                    "stream": true
                });
                let resp = client
                    .post(&endpoint)
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .map_err(|e| llm_send_error("Ollama stream open failed", &e))?;
                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    return Err(format!(
                        "Ollama stream open error ({}): {}",
                        status,
                        truncate(&text, 500)
                    ));
                }
                Ok(resp)
            }
            _ => {
                // OpenAI-compatible: openai, groq, cerebras, nvidia,
                // openrouter, google, custom.
                let body = serde_json::json!({
                    "model": resolved_model,
                    "messages": [{ "role": "user", "content": body_text }],
                    "max_tokens": 1024,
                    "temperature": 0.0,
                    "stream": true
                });
                let mut req = client
                    .post(&endpoint)
                    .header("content-type", "application/json")
                    .header("accept", "text/event-stream")
                    .json(&body);
                if let Some(k) = api_key {
                    req = req.header("Authorization", format!("Bearer {}", k));
                }
                let resp = req.send().map_err(|e| {
                    llm_send_error(&format!("{} stream open failed", provider_type), &e)
                })?;
                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    return Err(format!(
                        "{} stream open error ({}): {}",
                        provider_type,
                        status,
                        truncate(&text, 500)
                    ));
                }
                Ok(resp)
            }
        }
    }
}

// ── Public streaming API (builtins call these) ───────────────────────

/// `llm_stream_open` body — call via global SmartRouter (leкало
/// `call_via_smart_router`).
pub fn stream_via_smart_router(
    prompt: &str,
    input: &str,
    model_override: Option<&str>,
    timeout_override: Option<Duration>,
) -> Result<LlmStreamId, String> {
    let Ok(g) = GLOBAL_SMART_ROUTER.lock() else {
        return Err("llm_stream_open(): SmartRouter mutex poisoned".to_string());
    };
    let Some(ref router) = *g else {
        return Err(
            "llm_stream_open(): STREAM_UNSUPPORTED — no llm {} providers configured \
             (no global SmartRouter); the mock does not stream — configure llm { providers: [...] } for real streaming"
                .to_string(),
        );
    };
    let state = router.stream_open(prompt, input, model_override, timeout_override)?;
    let Ok(mut reg) = LLM_STREAM_REGISTRY.lock() else {
        return Err("llm_stream_open(): stream registry mutex poisoned".to_string());
    };
    reg.insert(state)
}

/// `llm_stream_next(handle)` — one blocking `Read::read` + parse one
/// SSE delta. Returns the delta text, `""` for keep-alive ping, or
/// `LLM_STREAM_END_MARKER` (`"__end__"`) when the stream is fully
/// consumed (ADR-0137 §D1 / §D3).
pub fn stream_next(handle: LlmStreamId) -> Result<String, String> {
    let Ok(mut reg) = LLM_STREAM_REGISTRY.lock() else {
        return Err("llm_stream_next(): stream registry mutex poisoned".to_string());
    };
    reg.with_mut(handle, stream_next_inner)
        .ok_or_else(|| format!("llm_stream_next(): unknown LlmStream handle #{}", handle.0))?
}

/// Inner logic — separated so it can be unit-tested without touching the
/// global registry.
fn stream_next_inner(state: &mut LlmStreamState) -> Result<String, String> {
    if state.ended {
        return Ok(LLM_STREAM_END_MARKER.to_string());
    }
    // Read loop: pull bytes from the response into `line_buf`, then try
    // to parse one complete SSE event. If `line_buf` does not yet contain
    // a full event, keep reading (one `Read::read` per iteration — every
    // iteration yields control as soon as bytes arrive).
    let mut buf = [0u8; 8192];
    loop {
        if let Some(delta) = try_parse_one_event(state)? {
            state.aggregated_text.push_str(&delta);
            return Ok(delta);
        }
        if state.ended {
            return Ok(LLM_STREAM_END_MARKER.to_string());
        }
        // `Read::read` blocks the calling thread until the TCP buffer
        // yields at least one byte (or returns 0 = EOF / connection
        // closed). ADR-0096: blocking here is safe — the route handler
        // is on a `spawn_blocking` thread, not on a tokio worker.
        let n = std::io::Read::read(&mut state.response, &mut buf).map_err(|e| {
            state.status = "error";
            format!("llm_stream_next(): read error: {}", e)
        })?;
        if n == 0 {
            // EOF — provider closed the connection. If we have an unfinished
            // event in the buffer, that's a protocol violation, but we
            // mark the stream ended and let the user `close` cleanly.
            state.ended = true;
            return Ok(LLM_STREAM_END_MARKER.to_string());
        }
        state.line_buf.extend_from_slice(&buf[..n]);
    }
}

/// Try to parse one complete SSE event from `line_buf`. Returns:
/// - `Ok(Some(delta))` — one delta parsed and consumed; return to caller.
/// - `Ok(None)` — need more bytes; the read loop will pull more.
/// - `Err(_)` — protocol-level error (malformed SSE).
///
/// SSE event format (industry standard, OpenAI/Anthropic/Ollama follow it):
/// ```text
/// event: <name>\r\n     <- optional, default "message"
/// data: <json>\r\n      <- one or more `data:` lines
/// \r\n                  <- blank line = event terminator
/// ```
/// Ollama native (not strict SSE, but close): one JSON object per line,
/// terminated by `\n`. We detect ollama's `"done": true` as end.
fn try_parse_one_event(state: &mut LlmStreamState) -> Result<Option<String>, String> {
    // Different providers shape their stream slightly differently:
    // - OpenAI/Anthropic: strict SSE — `data: <json>\n\n` blocks.
    // - Ollama: newline-delimited JSON (not SSE). We detect by
    //   `provider_type == "ollama"` and parse one JSON object per line.
    if state.provider_type == "ollama" {
        return try_parse_ollama_line(state);
    }
    try_parse_sse_event(state)
}

fn try_parse_sse_event(state: &mut LlmStreamState) -> Result<Option<String>, String> {
    // Find the event terminator: a blank line. SSE allows `\n\n` or
    // `\r\n\r\n`. We accept both — split on either.
    let terminator = find_sse_terminator(&state.line_buf);
    let Some(term_len) = terminator else {
        return Ok(None); // need more bytes
    };
    // Consume the event bytes.
    let event_bytes = state.line_buf.drain(..term_len).collect::<Vec<u8>>();
    // Trim the trailing blank line — what's left is `data:` lines.
    let event_str = String::from_utf8_lossy(&event_bytes);
    let mut delta_out = String::new();
    for line in event_str.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if let Some(payload) = line
            .strip_prefix("data:")
            .or_else(|| line.strip_prefix("data: "))
        {
            let payload = payload.trim();
            if payload == "[DONE]" {
                // OpenAI end-of-stream marker.
                state.ended = true;
                continue;
            }
            // Try to parse as JSON — Anthropic / OpenAI emit JSON deltas.
            // Extract the delta text; provider shape varies.
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(delta) = extract_delta_text(&state.provider_type, &parsed) {
                    delta_out.push_str(&delta);
                }
                // Extract usage from the final event (Anthropic
                // `message_delta` / OpenAI final chunk with
                // `stream_options.include_usage`).
                if let Some((in_t, out_t)) = extract_usage(&state.provider_type, &parsed) {
                    state.input_tokens = Some(in_t);
                    state.output_tokens = Some(out_t);
                }
                // Detect Anthropic end-of-stream.
                if parsed.get("type").and_then(|v| v.as_str()) == Some("message_stop") {
                    state.ended = true;
                }
            }
            // If JSON parse failed — provider sent malformed data; skip
            // this `data:` line silently (some providers send keep-alive
            // comments or partial JSON; we don't crash on them).
        }
        // Lines without `data:` prefix are ignored (comments, `event:`,
        // `id:`, `retry:` — SSE protocol meta, not deltas).
    }
    Ok(Some(delta_out))
}

fn try_parse_ollama_line(state: &mut LlmStreamState) -> Result<Option<String>, String> {
    // Ollama emits one JSON object per line, terminated by `\n`.
    let Some(newline_idx) = state.line_buf.iter().position(|&b| b == b'\n') else {
        return Ok(None); // need more bytes
    };
    let line_bytes = state.line_buf.drain(..=newline_idx).collect::<Vec<u8>>();
    let line_str = String::from_utf8_lossy(&line_bytes);
    let line_str = line_str.trim();
    if line_str.is_empty() {
        return Ok(Some(String::new())); // keep-alive blank line
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line_str) else {
        return Ok(Some(String::new())); // malformed line — skip silently
    };
    let mut delta_out = String::new();
    if let Some(resp) = parsed.get("response").and_then(|v| v.as_str()) {
        delta_out.push_str(resp);
    }
    // Ollama reports usage in the final chunk (`done: true`).
    if parsed
        .get("done")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        state.ended = true;
        if let Some(counts) = parsed.get("eval_count").and_then(|v| v.as_u64()) {
            state.output_tokens = Some(counts);
        }
        if let Some(counts) = parsed.get("prompt_eval_count").and_then(|v| v.as_u64()) {
            state.input_tokens = Some(counts);
        }
    } else {
        // Per-chunk usage — accumulate only the final, but record both
        // every time (the final chunk overrides).
        if let Some(counts) = parsed.get("eval_count").and_then(|v| v.as_u64()) {
            state.output_tokens = Some(counts);
        }
        if let Some(counts) = parsed.get("prompt_eval_count").and_then(|v| v.as_u64()) {
            state.input_tokens = Some(counts);
        }
    }
    Ok(Some(delta_out))
}

/// Find the index of the first SSE event terminator (`\n\n` or `\r\n\r\n`)
/// in `buf`, plus the terminator's own length. Returns `None` if no
/// terminator yet.
fn find_sse_terminator(buf: &[u8]) -> Option<usize> {
    // Try `\n\n` first (most common — Anthropic / OpenAI).
    for i in 0..buf.len().saturating_sub(1) {
        if buf[i] == b'\n' && buf[i + 1] == b'\n' {
            return Some(i + 2); // include the terminator itself
        }
    }
    // Try `\r\n\r\n`.
    for i in 0..buf.len().saturating_sub(3) {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' && buf[i + 2] == b'\r' && buf[i + 3] == b'\n' {
            return Some(i + 4);
        }
    }
    None
}

/// Provider-specific delta-text extraction from a parsed SSE JSON chunk.
fn extract_delta_text(provider: &str, parsed: &serde_json::Value) -> Option<String> {
    match provider {
        "anthropic" => {
            // `content_block_delta` events carry `delta.text`.
            if parsed.get("type").and_then(|v| v.as_str()) == Some("content_block_delta") {
                parsed
                    .get("delta")
                    .and_then(|d| d.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        }
        _ => {
            // OpenAI-compatible: `choices[0].delta.content`.
            parsed
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c0| c0.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        }
    }
}

/// Provider-specific usage extraction from a parsed SSE JSON chunk.
/// Returns `(input_tokens, output_tokens)` when the chunk reports usage.
fn extract_usage(provider: &str, parsed: &serde_json::Value) -> Option<(u64, u64)> {
    match provider {
        "anthropic" => {
            // `message_delta` carries `usage.output_tokens`; the
            // initial `message_start` carries `usage.input_tokens`.
            if let Some(u) = parsed
                .get("message")
                .and_then(|m| m.get("usage"))
                .or_else(|| parsed.get("usage"))
            {
                let in_t = u.get("input_tokens").and_then(|v| v.as_u64());
                let out_t = u.get("output_tokens").and_then(|v| v.as_u64());
                if in_t.is_some() || out_t.is_some() {
                    return Some((in_t.unwrap_or(0), out_t.unwrap_or(0)));
                }
            }
            None
        }
        _ => {
            // OpenAI-compatible: final chunk carries `usage` when
            // `stream_options.include_usage: true` is set (we don't set
            // it in v1 — honest: returns None).
            if let Some(u) = parsed.get("usage") {
                let in_t = u.get("prompt_tokens").and_then(|v| v.as_u64());
                let out_t = u.get("completion_tokens").and_then(|v| v.as_u64());
                if in_t.is_some() || out_t.is_some() {
                    return Some((in_t.unwrap_or(0), out_t.unwrap_or(0)));
                }
            }
            None
        }
    }
}

/// `llm_stream_close(handle)` — drop the response, aggregate final
/// metadata, write ONE trace line (ADR-0138 §D4 — one line per completed
/// stream, never per chunk).
pub fn stream_close(handle: LlmStreamId) -> Result<LlmStreamFinal, String> {
    let mut state = {
        let Ok(mut reg) = LLM_STREAM_REGISTRY.lock() else {
            return Err("llm_stream_close(): stream registry mutex poisoned".to_string());
        };
        reg.take(handle)
            .ok_or_else(|| format!("llm_stream_close(): unknown LlmStream handle #{}", handle.0))?
    };
    // If the user closes before the stream's natural end, we still
    // trace honestly — status "ok" if we got at least some data and
    // ended cleanly, "error" otherwise. mid-stream close → "ok" with
    // whatever we have (provider saw a clean TCP close from our side).
    let latency_ms = state.started_at.elapsed().as_millis() as u64;
    let status = state.status;
    let provider = state.provider_type.clone();
    let provider_alias = state.provider_alias.clone();
    let model = state.resolved_model.clone();
    let input_tokens = state.input_tokens;
    let output_tokens = state.output_tokens;
    let aggregated_text = std::mem::take(&mut state.aggregated_text);
    // Dropping `state` here drops the `reqwest::blocking::Response`,
    // which closes the underlying TCP connection (visible to the server
    // as a client-side close — tested in naryad_275_stream_close_before_end).
    drop(state);

    // ONE trace line per completed stream (ADR-0138 §D4 contract).
    trace_llm_call(&LlmTraceEvent {
        provider_name: Some(&provider),
        model: Some(&model),
        input_tokens,
        output_tokens,
        latency_ms,
        status,
        cache: "miss",
        provider_alias: Some(&provider_alias),
    });

    Ok(LlmStreamFinal {
        provider,
        model,
        input_tokens,
        output_tokens,
        latency_ms,
        status,
        aggregated_text,
    })
}

/// Peek the provider_type + resolved_model of an open stream — used by
/// `llm_stream_open` builtin to populate the returned Struct's metadata
/// fields. Read-only — does not advance the stream, does not take
/// ownership. Returns `None` if the handle is unknown (closed or never
/// opened). Non-fatal — the open succeeded, we just couldn't peek.
pub fn peek_stream_provenance(handle: LlmStreamId) -> Option<(String, String)> {
    let Ok(reg) = LLM_STREAM_REGISTRY.lock() else {
        return None;
    };
    reg.streams
        .get(&handle.0)
        .map(|s| (s.provider_type.clone(), s.resolved_model.clone()))
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
        // Н454: the mock no longer echoes the prompt — it answers with the
        // deterministic non-echo marker (renamed semantics, same intent:
        // MockLlm is deterministic for goldens).
        let backend = MockLlm;
        let result = backend.call("classify this", "input text");
        let answer = result.unwrap();
        assert!(
            answer.starts_with("[mock-llm:") && answer.ends_with("]"),
            "expected the №454 marker, got: {}",
            answer
        );
        assert!(!answer.contains("classify this"), "no prompt echo");
    }

    #[test]
    fn test_mock_llm_ignores_input() {
        // Н454: the marker depends only on the prompt — the input is ignored.
        let backend = MockLlm;
        let a = backend.call("expected", "ignored").unwrap();
        let b = backend.call("expected", "different input").unwrap();
        assert_eq!(a, b, "input must not affect the mock answer");
        assert!(!a.contains("ignored"));
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
    fn test_create_llm_backend_default_is_real_fail_loud() {
        // Н454: without the variable the factory returns the REAL backend —
        // which fails loudly at the first call (no credentials). No network
        // is touched: the key check fires before any request.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("METALOGOS_MOCK_LLM");
        let backend = create_llm_backend();
        let result = backend.call("prompt", "input");
        assert!(
            result.is_err(),
            "default backend must be the real one, fail-loud (№454)"
        );
        assert!(
            result.unwrap_err().contains("METALOGOS_API_KEY"),
            "loud failure must point at the missing credentials"
        );
    }

    #[test]
    fn test_create_llm_backend_explicit_mock_true() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_MOCK_LLM", "true");
        let backend = create_llm_backend();
        let answer = backend.call("prompt", "input").unwrap();
        assert!(
            answer.starts_with("[mock-llm:") && answer.ends_with("]"),
            "mock answer must be the deterministic non-echo marker (№454), got: {}",
            answer
        );
        assert!(!answer.contains("prompt"), "mock must not echo the prompt");
        env::remove_var("METALOGOS_MOCK_LLM");
    }

    #[test]
    fn test_create_llm_backend_explicit_mock_1() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("METALOGOS_MOCK_LLM", "1");
        let backend = create_llm_backend();
        let answer = backend.call("prompt", "input").unwrap();
        assert!(
            answer.starts_with("[mock-llm:") && answer.ends_with("]"),
            "mock answer must be the deterministic non-echo marker (№454), got: {}",
            answer
        );
        assert!(!answer.contains("prompt"), "mock must not echo the prompt");
        env::remove_var("METALOGOS_MOCK_LLM");
    }

    #[test]
    fn test_mock_response_deterministic_and_non_echo() {
        let a = mock_response("classify: SECRET_MARKER_XYZ");
        let b = mock_response("classify: SECRET_MARKER_XYZ");
        let c = mock_response("classify: another input");
        assert_eq!(a, b, "same prompt → same marker (deterministic)");
        assert_ne!(a, c, "different prompt → different marker");
        assert!(!a.contains("SECRET_MARKER_XYZ"), "no prompt echo");
        let hex = a
            .strip_prefix("[mock-llm:")
            .and_then(|s| s.strip_suffix("]"))
            .unwrap();
        assert_eq!(hex.len(), 8, "8 hex chars, got: {}", hex);
        assert!(hex.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn test_mock_llm_requested_explicit_only() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("METALOGOS_MOCK_LLM");
        assert!(!mock_llm_requested(), "unset → real backend (№454)");
        for truthy in ["1", "true", "TRUE", "True"] {
            env::set_var("METALOGOS_MOCK_LLM", truthy);
            assert!(mock_llm_requested(), "{} → mock", truthy);
        }
        for falsy in ["0", "false", "FALSE", "yes", ""] {
            env::set_var("METALOGOS_MOCK_LLM", falsy);
            assert!(!mock_llm_requested(), "{} → real backend", falsy);
        }
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

    // ── SmartRouter resolve_endpoint deduplication (Наряд №4 fix) ──

    #[test]
    fn test_smart_router_resolve_endpoint_base_url_only() {
        let router = make_test_router();
        // Base URL without path — suffix should be appended
        let result = router.resolve_endpoint("openai", Some("http://localhost:8080"));
        assert_eq!(result, "http://localhost:8080/chat/completions");
    }

    #[test]
    fn test_smart_router_resolve_endpoint_base_with_v1() {
        let router = make_test_router();
        // Base URL with /v1 — suffix appended (same as RealLlm Наряд №32 behavior)
        let result = router.resolve_endpoint("openai", Some("http://localhost:8080/v1"));
        assert_eq!(result, "http://localhost:8080/v1/chat/completions");
    }

    #[test]
    fn test_smart_router_resolve_endpoint_full_path_no_dup() {
        let router = make_test_router();
        // Full endpoint URL — should NOT double the path
        let result =
            router.resolve_endpoint("openai", Some("http://localhost:8080/v1/chat/completions"));
        assert_eq!(result, "http://localhost:8080/v1/chat/completions");
    }

    #[test]
    fn test_smart_router_resolve_endpoint_ollama_full_path_no_dup() {
        let router = make_test_router();
        let result = router.resolve_endpoint("ollama", Some("http://localhost:11434/api/generate"));
        assert_eq!(result, "http://localhost:11434/api/generate");
    }

    #[test]
    fn test_smart_router_resolve_endpoint_anthropic_full_path_no_dup() {
        let router = make_test_router();
        let result = router.resolve_endpoint("anthropic", Some("https://myproxy.com/v1/messages"));
        assert_eq!(result, "https://myproxy.com/v1/messages");
    }

    #[test]
    fn test_smart_router_resolve_endpoint_groq_full_path_no_dup() {
        let router = make_test_router();
        // groq default has /openai/v1/chat/completions — extract_endpoint_suffix strips /openai/v1/ → /chat/completions
        let result = router.resolve_endpoint(
            "groq",
            Some("https://myproxy.com/openai/v1/chat/completions"),
        );
        assert_eq!(result, "https://myproxy.com/openai/v1/chat/completions");
    }

    #[test]
    fn test_smart_router_resolve_endpoint_no_custom_url() {
        let router = make_test_router();
        let result = router.resolve_endpoint("openai", None);
        assert_eq!(result, "https://api.openai.com/v1/chat/completions");
    }

    fn make_test_router() -> SmartRouter {
        SmartRouter {
            providers: vec![],
            default_model: None,
            failover: true,
            timeout: 30,
            tracker: LlmUsageTracker::new(vec![], 3),
        }
    }

    // ── Integration Tests (require real API keys) ──────────────────

    #[test]
    #[ignore = "METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=openai METALOGOS_API_KEY=sk-xxx cargo test -- --ignored"]
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
    #[ignore = "METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=anthropic METALOGOS_API_KEY=sk-ant-xxx cargo test -- --ignored"]
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
    #[ignore = "METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=ollama cargo test -- --ignored"]
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

    // ── SmartRouter integration: failover + circuit breaker (Наряд №4) ──
    // These tests use a real HTTP listener as echo server.
    // Run with: cargo test --lib naryad4_integration -- --ignored --nocapture

    /// Minimal OpenAI-compatible echo server running in a background thread.
    /// Returns (port, shutdown_trigger).
    /// The server responds 200 with the user's message prefixed by "ECHO: ".
    fn start_echo_server() -> (u16, std::sync::mpsc::Sender<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind echo server");
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel::<()>();

        std::thread::spawn(move || {
            listener.set_nonblocking(true).ok();
            let mut buf = [0u8; 8192];
            loop {
                // Check shutdown
                if rx.try_recv().is_ok() {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                        let n = stream.read(&mut buf).unwrap_or(0);
                        if n > 0 {
                            let body = String::from_utf8_lossy(&buf[..n]);
                            // Extract user message
                            let user_msg = extract_user_msg_inner(&body);
                            let response = format!(
                                r#"{{"choices":[{{"message":{{"role":"assistant","content":"ECHO: {}"}},"finish_reason":"stop"}}]}}"#,
                                user_msg.replace('"', "\\\"")
                            );
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                response.len(),
                                response
                            );
                            let _ = stream.write_all(resp.as_bytes());
                            eprintln!("  [ECHO] 200 OK — {} bytes", n);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        (port, tx)
    }

    fn extract_user_msg_inner(json_str: &str) -> String {
        if let Some(start) = json_str.find("\"content\":\"") {
            let rest = &json_str[start + 11..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
        "(no message found)".to_string()
    }

    #[test]
    fn test_naryad4_failover_to_echo_server() {
        // Start echo server
        let (echo_port, echo_shutdown) = start_echo_server();
        std::thread::sleep(Duration::from_millis(200));

        // Create router: first provider dead, second is echo
        let providers = vec![
            (
                "dead".to_string(),
                "openai".to_string(),
                None,
                Some("http://127.0.0.1:19999".to_string()),
            ),
            (
                "echo".to_string(),
                "openai".to_string(),
                None,
                Some(format!(
                    "http://127.0.0.1:{}/v1/chat/completions",
                    echo_port
                )),
            ),
        ];
        let router = SmartRouter {
            providers,
            default_model: Some("test-model".to_string()),
            failover: true,
            timeout: 5,
            tracker: LlmUsageTracker::new(vec!["dead".to_string(), "echo".to_string()], 3),
        };

        let start = Instant::now();
        let result = router.call("Say hello", "world", None, None);
        let elapsed = start.elapsed();

        // Cleanup
        let _ = echo_shutdown.send(());

        eprintln!("  Failover test: elapsed={:?}", elapsed);
        eprintln!("  Result: {:?}", result);

        // Assertions
        assert!(
            result.is_ok(),
            "SmartRouter failover should succeed, got: {:?}",
            result
        );
        let resp = result.unwrap();
        assert!(
            resp.contains("ECHO"),
            "Response should come from echo server, got: {}",
            resp
        );

        // Verify usage: 2 calls total (1 dead fail + 1 echo success), 1 error
        let report = router.usage_report();
        eprintln!(
            "  Usage: calls={}, errors={}",
            report.total_calls, report.total_errors
        );
        assert_eq!(report.total_calls, 2.0, "Should have tried 2 providers");
        assert_eq!(
            report.total_errors, 1.0,
            "Should have 1 error (dead provider)"
        );
    }

    #[test]
    fn test_naryad4_circuit_breaker_opens() {
        // Single dead provider, circuit_threshold=3
        let providers = vec![(
            "dead".to_string(),
            "openai".to_string(),
            None,
            Some("http://127.0.0.1:19999".to_string()),
        )];
        let router = SmartRouter {
            providers,
            default_model: Some("test-model".to_string()),
            failover: true,
            timeout: 2,
            tracker: LlmUsageTracker::new(vec!["dead".to_string()], 3),
        };

        // Calls 1-3: should attempt the dead provider (each ~2s timeout)
        let mut timings = Vec::new();
        for i in 1..=4 {
            let start = Instant::now();
            let result = router.call(&format!("test{}", i), "input", None, None);
            let elapsed = start.elapsed();
            timings.push(elapsed);
            eprintln!(
                "  Call {}: {:?} — {}",
                i,
                elapsed,
                if result.is_err() { "ERR" } else { "OK" }
            );
        }

        // Verify: calls 1-3 should actually attempt the provider,
        // call 4 should be near-instant (circuit open, provider skipped).
        let avg_first_three_us: u128 = timings[..3].iter().map(|d| d.as_micros()).sum::<u128>() / 3;
        let call4_us = timings[3].as_micros();

        eprintln!(
            "  Avg calls 1-3: {}µs, Call 4: {}µs",
            avg_first_three_us, call4_us
        );

        // Call 4 must be near-instant — circuit breaker skips the provider entirely.
        // Use a generous 1ms threshold; the actual value should be <100µs.
        assert!(
            call4_us < 1000,
            "Call 4 should be near-instant (circuit open), took {}µs",
            call4_us
        );
        // Calls 1-3 must have actually attempted the provider (at least a TCP connect attempt).
        // Connection refused is fast (~1ms), but still orders of magnitude slower than skipping.
        assert!(
            avg_first_three_us > 100,
            "Calls 1-3 should attempt the provider, avg was {}µs",
            avg_first_three_us
        );

        // Usage: 3 provider attempts (call 4 skips via circuit)
        let report = router.usage_report();
        eprintln!(
            "  Usage: total_calls={}, total_errors={}",
            report.total_calls, report.total_errors
        );
        // Calls 1-3 hit the provider, call 4 was skipped by circuit breaker
        assert_eq!(
            report.total_calls, 3.0,
            "Only 3 actual provider calls (call 4 skipped by CB)"
        );
        assert_eq!(report.total_errors, 3.0, "All 3 provider calls failed");
    }
}
