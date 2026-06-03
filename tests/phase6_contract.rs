// ── Phase 6 Contract Tests ─────────────────────────────────────────
// Test parsing, semantic analysis, and opaque type enforcement
// for all Phase 6 features: server, templates, DB, encryption, auth, bot.

#[cfg(test)]
mod phase6_parsing_tests {
    #[test]
    fn test_61_mlogserver_minimal() {
        let source = r#"
mlogserver {
  port: 3000
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = metalogos::parser::parse(source).unwrap();
        assert_eq!(decls.len(), 1);
        if let metalogos::ast::Declaration::MlogServer(srv) = &decls[0] {
            assert_eq!(srv.port, 3000);
            assert_eq!(srv.routes.len(), 1);
        } else {
            panic!("expected MlogServer declaration");
        }
    }

    #[test]
    fn test_61_mlogserver_full() {
        let source = r#"
mlogserver {
  port: 8080
  middleware: [session, csrf, security_headers]
  route "/" method=GET { return "Home" }
  route "/login" method=POST { return "Login" }
  route "/admin" method=GET requires=[admin] { return "Admin" }
  route "/users" method=PUT requires=[admin] { return "Updated" }
  route "/webhook/telegram" method=POST { return "OK" }
}
"#;
        let decls = metalogos::parser::parse(source).unwrap();
        if let metalogos::ast::Declaration::MlogServer(srv) = &decls[0] {
            assert_eq!(srv.middleware.len(), 3);
            assert_eq!(srv.routes.len(), 5);
            assert_eq!(srv.routes[2].requires, vec!["admin".to_string()]);
            assert_eq!(srv.routes[3].method, "PUT");
        }
    }
}

#[cfg(test)]
mod phase6_template_tests {
    #[test]
    fn test_62_template_basic() {
        let source = r#"
template Hello(name: String) -> Html {
  <h1>Hello, {{ name }}</h1>
}
"#;
        let decls = metalogos::parser::parse(source).unwrap();
        if let metalogos::ast::Declaration::Template(t) = &decls[0] {
            assert_eq!(t.name, "Hello");
            assert_eq!(t.params.len(), 1);
            assert_eq!(t.params[0].name, "name");
            assert_eq!(t.return_type, "Html");
            assert!(t.body.contains("{{ name }}"));
        }
    }

    #[test]
    fn test_62_template_with_layout() {
        let source = r#"
template Layout(title: String, content: String) -> Html {
  <!DOCTYPE html>
  <html>
  <head><title>{{ title }}</title></head>
  <body>{{ content }}</body>
  </html>
}
"#;
        let decls = metalogos::parser::parse(source).unwrap();
        if let metalogos::ast::Declaration::Template(t) = &decls[0] {
            assert_eq!(t.params.len(), 2);
            assert!(t.body.contains("{{ title }}"));
            assert!(t.body.contains("{{ content }}"));
        }
    }
}

#[cfg(test)]
mod phase6_db_tests {
    #[test]
    fn test_63_db_block() {
        let source = r#"
db {
  pool_size: 10
  migrate: "./migrations"
}
"#;
        let decls = metalogos::parser::parse(source).unwrap();
        if let metalogos::ast::Declaration::Db(db) = &decls[0] {
            assert_eq!(db.pool_size, Some(10));
            assert_eq!(db.migrate, Some("./migrations".to_string()));
        }
    }
}

#[cfg(test)]
mod phase6_encryption_tests {
    use metalogos::interpreter::Value;

    #[test]
    fn test_64_secret_type_opaque() {
        let secret = Value::Secret(metalogos::interpreter::SecretString::new("my-api-key".to_string()));
        assert_eq!(secret.type_name(), "Secret");
        // Display must NOT expose the value
        assert_eq!(format!("{}", secret), "[Secret]");
    }

    #[test]
    fn test_64_hash_type_opaque() {
        let hash = Value::Hash("$argon2id$v=19$m=...".to_string());
        assert_eq!(hash.type_name(), "Hash");
        assert_eq!(format!("{}", hash), "[Hash]");
    }

    #[test]
    fn test_64_encrypted_type_opaque() {
        let enc = Value::Encrypted(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(enc.type_name(), "Encrypted");
        assert_eq!(format!("{}", enc), "[Encrypted]");
    }

    #[test]
    fn test_64_env_builtin_returns_secret() {
        std::env::set_var("TEST_MLOG_KEY", "secret_value");
        let interp = metalogos::interpreter::Interpreter::new();
        let result = interp.eval_expr(&metalogos::ast::Expr::FnCall(
            "env".to_string(),
            vec![metalogos::ast::Expr::StringLit("TEST_MLOG_KEY".to_string())],
        ));
        match result {
            Ok(Value::Secret(zs)) => assert_eq!(zs.as_str(), "secret_value"),
            other => panic!("env() should return Secret, got: {:?}", other),
        }
        std::env::remove_var("TEST_MLOG_KEY");
    }

    #[test]
    fn test_64_hash_password_builtin() {
        let interp = metalogos::interpreter::Interpreter::new();
        let result = interp.eval_expr(&metalogos::ast::Expr::FnCall(
            "hash_password".to_string(),
            vec![metalogos::ast::Expr::StringLit("password123".to_string())],
        ));
        match result {
            Ok(Value::Hash(_)) => {} // Hash is opaque — we can't see the value
            other => panic!("hash_password() should return Hash, got: {:?}", other),
        }
    }
}

#[cfg(test)]
mod phase73_real_encryption_contracts {
    use metalogos::interpreter::Value;
    use metalogos::builtins::Builtins;

