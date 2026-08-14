// НАРЯД №76 — http_download binary contract tests
//
// Four scenarios against a local Python test server
// (tests/p76_http_download_server.py, port 18776):
//
// 1. Successful download — server returns 200 + binary, http_download
//    returns true, file exists on disk.
// 2. Byte-for-byte match — downloaded file is compared against the
//    expected content (bytes 0x00..0xFF, generated deterministically
//    both in Python and here). NOT text comparison — the content is
//    intentionally non-UTF-8, so any accidental resp.text() in the
//    implementation would corrupt the bytes and this test would catch
//    it.
// 3. HTTP 404 — http_download returns false, no file written.
// 4. Sandbox escape attempt (dest_path = "../outside.bin") —
//    http_download returns false, file is NOT created outside the
//    working directory.
//
// All tests share port 18776, so they MUST run serially — without
// #[serial_test::serial], two tests would race on the port and one
// would fail to spawn its server. Same pattern as naryad_71_http_retry.

#[cfg(test)]
mod tests {
    use metalogos::builtins::Builtins;
    use metalogos::interpreter::Value;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::thread;
    use std::time::Duration;

    const SERVER_PORT: u16 = 18776;
    const BASE_URL: &str = "http://127.0.0.1:18776";

    /// RAII guard: kills the child server process on drop.
    struct ServerGuard(Child);

    impl ServerGuard {
        fn spawn() -> Self {
            let child = Command::new("python3")
                .arg("tests/p76_http_download_server.py")
                .arg("--port")
                .arg(SERVER_PORT.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("failed to start p76 test server");
            // Wait for server to be ready
            thread::sleep(Duration::from_millis(500));
            Self(child)
        }
    }

    impl Drop for ServerGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Expected binary content: bytes 0x00..0xFF in order.
    /// Matches the Python `make_binary_content()` in
    /// tests/p76_http_download_server.py — same formula on both sides.
    fn expected_binary_content() -> Vec<u8> {
        (0u8..=255u8).collect()
    }

    /// Helper: clean up a file if it exists (ignores errors).
    fn cleanup_file(path: &str) {
        let _ = std::fs::remove_file(path);
    }

    /// Helper: clean up a directory if it exists (ignores errors).
    fn cleanup_dir(path: &str) {
        let _ = std::fs::remove_dir_all(path);
    }

    /// Helper: get the http_download builtin.
    fn http_download_fn() -> metalogos::builtins::BuiltinFn {
        let b = Builtins::new();
        *b.get("http_download")
            .expect("http_download must be registered")
    }

    // ── Scenario 1: Successful download ─────────────────────────────

    #[test]
    #[serial]
    fn test_http_download_success() {
        let _server = ServerGuard::spawn();
        let dest = "downloads/p76_test_success.bin";
        cleanup_file(dest);
        cleanup_dir("downloads");

        let http_download = http_download_fn();
        let result = http_download(&[
            Value::String(format!("{}/file.bin", BASE_URL)),
            Value::String(dest.to_string()),
        ]);

        match result {
            Ok(Value::Bool(true)) => {}
            other => panic!(
                "Expected Ok(Bool(true)) for successful download, got: {:?}",
                other
            ),
        }

        // File must exist on disk
        let path = PathBuf::from(dest);
        assert!(path.exists(), "downloaded file should exist at {:?}", path);

        // Clean up
        cleanup_file(dest);
        cleanup_dir("downloads");
    }

    // ── Scenario 2: Byte-for-byte match (non-UTF-8 binary) ──────────

    #[test]
    #[serial]
    fn test_http_download_byte_for_byte_match() {
        let _server = ServerGuard::spawn();
        let dest = "downloads/p76_test_bytes.bin";
        cleanup_file(dest);
        cleanup_dir("downloads");

        let http_download = http_download_fn();
        let result = http_download(&[
            Value::String(format!("{}/file.bin", BASE_URL)),
            Value::String(dest.to_string()),
        ]);
        assert!(
            matches!(result, Ok(Value::Bool(true))),
            "download should succeed, got: {:?}",
            result
        );

        // Read the downloaded file as raw bytes — NOT as a string.
        let downloaded = fs::read(dest).expect("downloaded file should be readable");
        let expected = expected_binary_content();

        assert_eq!(
            downloaded.len(),
            expected.len(),
            "downloaded size {} != expected {} — bytes were lost or mangled",
            downloaded.len(),
            expected.len()
        );

        // Byte-for-byte comparison — the whole point of this test.
        // Any accidental resp.text() in the implementation would
        // produce a UTF-8-decoded variant and fail here.
        for (i, (got, want)) in downloaded.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                got, want,
                "byte mismatch at offset {}: got 0x{:02X}, expected 0x{:02X}",
                i, got, want
            );
        }

        // Clean up
        cleanup_file(dest);
        cleanup_dir("downloads");
    }

    // ── Scenario 3: HTTP 404 → false, no file written ───────────────

    #[test]
    #[serial]
    fn test_http_download_404_returns_false() {
        let _server = ServerGuard::spawn();
        let dest = "downloads/p76_test_404.bin";
        cleanup_file(dest);
        cleanup_dir("downloads");

        let http_download = http_download_fn();
        let result = http_download(&[
            Value::String(format!("{}/notfound", BASE_URL)),
            Value::String(dest.to_string()),
        ]);

        assert!(
            matches!(result, Ok(Value::Bool(false))),
            "Expected Ok(Bool(false)) for 404, got: {:?}",
            result
        );

        // File must NOT exist (no partial write)
        let path = PathBuf::from(dest);
        assert!(
            !path.exists(),
            "no file should be written on 404, but found {:?}",
            path
        );

        // Clean up
        cleanup_file(dest);
        cleanup_dir("downloads");
    }

    // ── Scenario 4: Sandbox escape attempt → false ──────────────────

    #[test]
    #[serial]
    fn test_http_download_sandbox_escape_rejected() {
        let _server = ServerGuard::spawn();
        let outside_file = "p76_outside_sandbox.bin";
        cleanup_file(outside_file);
        let dest = "../p76_outside_sandbox.bin";

        let http_download = http_download_fn();
        let result = http_download(&[
            Value::String(format!("{}/file.bin", BASE_URL)),
            Value::String(dest.to_string()),
        ]);

        assert!(
            matches!(result, Ok(Value::Bool(false))),
            "Expected Ok(Bool(false)) for sandbox violation, got: {:?}",
            result
        );

        // File must NOT exist outside the working directory
        let outside_path = PathBuf::from(outside_file);
        assert!(
            !outside_path.exists(),
            "file should NOT be written outside sandbox, but found {:?}",
            outside_path
        );

        // Clean up (defensive — test should never have written this)
        cleanup_file(outside_file);
    }
}
