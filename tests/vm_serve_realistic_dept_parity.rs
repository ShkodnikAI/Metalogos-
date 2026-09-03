// VM-for-serve: реалистичное сравнение TW vs VM на многодепартаментной
// структуре, близкой к настоящей FOSVED (диспетчер + условная
// маршрутизация по query_param + несколько импортированных модулей).
//
// Проверяет каждую условную ветку отдельно — наряд №161 показал, что
// TW-путь под VM-параллелью может "молчать" именно на непроверенных
// ветках, а не потому что реально резолвит их иначе.

use metalogos::server::ServeBackend;

const DISPATCHER_SOURCE: &str = include_str!("../examples/vm_serve_realistic_dispatcher.mlog");

async fn start_server(
    backend: ServeBackend,
) -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    metalogos::server::run_test_server_with_backend(DISPATCHER_SOURCE, backend)
        .await
        .expect("test server should start")
}

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body should be readable");
    (status, body)
}

async fn assert_all_depts_parity(backend: ServeBackend, backend_name: &str) {
    let (port, _handle) = start_server(backend).await;
    for (dept, expected) in [
        ("a", "dept-a: hi"),
        ("b", "dept-b: hi"),
        ("c", "dept-c: hi"),
    ] {
        let (status, body) = http_get(port, &format!("/dispatch?dept={}&q=hi", dept)).await;
        assert_eq!(
            status, 200,
            "{}: dept={} должен вернуть 200",
            backend_name, dept
        );
        assert_eq!(
            body, expected,
            "{}: dept={} должен вызвать правильный обработчик",
            backend_name, dept
        );
    }
    let (status, body) = http_get(port, "/dispatch?dept=z&q=hi").await;
    assert_eq!(
        status, 200,
        "{}: неизвестный dept всё равно 200",
        backend_name
    );
    assert_eq!(body, "unknown dept", "{}: fallback-ветка", backend_name);
}

#[tokio::test]
async fn tw_serves_all_dept_branches_correctly() {
    assert_all_depts_parity(ServeBackend::Interpreter, "TW").await;
}

#[tokio::test]
async fn vm_serves_all_dept_branches_correctly() {
    assert_all_depts_parity(ServeBackend::Vm, "VM").await;
}

#[tokio::test]
async fn tw_vm_full_parity_across_all_branches() {
    let (tw_port, _tw_handle) = start_server(ServeBackend::Interpreter).await;
    let (vm_port, _vm_handle) = start_server(ServeBackend::Vm).await;

    for dept in ["a", "b", "c", "z"] {
        let path = format!("/dispatch?dept={}&q=parity-check", dept);
        let (tw_status, tw_body) = http_get(tw_port, &path).await;
        let (vm_status, vm_body) = http_get(vm_port, &path).await;
        assert_eq!(
            tw_status, vm_status,
            "dept={}: статус должен совпадать между бэкендами",
            dept
        );
        assert_eq!(
            tw_body, vm_body,
            "dept={}: тело ответа должно совпадать между бэкендами",
            dept
        );
    }
}