    /// Helper: call a builtin by name
    fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
        let builtins = Builtins::new();
        match builtins.get(name) {
            Some(fn_ptr) => fn_ptr(args),
            None => panic!("builtin '{}' not found", name),
        }
    }

    /// Contract: verify_password with correct password returns true
    #[test]
    fn test_73_verify_correct_password() {
        // Step 1: hash a password
        let hash_result = call_builtin("hash_password", &[
            Value::String("correct_password".to_string()),
        ]);
        let hash_val = match hash_result {
            Ok(Value::Hash(h)) => h,
            other => panic!("hash_password() should return Hash, got: {:?}", other),
        };

        // Step 2: verify with correct password — returns true
        let verify_result = call_builtin("verify_password", &[
            Value::String("correct_password".to_string()),
            Value::Hash(hash_val),
        ]);
        match verify_result {
            Ok(Value::Bool(true)) => {} // Expected
            other => panic!("verify(correct_password, hash) should return true, got: {:?}", other),
        }
    }

    /// Contract: verify_password with wrong password returns false (NOT panic)
    #[test]
    fn test_73_verify_wrong_password_returns_false() {
        // Generate a real hash
        let hash_result = call_builtin("hash_password", &[
            Value::String("correct_password".to_string()),
        ]);
        let hash_val = match hash_result {
            Ok(Value::Hash(h)) => h,
            other => panic!("hash_password() should return Hash, got: {:?}", other),
        };

        // Verify with WRONG password → should return false, not panic
        let result = call_builtin("verify_password", &[
            Value::String("wrong_password".to_string()),
            Value::Hash(hash_val),
        ]);
        match result {
            Ok(Value::Bool(false)) => {} // Expected
            Ok(Value::Bool(true)) => panic!("verify(wrong) should return false, got true"),
            other => panic!("verify(wrong) should return Bool(false), got: {:?}", other),
        }
    }

    /// Contract: generate_key → encrypt → decrypt round-trip
    #[test]
    fn test_73_encrypt_decrypt_roundtrip() {
        // Step 1: generate_key()
        let key_result = call_builtin("generate_key", &[]);
        let key = match key_result {
            Ok(Value::Secret(k)) => k,
            other => panic!("generate_key() should return Secret, got: {:?}", other),
        };

        // Step 2: encrypt("secret data", key)
        let plaintext = "My highly confidential data";
        let encrypt_result = call_builtin("encrypt", &[
            Value::String(plaintext.to_string()),
            Value::Secret(metalogos::interpreter::SecretString::new(key.as_str().to_string())),
        ]);
        let encrypted = match encrypt_result {
            Ok(Value::Encrypted(data)) => data,
            other => panic!("encrypt() should return Encrypted, got: {:?}", other),
        };

        // Encrypted data should be different from plaintext
        assert_ne!(encrypted.len(), 0);
        // Should be at least 12 (nonce) + 16 (tag) bytes longer than plaintext
        assert!(encrypted.len() > plaintext.len());

        // Step 3: decrypt(encrypted, key) → original plaintext
        let decrypt_result = call_builtin("decrypt", &[
            Value::Encrypted(encrypted),
            Value::Secret(metalogos::interpreter::SecretString::new(key.as_str().to_string())),
        ]);
        match decrypt_result {
            Ok(Value::String(s)) => assert_eq!(s, plaintext),
            other => panic!("decrypt() should return original String, got: {:?}", other),
        }
    }

    /// Contract: decrypt with wrong key returns error (not panic)
    #[test]
    fn test_73_decrypt_wrong_key_returns_error() {
        // Generate key #1 and encrypt
        let key1_result = call_builtin("generate_key", &[]);
        let key1 = match key1_result {
            Ok(Value::Secret(k)) => k,
            other => panic!("generate_key() should return Secret, got: {:?}", other),
        };

        let encrypt_result = call_builtin("encrypt", &[
            Value::String("secret message".to_string()),
            Value::Secret(metalogos::interpreter::SecretString::new(key1.as_str().to_string())),
        ]);
        let encrypted = match encrypt_result {
            Ok(Value::Encrypted(data)) => data,
            other => panic!("encrypt() should return Encrypted, got: {:?}", other),
        };

        // Generate key #2 (different) and try to decrypt
        let key2_result = call_builtin("generate_key", &[]);
        let key2 = match key2_result {
            Ok(Value::Secret(k)) => k,
            other => panic!("generate_key() should return Secret, got: {:?}", other),
        };

        // Decrypt with wrong key → should return Err, not panic
        let decrypt_result = call_builtin("decrypt", &[
            Value::Encrypted(encrypted),
            Value::Secret(metalogos::interpreter::SecretString::new(key2.as_str().to_string())),
        ]);
        match decrypt_result {
            Err(msg) => {
                assert!(msg.contains("decrypt() failed"),
                    "decrypt with wrong key should return error, got: {}", msg);
            }
            Ok(v) => panic!("decrypt with wrong key should fail, but got: {:?}", v),
        }
    }

