use metalogos::builtins::check_builtin_arity;

#[test]
fn registry_arity_spot_checks() {
    // http_post: 2..5
    assert!(
        check_builtin_arity("http_post", 2).is_ok(),
        "http_post(2) should be ok"
    );
    assert!(
        check_builtin_arity("http_post", 3).is_ok(),
        "http_post(3) should be ok"
    );
    assert!(
        check_builtin_arity("http_post", 4).is_ok(),
        "http_post(4) should be ok"
    );
    assert!(
        check_builtin_arity("http_post", 5).is_ok(),
        "http_post(5) should be ok"
    );
    assert!(
        check_builtin_arity("http_post", 1).is_err(),
        "http_post(1) should be err"
    );
    assert!(
        check_builtin_arity("http_post", 6).is_err(),
        "http_post(6) should be err"
    );
    // require: 1..2
    assert!(check_builtin_arity("require", 1).is_ok());
    assert!(check_builtin_arity("require", 2).is_ok());
    assert!(check_builtin_arity("require", 0).is_err());
    // upper: exact 1
    assert!(check_builtin_arity("upper", 1).is_ok());
    assert!(check_builtin_arity("upper", 0).is_err());
    assert!(check_builtin_arity("upper", 2).is_err());
}
