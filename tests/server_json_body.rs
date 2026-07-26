// ── Наряд №3: json_body() integration tests ──────────────────────
// Verifies that json_body() correctly parses JSON from POST requests
// and supports nested field access via dot notation.
//
// Contract (Telegram webhook pattern):
//   POST /webhook with {"message":{"text":"hello","chat":{"id":12345}}}
//   → "Got: hello from 12345"

/// Source for the Telegram webhook contract.
const SOURCE_WEBHOOK: &str = r#"
mlogserver {
  port: 8091
  route "/webhook" method=POST {
    let data = json_body()
    let text = data.message.text
    let chat_id = data.message.chat.id
    respond("Got: " + text + " from " + to_string(chat_id))
  }
}
"#;

/// Source with a simpler flat JSON body.
const SOURCE_FLAT_JSON: &str = r#"
mlogserver {
  port: 8091
  route "/echo" method=POST {
    let data = json_body()
    respond(data.name)
  }
}
"#;

/// Source testing all JSON types: string, number, bool, null, array, nested.
const SOURCE_ALL_TYPES: &str = r#"
mlogserver {
  port: 8091
  route "/types" method=POST {
    let data = json_body()
    respond(data.status)
  }
}
"#;

// ── Unit test: recursive JSON → Value conversion ──────────────────────

#[test]
fn test_json_to_value_string() {
    let json = serde_json::json!("hello");
    let val = metalogos::server::json_value_to_value(&json);
    match val {
        metalogos::interpreter::Value::String(s) => assert_eq!(s, "hello"),
        other => panic!("expected String, got: {:?}", other),
    }
}

#[test]
fn test_json_to_value_number() {
    let json = serde_json::json!(42);
    let val = metalogos::server::json_value_to_value(&json);
    match val {
        metalogos::interpreter::Value::Float(f) => assert_eq!(f, 42.0),
        other => panic!("expected Float, got: {:?}", other),
    }
}

#[test]
fn test_json_to_value_bool() {
    let json = serde_json::json!(true);
    let val = metalogos::server::json_value_to_value(&json);
    match val {
        metalogos::interpreter::Value::Bool(b) => assert_eq!(b, true),
        other => panic!("expected Bool, got: {:?}", other),
    }
}

#[test]
fn test_json_to_value_null() {
    let json = serde_json::json!(null);
    let val = metalogos::server::json_value_to_value(&json);
    match val {
        metalogos::interpreter::Value::Unit => {}
        other => panic!("expected Unit for null, got: {:?}", other),
    }
}

#[test]
fn test_json_to_value_array() {
    let json = serde_json::json!([1, 2, 3]);
    let val = metalogos::server::json_value_to_value(&json);
    match val {
        metalogos::interpreter::Value::List(items) => {
            assert_eq!(items.len(), 3);
        }
        other => panic!("expected List, got: {:?}", other),
    }
}

#[test]
fn test_json_to_value_nested_object() {
    let json = serde_json::json!({
        "message": {
            "text": "hello",
            "chat": {
                "id": 12345
            }
        }
    });
    let val = metalogos::server::json_value_to_value(&json);

    // Top level should be Struct
    if let metalogos::interpreter::Value::Struct { ref fields, .. } = val {
        assert!(fields.contains_key("message"));

        // message should be a nested Struct
        if let metalogos::interpreter::Value::Struct { ref fields, .. } = fields["message"] {
            assert!(fields.contains_key("text"));
            assert!(fields.contains_key("chat"));

            // text should be "hello"
            match &fields["text"] {
                metalogos::interpreter::Value::String(s) => assert_eq!(s, "hello"),
                other => panic!("message.text should be String, got: {:?}", other),
            }

            // chat should be a nested Struct
            if let metalogos::interpreter::Value::Struct { ref fields, .. } = fields["chat"] {
                assert!(fields.contains_key("id"));
                match &fields["id"] {
                    metalogos::interpreter::Value::Float(f) => assert_eq!(*f, 12345.0),
                    other => panic!("chat.id should be Float, got: {:?}", other),
                }
            } else {
                panic!("chat should be Struct");
            }
        } else {
            panic!("message should be Struct");
        }
    } else {
        panic!("top level should be Struct, got: {:?}", val);
    }
}

#[test]
fn test_json_to_value_deeply_nested() {
    let json = serde_json::json!({
        "a": { "b": { "c": "deep" } }
    });
    let val = metalogos::server::json_value_to_value(&json);

    // a.b.c → "deep" via chained get_field
    let a = val.get_field("a").cloned().unwrap();
    let b = a.get_field("b").cloned().unwrap();
    let c = b.get_field("c").cloned().unwrap();
    match c {
        metalogos::interpreter::Value::String(s) => assert_eq!(s, "deep"),
        other => panic!("expected 'deep', got: {:?}", other),
    }
}

// ── Integration tests: live server + reqwest ─────────────────────────

#[tokio::test]
async fn test_webhook_telegram_contract() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_WEBHOOK)
        .await
        .expect("server should start");

    let url = format!("http://127.0.0.1:{}/webhook", port);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "message": {
                "text": "hello",
                "chat": { "id": 12345 }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Got: hello from 12345");
}

#[tokio::test]
async fn test_webhook_flat_json() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_FLAT_JSON)
        .await
        .expect("server should start");

    let url = format!("http://127.0.0.1:{}/echo", port);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({"name": "Fosved"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Fosved");
}

#[tokio::test]
async fn test_webhook_array_field() {
    let source = r#"
mlogserver {
  port: 8091
  route "/items" method=POST {
    let data = json_body()
    let first = get(data.items, 0)
    respond(first)
  }
}
"#;
    let (port, _handle) = metalogos::server::run_test_server(source)
        .await
        .expect("server should start");

    let url = format!("http://127.0.0.1:{}/items", port);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({"items": ["alpha", "beta"]}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "alpha");
}

#[tokio::test]
async fn test_webhook_empty_body_returns_empty_struct() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_WEBHOOK)
        .await
        .expect("server should start");

    let url = format!("http://127.0.0.1:{}/webhook", port);
    let client = reqwest::Client::new();
    let resp = client.post(&url).send().await.unwrap();

    // Empty body: json_body() returns empty struct
    // data.message.text → field not found → error → 500
    assert_eq!(resp.status(), 500);
}

#[tokio::test]
async fn test_webhook_bool_field() {
    let source = r#"
mlogserver {
  port: 8091
  route "/check" method=POST {
    let data = json_body()
    let active = data.active
    if active { respond("yes") } else { respond("no") }
  }
}
"#;
    let (port, _handle) = metalogos::server::run_test_server(source)
        .await
        .expect("server should start");

    let url = format!("http://127.0.0.1:{}/check", port);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({"active": true}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "yes");
}

#[tokio::test]
async fn test_webhook_null_field_becomes_unit() {
    let source = r#"
mlogserver {
  port: 8091
  route "/null" method=POST {
    let data = json_body()
    let value = data.maybe
    respond(to_string(value))
  }
}
"#;
    let (port, _handle) = metalogos::server::run_test_server(source)
        .await
        .expect("server should start");

    let url = format!("http://127.0.0.1:{}/null", port);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({"maybe": null}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "()"); // Unit displays as "()"
}