    /// Contract: generate_key produces 64 hex chars (256-bit key)
    #[test]
    fn test_73_generate_key_256bit() {
        let result = call_builtin("generate_key", &[]);
        match result {
            Ok(Value::Secret(k)) => {
                let hex_str = k.as_str();
                assert_eq!(hex_str.len(), 64, "Key should be 64 hex chars (256-bit)");
                // Verify it's valid hex
                hex::decode(hex_str).expect("Key should be valid hex");
            }
            other => panic!("generate_key() should return Secret, got: {:?}", other),
        }
    }

    /// Contract: print(Secret) should error
    #[test]
    fn test_73_print_secret_errors() {
        let result = call_builtin("print", &[
            Value::Secret(metalogos::interpreter::SecretString::new("hidden".to_string())),
        ]);
        match result {
            Err(msg) => assert!(msg.contains("expected String"),
                "print(Secret) should error, got: {}", msg),
            Ok(v) => panic!("print(Secret) should error, but got: {:?}", v),
        }
    }

    /// Contract: hash_password output format is PHC argon2id string
    #[test]
    fn test_73_hash_password_format() {
        let result = call_builtin("hash_password", &[
            Value::String("test_pass".to_string()),
        ]);
        match result {
            Ok(Value::Hash(h)) => {
                // PHC format: $argon2id$v=19$m=...
                assert!(h.starts_with("$argon2id$"),
                    "Hash should start with $argon2id$, got: {}", h);
                assert!(h.contains("$v=19$"),
                    "Hash should contain version $v=19$, got: {}", h);
            }
            other => panic!("hash_password() should return Hash, got: {:?}", other),
        }
    }

    /// Contract: each hash_password call produces different hash (random salt)
    #[test]
    fn test_73_hash_password_random_salt() {
        let h1 = call_builtin("hash_password", &[Value::String("same_pass".to_string())]);
        let h2 = call_builtin("hash_password", &[Value::String("same_pass".to_string())]);

        match (h1, h2) {
            (Ok(Value::Hash(a)), Ok(Value::Hash(b))) => {
                assert_ne!(a, b, "Two hashes of same password should differ (random salt)");
            }
            other => panic!("Both should return Hash, got: {:?}", other),
        }
    }
}

#[cfg(test)]
mod phase6_auth_tests {
    #[test]
    fn test_65_session_type_opaque() {
        use metalogos::interpreter::Value;
        use std::collections::HashMap;
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), "123".to_string());
        let session = Value::Session(data);
        assert_eq!(session.type_name(), "Session");
        assert_eq!(format!("{}", session), "[Session]");
    }

    #[test]
    fn test_65_csrf_required_for_post() {
        // Semantic check: mlogserver with POST routes but no csrf middleware → warning
        let source = r#"
mlogserver {
  middleware: [session, security_headers]
  route "/login" method=POST { return "OK" }
}
"#;
        let decls = metalogos::parser::parse(source).unwrap();
        let result = metalogos::semantic::check_program(&decls);
        assert!(result.warnings.iter().any(|w| w.contains("csrf")));
    }
}

#[cfg(test)]
mod phase6_bot_tests {
    #[test]
    fn test_66_webhook_route_parsing() {
        let source = r#"
mlogserver {
  port: 8080
  route "/webhook/telegram" method=POST {
    return "OK"
  }
}
"#;
        let decls = metalogos::parser::parse(source).unwrap();
        if let metalogos::ast::Declaration::MlogServer(srv) = &decls[0] {
            assert_eq!(srv.routes[0].path, "/webhook/telegram");
            assert_eq!(srv.routes[0].method, "POST");
        }
    }
}

#[cfg(test)]
mod phase6_xss_prevention_tests {
    use metalogos::server;

    #[test]
    fn test_html_escaping_script_tag() {
        assert_eq!(
            server::escape_html("<script>alert('XSS')</script>"),
            "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_html_escaping_ampersand() {
        assert_eq!(server::escape_html("foo & bar"), "foo &amp; bar");
    }

    #[test]
    fn test_html_escaping_quotes() {
        assert_eq!(server::escape_html(r#"value="test""#), r#"value=&quot;test&quot;"#);
    }

    #[test]
    fn test_template_xss_prevention() {
        let template = "<div>{{ content }}</div>";
        let mut vars = std::collections::HashMap::new();
        vars.insert("content".to_string(), "<script>alert(1)</script>".to_string());
        let result = server::render_template(template, &vars);
        assert!(!result.contains("<script>"));
        assert!(result.contains("&lt;script&gt;"));
    }
}
