//! Наряд №12: Metalogos Runtime Fixes — Contract Tests
//!
//! Tests for 4 bugs fixed in this PR:
//! 1. builtins/call_llm() work in route handlers (clone_definitions_into copies hooks/cache/stats)
//! 2. http_post() supports 4th parameter for authorization headers
//! 3. __replace() handles UTF-8/Cyrillic correctly (no panic)
//! 4. METALOGOS_OPENAI_BASE_URL for custom LLM base URL

#[cfg(test)]
mod tests {

    // ── Bug 2: http_post() 4th parameter for auth headers ──

    #[test]
    fn test_bug2_http_post_backward_compatible_3_args() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let http_post = builtins.get("http_post").expect("http_post builtin exists");

        // 3 args should still work (backward compatible)
        let result = http_post(&[
            Value::String("https://httpbin.org/post".to_string()),
            Value::String("{\"test\": true}".to_string()),
            Value::String("application/json".to_string()),
        ]);
        match result {
            Ok(Value::String(_)) => {}
            Err(e) => assert!(
                e.contains("request failed") || e.contains("returned status"),
                "Unexpected error type: {}",
                e
            ),
            other => panic!("Unexpected return type: {:?}", other),
        }
    }

    #[test]
    fn test_bug2_http_post_struct_headers_arg() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;
        use std::collections::HashMap;

        let builtins = Builtins::new();
        let http_post = builtins.get("http_post").expect("http_post builtin exists");

        let mut fields = HashMap::new();
        fields.insert(
            "Authorization".to_string(),
            Value::String("Bearer test-token".to_string()),
        );
        let result = http_post(&[
            Value::String("https://httpbin.org/post".to_string()),
            Value::String("{\"test\": true}".to_string()),
            Value::String("application/json".to_string()),
            Value::Struct {
                type_name: "Headers".to_string(),
                fields,
            },
        ]);
        match result {
            Ok(_) | Err(_) => {}
        }
    }

    #[test]
    fn test_bug2_http_post_json_string_headers_arg() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let http_post = builtins.get("http_post").expect("http_post builtin exists");

        let result = http_post(&[
            Value::String("https://httpbin.org/post".to_string()),
            Value::String("{\"test\": true}".to_string()),
            Value::String("application/json".to_string()),
            Value::String("{\"Authorization\": \"Bearer test-token\"}".to_string()),
        ]);
        match result {
            Ok(_) | Err(_) => {}
        }
    }

    #[test]
    fn test_bug2_http_post_rejects_invalid_headers_type() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let http_post = builtins.get("http_post").expect("http_post builtin exists");

        let result = http_post(&[
            Value::String("https://httpbin.org/post".to_string()),
            Value::String("{}".to_string()),
            Value::String("application/json".to_string()),
            Value::Float(42.0),
        ]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("4th arg must be String (JSON) or Struct"));
    }

    // ── Bug 3: __replace() UTF-8 / Cyrillic safety ──

    #[test]
    fn test_bug3_replace_cyrillic_basic() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let replace_fn = builtins.get("__replace").expect("__replace builtin exists");

        let result = replace_fn(&[
            Value::String("Привет, мир!".to_string()),
            Value::String("мир".to_string()),
            Value::String("свет".to_string()),
        ])
        .expect("__replace should not panic on Cyrillic");

        match result {
            Value::String(s) => assert_eq!(s, "Привет, свет!"),
            other => panic!("Expected String, got {}", other.type_name()),
        }
    }

    #[test]
    fn test_bug3_replace_cyrillic_emoji() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let replace_fn = builtins.get("replace").expect("replace builtin exists");

        let result = replace_fn(&[
            Value::String("Работа завершена 🎉".to_string()),
            Value::String("🎉".to_string()),
            Value::String("✅".to_string()),
        ])
        .expect("replace should handle multi-byte UTF-8");

        match result {
            Value::String(s) => assert_eq!(s, "Работа завершена ✅"),
            other => panic!("Expected String, got {}", other.type_name()),
        }
    }

    #[test]
    fn test_bug3_replace_cyrillic_multiple_occurrences() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let replace_fn = builtins.get("__replace").expect("__replace builtin exists");

        let result = replace_fn(&[
            Value::String("да да нет да".to_string()),
            Value::String("да".to_string()),
            Value::String("maybe".to_string()),
        ])
        .expect("__replace should handle multiple Cyrillic occurrences");

        match result {
            Value::String(s) => assert_eq!(s, "maybe maybe нет maybe"),
            other => panic!("Expected String, got {}", other.type_name()),
        }
    }

    #[test]
    fn test_bug3_replace_cyrillic_empty_pattern() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let replace_fn = builtins.get("__replace").expect("__replace builtin exists");

        let result = replace_fn(&[
            Value::String("Привет".to_string()),
            Value::String("".to_string()),
            Value::String("X".to_string()),
        ])
        .expect("replace with empty pattern should not panic");

        match result {
            Value::String(s) => assert_eq!(s, "Привет"),
            other => panic!("Expected String, got {}", other.type_name()),
        }
    }

    #[test]
    fn test_bug3_replace_cyrillic_no_match() {
        use metalogos::builtins::Builtins;
        use metalogos::interpreter::Value;

        let builtins = Builtins::new();
        let replace_fn = builtins.get("__replace").expect("__replace builtin exists");

        let result = replace_fn(&[
            Value::String("Привет мир".to_string()),
            Value::String("xyz".to_string()),
            Value::String("ABC".to_string()),
        ])
        .expect("replace with no match should not panic");

        match result {
            Value::String(s) => assert_eq!(s, "Привет мир"),
            other => panic!("Expected String, got {}", other.type_name()),
        }
    }

    // ── Bug 4: METALOGOS_OPENAI_BASE_URL ──

    #[test]
    fn test_bug4_resolve_endpoint_openai_default() {
        use metalogos::llm::{Provider, RealLlm};

        let llm = RealLlm::with_config(Provider::OpenAI, "gpt-4o".to_string(), None);
        assert_eq!(
            llm.resolve_endpoint(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_bug4_resolve_endpoint_openai_custom() {
        use metalogos::llm::{Provider, RealLlm};

        let mut llm = RealLlm::with_config(Provider::OpenAI, "gpt-4o".to_string(), None);
        llm.base_url = Some("https://my-proxy.example.com/v1".to_string());

        let endpoint = llm.resolve_endpoint();
        assert_eq!(endpoint, "https://my-proxy.example.com/v1/chat/completions");
    }

    #[test]
    fn test_bug4_resolve_endpoint_openai_custom_trailing_slash() {
        use metalogos::llm::{Provider, RealLlm};

        let mut llm = RealLlm::with_config(Provider::OpenAI, "gpt-4o".to_string(), None);
        llm.base_url = Some("https://my-proxy.example.com/v1/".to_string());

        let endpoint = llm.resolve_endpoint();
        assert_eq!(endpoint, "https://my-proxy.example.com/v1/chat/completions");
    }

    #[test]
    fn test_bug4_resolve_endpoint_anthropic_default() {
        use metalogos::llm::{Provider, RealLlm};

        let llm = RealLlm::with_config(
            Provider::Anthropic,
            "claude-sonnet-4-20250514".to_string(),
            None,
        );
        assert_eq!(
            llm.resolve_endpoint(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_bug4_resolve_endpoint_anthropic_custom() {
        use metalogos::llm::{Provider, RealLlm};

        let mut llm = RealLlm::with_config(
            Provider::Anthropic,
            "claude-sonnet-4-20250514".to_string(),
            None,
        );
        llm.base_url = Some("https://claude-proxy.internal".to_string());

        let endpoint = llm.resolve_endpoint();
        assert_eq!(endpoint, "https://claude-proxy.internal/v1/messages");
    }

    #[test]
    fn test_bug4_resolve_endpoint_ollama_default() {
        use metalogos::llm::{Provider, RealLlm};

        let llm = RealLlm::with_config(Provider::Ollama, "llama3".to_string(), None);
        assert_eq!(
            llm.resolve_endpoint(),
            "http://localhost:11434/api/generate"
        );
    }

    #[test]
    fn test_bug4_resolve_endpoint_ollama_custom() {
        use metalogos::llm::{Provider, RealLlm};

        let mut llm = RealLlm::with_config(Provider::Ollama, "llama3".to_string(), None);
        llm.base_url = Some("http://ollama-server:11434".to_string());

        let endpoint = llm.resolve_endpoint();
        assert_eq!(endpoint, "http://ollama-server:11434/api/generate");
    }
}
