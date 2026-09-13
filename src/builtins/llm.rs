// ── LLM / Voice builtins: call_llm, call_claude, llm_usage, whisper_transcribe, tts_generate, tts_send ──

use crate::interpreter::Value;

use super::core::expect_string_arg;

/// `call_claude(api_key, model, system_prompt, user_message)` — Send a
/// request to the Anthropic Claude Messages API.
/// Наряд №276: the HTTP exchange is traced at this single point — status,
/// latency, model, and usage (Anthropic reports `usage.input_tokens` /
/// `usage.output_tokens`; absent fields stay absent — honest data).
/// Argument-type errors are NOT traced: no request reached any provider.
pub(crate) fn builtin_call_claude(args: &[Value]) -> Result<Value, String> {
    let api_key = expect_string_arg("call_claude", args, 0)?;
    let model = expect_string_arg("call_claude", args, 1)?;
    let system_prompt = expect_string_arg("call_claude", args, 2)?;
    let user_message = expect_string_arg("call_claude", args, 3)?;

    let t0 = std::time::Instant::now();
    let result = call_claude_impl(&api_key, &model, &system_prompt, &user_message);
    let (input_tokens, output_tokens) = match &result {
        Ok((_, usage)) => (
            usage.map(|u| u.input_tokens),
            usage.map(|u| u.output_tokens),
        ),
        Err(_) => (None, None),
    };
    crate::llm::trace_llm_call(&crate::llm::LlmTraceEvent {
        provider_name: Some("anthropic"),
        model: Some(&model),
        input_tokens,
        output_tokens,
        latency_ms: t0.elapsed().as_millis() as u64,
        status: if result.is_ok() { "ok" } else { "error" },
        cache: "miss",
        provider_alias: None,
    });
    result.map(|(text, _)| Value::String(text))
}

/// HTTP exchange for call_claude: returns `(text, usage)` — usage is Some
/// only when the response carried Anthropic's usage block.
fn call_claude_impl(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
) -> Result<(String, Option<crate::llm::ProviderTokenUsage>), String> {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{"role": "user", "content": user_message}]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("call_claude(): failed to create client: {}", e))?;

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("call_claude(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "call_claude() returned status {}: {}",
            status, resp_body
        ));
    }

    // Parse response and extract content[0].text
    let parsed: serde_json::Value = serde_json::from_str(&resp_body)
        .map_err(|e| format!("call_claude(): JSON parse error: {}", e))?;

    let content = parsed["content"][0]["text"]
        .as_str()
        .unwrap_or("Claude API returned an unexpected response format")
        .to_string();

    // Наряд №276: usage from the SAME parsed body (honest — absent stays None).
    let usage = crate::llm::extract_usage_from_anthropic_body(&parsed);

    Ok((content, usage))
}

/// `call_llm(prompt, input)` — call the LLM backend with a prompt and input.
/// Наряд №4: tries GLOBAL_SMART_ROUTER first; falls back to legacy create_llm_backend().
/// When no SmartRouter is installed and METALOGOS_LLM_MOCK=true (default), returns mock.
pub(crate) fn builtin_call_llm(args: &[Value]) -> Result<Value, String> {
    let prompt = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "call_llm() expected String as prompt, got {}",
                other.type_name()
            ))
        }
        None => return Err("call_llm() requires at least 1 argument (prompt)".to_string()),
    };
    let input = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => String::new(),
    };

    // Наряд №4: try SmartRouter first
    // Наряд #156: no sandbox timeout for builtin call_llm — None
    if let Some(result) = crate::llm::call_via_smart_router(&prompt, &input, None, None) {
        return result.map(Value::String);
    }

    // Fallback: legacy path (no SmartRouter). Traced here (Наряд №276);
    // the SmartRouter path traces inside SmartRouter::call — one line per
    // actual LLM call, never both.
    // Check mock mode
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true); // Default: mock mode ON

    let t0 = std::time::Instant::now();
    if mock_mode {
        let result = Ok(Value::String(format!("[MOCK: {} | {}]", prompt, input)));
        crate::llm::trace_llm_call(&crate::llm::LlmTraceEvent {
            provider_name: Some("mock"),
            model: None,
            input_tokens: None,
            output_tokens: None,
            latency_ms: t0.elapsed().as_millis() as u64,
            status: "ok",
            cache: "miss",
            provider_alias: None,
        });
        result
    } else {
        // Real LLM call
        let backend = crate::llm::create_llm_backend();
        let result = backend
            .call(&prompt, &input)
            .map(Value::String)
            .map_err(|e| format!("call_llm() failed: {}", e));
        let model_env = std::env::var("METALOGOS_LLM_MODEL").ok();
        crate::llm::trace_llm_call(&crate::llm::LlmTraceEvent {
            provider_name: Some(crate::llm::provider_env_name()),
            model: model_env.as_deref(),
            input_tokens: None,
            output_tokens: None,
            latency_ms: t0.elapsed().as_millis() as u64,
            status: if result.is_ok() { "ok" } else { "error" },
            cache: "miss",
            provider_alias: None,
        });
        result
    }
}

