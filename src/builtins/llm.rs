// ── LLM / Voice builtins: call_llm, call_claude, llm_usage, whisper_transcribe, tts_send ──

use crate::interpreter::Value;

use super::core::expect_string_arg;

/// Send a request to Anthropic Claude Messages API.
/// Usage: call_claude(api_key, model, system_prompt, user_message) -> String
pub(crate) fn builtin_call_claude(args: &[Value]) -> Result<Value, String> {
    let api_key = expect_string_arg("call_claude", args, 0)?;
    let model = expect_string_arg("call_claude", args, 1)?;
    let system_prompt = expect_string_arg("call_claude", args, 2)?;
    let user_message = expect_string_arg("call_claude", args, 3)?;

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
        .header("x-api-key", &api_key)
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

    Ok(Value::String(content))
}

/// `call_llm(prompt, input)` — call the LLM backend with a prompt and input.
/// When METALOGOS_LLM_MOCK=true (default), returns "[MOCK: <prompt> | <input>]".
/// When METALOGOS_LLM_MOCK=false, calls the real LLM backend (30s timeout).
pub(crate) fn builtin_call_llm(args: &[Value]) -> Result<Value, String> {
    let prompt = match args.get(0) {
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

    // Check mock mode
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true); // Default: mock mode ON

    if mock_mode {
        Ok(Value::String(format!("[MOCK: {} | {}]", prompt, input)))
    } else {
        // Real LLM call
        let backend = crate::llm::create_llm_backend();
        backend
            .call(&prompt, &input)
            .map(Value::String)
            .map_err(|e| format!("call_llm() failed: {}", e))
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
    let (api_url, auth_header, auth_value) = match provider.as_str() {
        "groq" => (
            "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            "Authorization".to_string(),
            format!("Bearer {}", whisper_key),
        ),
        _ => (
            // openai
            "https://api.openai.com/v1/audio/transcriptions".to_string(),
            "Authorization".to_string(),
            format!("Bearer {}", whisper_key),
        ),
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

/// `tts_send(text, voice, bot_token, chat_id, mode?)` —
/// Convert text to speech via OpenAI TTS and send as voice note to Telegram.
pub(crate) fn builtin_tts_send(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("tts_send", args, 0)?;
    let voice = expect_string_arg("tts_send", args, 1)?;
    let bot_token = expect_string_arg("tts_send", args, 2)?;
    let chat_id = expect_string_arg("tts_send", args, 3)?;
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("tts_send(): OPENAI_API_KEY env var not set".to_string());
    }

    // Step 1: Call OpenAI TTS API
    let tts_body = serde_json::json!({
        "model": "tts-1",
        "input": text,
        "voice": voice,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("tts_send(): client error: {}", e))?;

    let tts_resp = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(tts_body.to_string())
        .send()
        .map_err(|e| format!("tts_send(): TTS request failed: {}", e))?;

    let status = tts_resp.status().as_u16();
    if status >= 400 {
        let err_body = tts_resp.text().unwrap_or_default();
        return Err(format!(
            "tts_send(): TTS API status {}: {}",
            status, err_body
        ));
    }

    let audio_bytes = tts_resp
        .bytes()
        .map_err(|e| format!("tts_send(): failed to read TTS audio: {}", e))?;

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
