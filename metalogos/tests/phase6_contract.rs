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
        let secret = Value::Secret("my-api-key".to_string());
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
            Ok(Value::Secret(s)) => assert_eq!(s, "secret_value"),
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