/// Наряд №4: `llm_usage()` — returns LLM usage statistics as a Struct.
/// Returns: { total_calls: Float, total_tokens: Float, total_errors: Float, providers: List }
pub(crate) fn builtin_llm_usage(_args: &[Value]) -> Result<Value, String> {
    let report = crate::llm::global_llm_usage_report();

    let mut fields = std::collections::HashMap::new();
    fields.insert("total_calls".to_string(), Value::Float(report.total_calls));
    fields.insert(
        "total_tokens".to_string(),
        Value::Float(report.total_tokens),
    );
    fields.insert(
        "total_errors".to_string(),
        Value::Float(report.total_errors),
    );
    fields.insert(
        "cache_hits_semantic".to_string(),
        Value::Float(report.cache_hits_semantic),
    );
    fields.insert(
        "canary_leaks".to_string(),
        Value::Float(report.canary_leaks),
    );

    let providers: Vec<Value> = report
        .providers
        .iter()
        .map(|p| {
            let mut pf = std::collections::HashMap::new();
            pf.insert("alias".to_string(), Value::String(p.alias.clone()));
            pf.insert("calls".to_string(), Value::Float(p.calls as f64));
            pf.insert("tokens".to_string(), Value::Float(p.tokens as f64));
            pf.insert("errors".to_string(), Value::Float(p.errors as f64));
            pf.insert("avg_latency_ms".to_string(), Value::Float(p.avg_latency_ms));
            pf.insert("health_score".to_string(), Value::Float(p.health_score));
            Value::Struct {
                type_name: "ProviderUsage".to_string(),
                fields: pf,
            }
        })
        .collect();
    fields.insert("providers".to_string(), Value::List(providers));

    Ok(Value::Struct {
        type_name: "LlmUsage".to_string(),
        fields,
    })
}

