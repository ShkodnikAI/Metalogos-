// ── Наряд №133: Контракт — push/len/first/last работают корректно ──
//
// После удаления сломанной обёртки push(items, item) из std/collections.mlog
// (которая вызывала несуществующий __push) и удаления stub-записей
// __push/__list_len из реестра:
//
// C1: push() builtin работает напрямую — возвращает новый список с добавленным элементом
// C2: len() builtin работает на списке — возвращает количество элементов
// C3: first()/last() работают через import std/collections (обёртки через __first/__last)
// C4: push() работает после import std/collections — обёртка удалена, builtin глобален

use std::path::Path;

/// Helper: run .mlog source via tree-walking interpreter with base_dir for imports.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn project_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

// ── C1: push() builtin работает напрямую ─────────────────────────
#[test]
fn test_n133_push_builtin_direct() {
    let root = project_root();
    let source = r#"
        pattern Test(_: String) -> String {
            let items = make_list("a", "b")
            let result = push(items, "c")
            let first_item = result[0]
            let second_item = result[1]
            let third_item = result[2]
            return first_item + "," + second_item + "," + third_item
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let output = run_tw(source, &root).unwrap();
    assert_eq!(output.as_deref(), Some("a,b,c"));
}

// ── C2: len() builtin работает на списке ─────────────────────────
#[test]
fn test_n133_len_builtin_on_list() {
    let root = project_root();
    let source = r#"
        pattern Test(_: String) -> String {
            let items = make_list("x", "y", "z")
            let n = len(items)
            return to_string(n)
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let output = run_tw(source, &root).unwrap();
    assert_eq!(output.as_deref(), Some("3"));
}

// ── C3: first()/last() работают через import std/collections ──────
#[test]
fn test_n133_first_last_via_std_collections() {
    let root = project_root();
    let source = r#"
        import std/collections

        pattern Test(_: String) -> String {
            let items = make_list("alpha", "beta", "gamma")
            let f = first(items)
            let l = last(items)
            return f + "," + l
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let output = run_tw(source, &root).unwrap();
    assert_eq!(output.as_deref(), Some("alpha,gamma"));
}

// ── C4: push() работает после import std/collections ─────────────
// Обёртка push удалена из std/collections.mlog, но builtin push
// глобально доступен — вызов должен пройти напрямую.
#[test]
fn test_n133_push_after_std_import() {
    let root = project_root();
    let source = r#"
        import std/collections

        pattern Test(_: String) -> String {
            let items = make_list("hello")
            let updated = push(items, "world")
            let n = len(updated)
            return to_string(n) + ":" + updated[0] + "," + updated[1]
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let output = run_tw(source, &root).unwrap();
    assert_eq!(output.as_deref(), Some("2:hello,world"));
}

// ── C5: Весь std/collections.mlog работает целиком при импорте ────
// Комбинированный тест: first + last + push + len + make_list.
#[test]
fn test_n133_full_std_collections_workflow() {
    let root = project_root();
    let source = r#"
        import std/collections

        pattern Test(_: String) -> String {
            let items = make_list("first")
            let items2 = push(items, "second")
            let items3 = push(items2, "third")
            let f = first(items3)
            let l = last(items3)
            let n = len(items3)
            return f + "-" + l + "-" + to_string(n)
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let output = run_tw(source, &root).unwrap();
    assert_eq!(output.as_deref(), Some("first-third-3"));
}
