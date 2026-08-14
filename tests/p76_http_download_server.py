#!/usr/bin/env python3
"""
Test HTTP server for НАРЯД №76 — http_download binary contract testing.

Scenarios (controlled via URL path):
  GET /file.bin        — 200 OK, returns pre-baked non-UTF-8 binary content
  GET /notfound        — 404 Not Found (tests network/HTTP error path)

The binary content is intentionally NOT valid UTF-8 — it contains
0xFF, 0xFE, 0xFD, 0x00, 0x80-0xFF byte sequences. This is what the
contract test relies on: if `http_download` ever accidentally calls
`resp.text()` instead of `resp.bytes()`, the bytes will be mangled
by UTF-8 decoding and the byte-for-byte comparison will fail.

The content is generated deterministically (no RNG) so the same file
is served on every request — the test compares the downloaded file
against the same generation formula in Rust.

Usage:
  python3 tests/p76_http_download_server.py [--port PORT]
"""

import sys
from http.server import HTTPServer, BaseHTTPRequestHandler

PORT = 18776


def make_binary_content() -> bytes:
    """Deterministic non-UTF-8 binary content.

    Properties:
      - Contains bytes 0x00 through 0xFF (all 256 byte values, in order)
      - Therefore NOT valid UTF-8 (0xFF/0xFE/0xFD etc. are illegal
        UTF-8 leading bytes)
      - Deterministic — same bytes on every call, so Rust test can
        reproduce the expected content with the same formula
    """
    return bytes(range(256))


class DownloadTestHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass  # suppress stderr

    def do_GET(self):
        if self.path == "/file.bin":
            content = make_binary_content()
            self.send_response(200)
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            self.wfile.write(content)
        elif self.path == "/notfound":
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()
        else:
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()


def main():
    global PORT
    if "--port" in sys.argv:
        idx = sys.argv.index("--port")
        PORT = int(sys.argv[idx + 1])

    server = HTTPServer(("127.0.0.1", PORT), DownloadTestHandler)
    print(f"http_download test server listening on http://127.0.0.1:{PORT}")
    print(f"  GET /file.bin    → 200, 256 bytes (0x00..0xFF, non-UTF-8)")
    print(f"  GET /notfound    → 404")
    print("Press Ctrl+C to stop")
    sys.stdout.flush()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    server.server_close()


if __name__ == "__main__":
    main()