/// `whisper_transcribe(file_id, bot_token, whisper_key, provider?)` —
/// Transcribe a Telegram voice message via Whisper API.
/// Naryad #279 fact-check: the registry declared min arity 1 while the
/// implementation has always required THREE string args (file_id,
/// bot_token, whisper_key) plus an optional provider — a 1-arg call passed
/// `mlog check` and exploded at runtime. Registry now says 3..4.
/// STT symmetry with №279 TTS: `METALOGOS_STT_BASE_URL` overrides the
/// transcription API base (mock servers / self-host proxies); the path
/// suffix `/audio/transcriptions` is appended, mirroring TTS.
pub(crate) fn builtin_whisper_transcribe(args: &[Value]) -> Result<Value, String> {
    let file_id = expect_string_arg("whisper_transcribe", args, 0)?;
    let bot_token = expect_string_arg("whisper_transcribe", args, 1)?;
    let whisper_key = expect_string_arg("whisper_transcribe", args, 2)?;
    let provider = match args.get(3) {
        Some(Value::String(s)) => s.clone(),
        _ => "openai".to_string(),
    };

    // Step 1: Get file path from Telegram
    let tg_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("whisper_transcribe(): client error: {}", e))?;

    let get_file_url = format!(
        "https://api.telegram.org/bot{}/getFile?file_id={}",
        bot_token, file_id
    );
    let tg_resp = tg_client
        .get(&get_file_url)
        .send()
        .map_err(|e| format!("whisper_transcribe(): Telegram getFile failed: {}", e))?;
    let tg_body: serde_json::Value = serde_json::from_str(&tg_resp.text().unwrap_or_default())
        .map_err(|e| format!("whisper_transcribe(): Telegram response parse error: {}", e))?;

    let file_path = tg_body
        .get("result")
        .and_then(|r| r.get("file_path"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();

    if file_path.is_empty() {
        return Err("whisper_transcribe(): Telegram returned empty file_path".to_string());
    }

    // Step 2: Download the file
    let download_url = format!(
        "https://api.telegram.org/file/bot{}/{}",
        bot_token, file_path
    );
    let audio_bytes = tg_client
        .get(&download_url)
        .send()
        .map_err(|e| format!("whisper_transcribe(): download failed: {}", e))?
        .bytes()
        .map_err(|e| format!("whisper_transcribe(): read bytes failed: {}", e))?;

    // Step 3: Send to Whisper API
    // Naryad #279: METALOGOS_STT_BASE_URL overrides the provider default
    // (same convention as METALOGOS_TTS_BASE_URL for synthesis).
    let stt_base = std::env::var("METALOGOS_STT_BASE_URL").ok();
    let (api_url, auth_header, auth_value) = match provider.as_str() {
        "groq" => {
            let url = stt_base
                .clone()
                .unwrap_or_else(|| "https://api.groq.com/openai/v1".to_string());
            (
                format!("{}/audio/transcriptions", url.trim_end_matches('/')),
                "Authorization".to_string(),
                format!("Bearer {}", whisper_key),
            )
        }
        _ => {
            let url = stt_base.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            (
                format!("{}/audio/transcriptions", url.trim_end_matches('/')),
                "Authorization".to_string(),
                format!("Bearer {}", whisper_key),
            )
        }
    };

    // Use multipart form
    let whisper_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("whisper_transcribe(): whisper client error: {}", e))?;

    let model = if provider == "groq" {
        "whisper-large-v3"
    } else {
        "whisper-1"
    };
    let mut form = reqwest::blocking::multipart::Form::new();
    form = form.text("model", model.to_string());
    let part =
        reqwest::blocking::multipart::Part::bytes(audio_bytes.to_vec()).file_name("audio.ogg");
    form = form.part("file", part);

    let whisper_resp = whisper_client
        .post(&api_url)
        .header(auth_header, auth_value)
        .multipart(form)
        .send()
        .map_err(|e| format!("whisper_transcribe(): whisper request failed: {}", e))?;

    let status = whisper_resp.status().as_u16();
    let whisper_body = whisper_resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "whisper_transcribe(): whisper API status {}: {}",
            status, whisper_body
        ));
    }

    // Parse response to extract text
    let parsed: serde_json::Value = serde_json::from_str(&whisper_body)
        .map_err(|e| format!("whisper_transcribe(): whisper response parse error: {}", e))?;
    let text = parsed
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Value::String(text))
}

/// `tts_generate(text, voice, provider?, model?) -> String(path)` —
/// Speech synthesis WITHOUT delivery: writes the audio file into the file
/// sandbox (write_file semantics: Naryad #252 symlink-safe write) and
/// returns the sandbox-relative path. Providers v1: `"openai"` with models
/// `tts-1` (default) / `tts-1-hd` / `gpt-4o-mini-tts` (model is a plain
/// argument, not a hardcode). Key: `METALOGOS_TTS_API_KEY`, falling back to
/// `OPENAI_API_KEY` (backward compatible with tts_send). Base URL:
/// `METALOGOS_TTS_BASE_URL` overrides `https://api.openai.com/v1` — the
/// `/audio/speech` suffix is appended (mock servers, self-host proxies).
/// Response format: the provider default (MP3 for OpenAI tts-*), extension
/// `.mp3`. Delivery (Telegram sendVoice/sendAudio) is tts_send's job.
pub(crate) fn builtin_tts_generate(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("tts_generate", args, 0)?;
    let voice = expect_string_arg("tts_generate", args, 1)?;
    let provider = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => "openai".to_string(),
    };
    let model = match args.get(3) {
        Some(Value::String(s)) => s.clone(),
        _ => "tts-1".to_string(),
    };
    if provider != "openai" {
        return Err(format!(
            "tts_generate(): unknown provider '{}' — supported in v1: 'openai' (ADR-documented scope)",
            provider
        ));
    }
    let audio_bytes = tts_synth(&text, &voice, &model)?;

    // Write into the file sandbox with the EXACT write_file semantics
    // (Naryad #252): sandbox-resolve, create parent dirs, symlink-safe open.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let fname = format!("tts_{}.mp3", ts);
    let safe_path = super::io::sandbox_path_ex(&fname, super::io::SandboxMode::ForWrite)
        .map_err(super::io::sandbox_violation)?;
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort, same as write_file
    }
    let mut file = super::io::open_sandbox_write(&safe_path, false)?;
    std::io::Write::write_all(&mut file, &audio_bytes)
        .map_err(|e| format!("tts_generate(): failed to write audio file: {}", e))?;

    Ok(Value::String(fname))
}

