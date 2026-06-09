// ── LLM client abstraction for METALOGOS M3 ─────────────────────────
// Phase 7.1: Real LLM backends — Anthropic, OpenAI, Ollama.
// Mock mode for testing (METALOGOS_MOCK_LLM=true).
// Retry with exponential backoff (3 retries, 1s/2s/4s). Timeout 30s.

use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// A trait for LLM backends — allows swapping between real and mock.
pub trait LlmBackend: Send + Sync {
    /// Call the LLM with a prompt + input text. Returns the model's text response.
    fn call(&self, prompt: &str, input: &str) -> Result<String, String>;

    /// Call the LLM with an optional per-call model override (ADR-0048).
    /// Default implementation ignores the override and delegates to `call()`.
    /// Real backends use the override model in the API JSON body;
    /// MockLlm records it for contract tests.
    fn call_with_model(&self, prompt: &str, input: &str, _model: Option<&str>) -> Result<String, String> {
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
        *MOCK_LLM_LAST_MODEL.lock().unwrap() = String::new();
    }

    /// Get the last model override passed to call_with_model().
    /// Empty string if no override was used or call() was called directly.
    pub fn last_model() -> String {
        MOCK_LLM_LAST_MODEL.lock().unwrap().clone()
    }
}

impl LlmBackend for MockLlm {
    fn call(&self, prompt: &str, _input: &str) -> Result<String, String> {
        MOCK_LLM_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(prompt.to_string())
    }

    fn call_with_model(&self, prompt: &str, input: &str, model: Option<&str>) -> Result<String, String> {
        MOCK_LLM_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        // Record model for contract tests
        if let Some(m) = model {
            *MOCK_LLM_LAST_MODEL.lock().unwrap() = m.to_string();
        } else {
            *MOCK_LLM_LAST_MODEL.lock().unwrap() = String::new();
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
        // Build blocking HTTP client with 30s timeout
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
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
    fn call_with_model(&self, prompt: &str, input: &str, model: Option<&str>) -> Result<String, String> {
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
    pub fn resolve_endpoint(&self) -> String {
        if let Some(ref base) = self.base_url {
            // Extract the path suffix from the default endpoint
            // e.g., "https://api.openai.com/v1/chat/completions" → "/chat/completions"
            let default = self.provider.endpoint();
            if let Some(idx) = default.find("://") {
                if let Some(slash_idx) = default[idx + 3..].find('/') {
                    let path = &default[idx + 3 + slash_idx..];
                    format!("{}{}", base.trim_end_matches('/'), path)
                } else {
                    base.clone()
                }
            } else {
                base.clone()
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
    let json: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| format!("Failed to parse Anthropic JSON: {}", e))?;

    json.get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter().find(|item| {
                item.get("type").and_then(|t| t.as_str()) == Some("text")
            })
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
    let json: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| format!("Failed to parse OpenAI JSON: {}", e))?;

    json.get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(|c| c.as_str())
        .map(|t| t.trim().to_string())
        .ok_or_else(|| {
            format!(
                "Unexpected OpenAI response format: {}",
                truncate(raw, 300)
            )
        })
}

/// Parse Ollama response: `{ "response": "..." }`
fn parse_ollama_response(raw: &str) -> Result<String, String> {
    let json: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| format!("Failed to parse Ollama JSON: {}", e))?;

    json.get("response")
        .and_then(|r| r.as_str())
        .map(|t| t.trim().to_string())
        .ok_or_else(|| {
            format!(
                "Unexpected Ollama response format: {}",
                truncate(raw, 300)
            )
        })
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

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        env::remove_var("METALOGOS_LLM_PROVIDER");
        assert_eq!(Provider::from_env(), Provider::Anthropic);
    }

    #[test]
    fn test_provider_from_env_openai() {
        env::set_var("METALOGOS_LLM_PROVIDER", "openai");
        assert_eq!(Provider::from_env(), Provider::OpenAI);
        env::remove_var("METALOGOS_LLM_PROVIDER");
    }

    #[test]
    fn test_provider_from_env_ollama() {
        env::set_var("METALOGOS_LLM_PROVIDER", "ollama");
        assert_eq!(Provider::from_env(), Provider::Ollama);
        env::remove_var("METALOGOS_LLM_PROVIDER");
    }

    #[test]
    fn test_provider_from_env_case_insensitive() {
        env::set_var("METALOGOS_LLM_PROVIDER", "OpenAI");
        assert_eq!(Provider::from_env(), Provider::OpenAI);
        env::remove_var("METALOGOS_LLM_PROVIDER");
    }

    #[test]
    fn test_default_models() {
        assert_eq!(Provider::Anthropic.default_model(), "claude-sonnet-4-20250514");
        assert_eq!(Provider::OpenAI.default_model(), "gpt-4o");
        assert_eq!(Provider::Ollama.default_model(), "llama3");
    }

    #[test]
    fn test_endpoint_urls() {
        assert_eq!(Provider::Anthropic.endpoint(), "https://api.anthropic.com/v1/messages");
        assert_eq!(Provider::OpenAI.endpoint(), "https://api.openai.com/v1/chat/completions");
        assert_eq!(Provider::Ollama.endpoint(), "http://localhost:11434/api/generate");
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
        assert!(!is_client_error("OpenAI API error (500): Internal Server Error"));
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
        env::remove_var("METALOGOS_MOCK_LLM");
        let backend = create_llm_backend();
        assert_eq!(backend.call("prompt", "input").unwrap(), "prompt");
    }

    #[test]
    fn test_create_llm_backend_explicit_mock_true() {
        env::set_var("METALOGOS_MOCK_LLM", "true");
        let backend = create_llm_backend();
        assert_eq!(backend.call("prompt", "input").unwrap(), "prompt");
        env::remove_var("METALOGOS_MOCK_LLM");
    }

    #[test]
    fn test_create_llm_backend_explicit_mock_1() {
        env::set_var("METALOGOS_MOCK_LLM", "1");
        let backend = create_llm_backend();
        assert_eq!(backend.call("prompt", "input").unwrap(), "prompt");
        env::remove_var("METALOGOS_MOCK_LLM");
    }

    // ── Integration Tests (require real API keys) ──────────────────

    #[test]
    #[ignore] // METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=openai METALOGOS_API_KEY=sk-xxx cargo test -- --ignored
    fn test_real_llm_openai_classify() {
        let api_key = env::var("METALOGOS_API_KEY")
            .expect("METALOGOS_API_KEY must be set");
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
        assert!(response.to_lowercase().contains("complaint"),
            "Expected 'complaint', got: {}", response);
    }

    #[test]
    #[ignore] // METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=anthropic METALOGOS_API_KEY=sk-ant-xxx cargo test -- --ignored
    fn test_real_llm_anthropic_classify() {
        let api_key = env::var("METALOGOS_API_KEY")
            .expect("METALOGOS_API_KEY must be set");
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
        assert!(response.to_lowercase().contains("complaint"),
            "Expected 'complaint', got: {}", response);
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
        assert!(response.to_lowercase().contains("complaint"),
            "Expected 'complaint', got: {}", response);
    }
}
