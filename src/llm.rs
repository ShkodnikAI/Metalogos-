// ── LLM client abstraction for METALOGOS M3 ─────────────────────────
// Production mode: HTTP POST to configurable API endpoint.
// Mock mode: deterministic stub returning the prompt string (for golden tests).

use std::env;

/// A trait for LLM backends — allows swapping between real and mock.
pub trait LlmBackend: Send + Sync {
    /// Call the LLM with a prompt + input text. Returns the model's response.
    fn call(&self, prompt: &str, input: &str) -> Result<String, String>;
}

/// Mock LLM backend for testing. Returns the prompt string as-is (deterministic).
/// This is what golden tests use — the "prompt" field IS the expected response.
pub struct MockLlm;

impl LlmBackend for MockLlm {
    fn call(&self, prompt: &str, _input: &str) -> Result<String, String> {
        Ok(prompt.to_string())
    }
}

/// Real LLM backend — HTTP POST to a configurable API endpoint.
/// Expects the API to accept { "prompt": "...", "input": "..." } and return
/// a JSON { "response": "..." } body (OpenAI-compatible or custom).
pub struct RealLlm {
    endpoint: String,
    api_key: Option<String>,
}

impl RealLlm {
    pub fn new() -> Self {
        let endpoint = env::var("METALOGOS_LLM_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:11434/api/generate".to_string());
        let api_key = env::var("METALOGOS_LLM_API_KEY").ok();
        RealLlm { endpoint, api_key }
    }
}

impl LlmBackend for RealLlm {
    fn call(&self, prompt: &str, input: &str) -> Result<String, String> {
        // Build request body
        let body = serde_json::json!({
            "prompt": format!("{}\n\nInput: {}", prompt, input),
            "stream": false
        });

        // We avoid reqwest dependency for now — use a minimal HTTP call
        // via std::process::Command calling curl as a simple solution
        let mut cmd = std::process::Command::new("curl");
        cmd.arg("-s")
            .arg("-X")
            .arg("POST")
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(&body.to_string());

        if let Some(key) = &self.api_key {
            cmd.arg("-H").arg(format!("Authorization: Bearer {}", key));
        }

        cmd.arg(&self.endpoint);

        let output = cmd.output()
            .map_err(|e| format!("failed to execute curl: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("LLM API call failed: {}", stderr));
        }

        let response_str = String::from_utf8_lossy(&output.stdout);
        parse_llm_response(&response_str)
    }
}

/// Parse LLM API response. Supports two formats:
/// 1. OpenAI-compatible: { "choices": [{ "message": { "content": "..." } }] }
/// 2. Ollama-compatible: { "response": "..." }
/// 3. Simple: { "response": "..." } or plain text
fn parse_llm_response(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();

    // Try JSON parsing
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // OpenAI format
        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
            if let Some(content) = choices.first()
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                return Ok(content.trim().to_string());
            }
        }
        // Ollama / simple format
        if let Some(response) = json.get("response").and_then(|r| r.as_str()) {
            return Ok(response.trim().to_string());
        }
    }

    // Fallback: return raw text
    Ok(trimmed.to_string())
}

/// Create an LLM backend based on environment.
/// If METALOGOS_MOCK_LLM is set (or not in production mode), returns MockLlm.
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