/// Shared synthesis HTTP exchange for tts_generate/tts_send (Naryad #279).
/// Returns the raw audio bytes. Base URL override + key resolution live here
/// so delivery (tts_send) and pure synthesis (tts_generate) cannot drift.
fn tts_synth(text: &str, voice: &str, model: &str) -> Result<Vec<u8>, String> {
    let api_key = match std::env::var("METALOGOS_TTS_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => std::env::var("OPENAI_API_KEY").unwrap_or_default(),
    };
    if api_key.is_empty() {
        return Err(
            "tts_generate(): no API key — set METALOGOS_TTS_API_KEY (or OPENAI_API_KEY)"
                .to_string(),
        );
    }
    let base = std::env::var("METALOGOS_TTS_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let url = format!("{}/audio/speech", base.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "input": text,
        "voice": voice,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("tts_generate(): client error: {}", e))?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("tts_generate(): TTS request failed: {}", e))?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let err_body = resp.text().unwrap_or_default();
        return Err(format!(
            "tts_generate(): TTS API status {}: {}",
            status, err_body
        ));
    }

    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("tts_generate(): failed to read TTS audio: {}", e))
}

/// `tts_send(text, voice, bot_token, chat_id, mode?)` —
/// Convert text to speech via the OpenAI TTS API and send as voice note to
/// Telegram. Naryad #279: delivery-only convenience — synthesis is
/// DELEGATED to the tts_synth helper (same exchange tts_generate uses, so
/// base-URL/key overrides behave identically); REFERENCE marks this builtin
/// as "delivery convenience". Kept for backward compatibility.
pub(crate) fn builtin_tts_send(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("tts_send", args, 0)?;
    let voice = expect_string_arg("tts_send", args, 1)?;
    let bot_token = expect_string_arg("tts_send", args, 2)?;
    let chat_id = expect_string_arg("tts_send", args, 3)?;

    // Step 1: synthesize (was inline OpenAI TTS call; model stays "tts-1")
    let audio_bytes = tts_synth(&text, &voice, "tts-1").map_err(|e| {
        // Keep the historical error prefix for the key-missing case.
        if e.contains("no API key") {
            "tts_send(): OPENAI_API_KEY env var not set".to_string()
        } else {
            e.replace("tts_generate():", "tts_send():")
        }
    })?;

    // Step 2: Send as voice note to Telegram via sendVoice
    // sendVoice (not sendAudio) displays as voice message bubble in Telegram.
    // Optional 5th arg "audio" switches back to sendAudio (audio player).
    let send_as = match args.get(4) {
        Some(Value::String(s)) if s == "audio" => "audio",
        _ => "voice",
    };
    let (field_name, endpoint) = match send_as {
        "audio" => ("audio", "sendAudio"),
        _ => ("voice", "sendVoice"),
    };
    let mut form = reqwest::blocking::multipart::Form::new();
    form = form.text("chat_id", chat_id.clone());
    let audio_part =
        reqwest::blocking::multipart::Part::bytes(audio_bytes.to_vec()).file_name("speech.ogg");
    form = form.part(field_name, audio_part);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("tts_send(): client error: {}", e))?;
    let tg_resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/{}",
            bot_token, endpoint
        ))
        .multipart(form)
        .send()
        .map_err(|e| format!("tts_send(): Telegram {} failed: {}", endpoint, e))?;

    let tg_status = tg_resp.status().as_u16();
    let tg_body = tg_resp.text().unwrap_or_default();

    if tg_status >= 400 {
        return Err(format!(
            "tts_send(): Telegram status {}: {}",
            tg_status, tg_body
        ));
    }

    Ok(Value::String(tg_body))
}
