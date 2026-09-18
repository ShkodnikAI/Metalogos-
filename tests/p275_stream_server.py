#!/usr/bin/env python3
"""
Test SSE server for НАРЯД №275 — llm_stream_open/next/close contract testing.

Mock OpenAI-compatible `/v1/chat/completions` endpoint that streams SSE chunks.
The deltas spell "Hello, world!" split into 5 chunks, then `[DONE]`.

Scenarios (controlled via URL path):
  POST /v1/chat/completions        — 200, SSE stream with 5 delta chunks
  POST /v1/chat/non_stream         — 200, full JSON (no stream) — for equivalence test
  POST /v1/chat/error              — 500, JSON error (provider down)
  POST /v1/chat/no_stream_flag     — 200, full JSON when body has stream:false

Usage:
  python3 tests/p275_stream_server.py [--port PORT]

The delta chunks spell out "Hello, world!" — the Rust test compares the
concatenated deltas against this exact string.
"""

import sys
import json
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn

PORT = 18775

# The expected full text — chunks concatenate to this exactly.
EXPECTED_TEXT = "Hello, world!"

# Chunks: split "Hello, world!" into 5 pieces, like a real streamer would.
DELTAS = ["Hel", "lo,", " wor", "ld", "!"]


class StreamServerHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass  # suppress stderr noise

    def do_POST(self):
        # Read the request body.
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length).decode("utf-8") if length else ""

        # Parse JSON body to honor `stream: true/false`.
        try:
            parsed = json.loads(body) if body else {}
        except json.JSONDecodeError:
            parsed = {}
        stream = parsed.get("stream", True)

        if self.path == "/v1/chat/error":
            self._send_error_response(500, "provider simulated failure")
            return

        if self.path == "/v1/chat/non_stream":
            # Always return full JSON, regardless of stream flag.
            self._send_full_json(EXPECTED_TEXT)
            return

        if self.path == "/v1/chat/completions":
            if not stream:
                self._send_full_json(EXPECTED_TEXT)
                return
            self._send_sse_stream(DELTAS)
            return

        # Unknown path.
        self._send_error_response(404, "unknown path")

    def _send_error_response(self, status: int, message: str):
        body = json.dumps({"error": {"message": message}}).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _send_full_json(self, text: str):
        """200 OK with a complete OpenAI-compatible chat completion JSON."""
        body = json.dumps({
            "id": "chatcmpl-mock-001",
            "object": "chat.completion",
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": text},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": len(text.split()),
                "total_tokens": 5 + len(text.split())
            }
        }).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _send_sse_stream(self, deltas):
        """200 OK with `text/event-stream` — emits SSE chunks then [DONE]."""
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.end_headers()
        for i, delta in enumerate(deltas):
            chunk = {
                "id": "chatcmpl-mock-stream",
                "object": "chat.completion.chunk",
                "model": "mock-model",
                "choices": [{
                    "index": 0,
                    "delta": {"content": delta},
                    "finish_reason": None if i < len(deltas) - 1 else "stop"
                }]
            }
            line = f"data: {json.dumps(chunk)}\n\n"
            self.wfile.write(line.encode("utf-8"))
            self.wfile.flush()
            time.sleep(0.01)  # small delay so client can observe incremental
        # Final chunk with usage (when stream_options.include_usage: true —
        # we send it always for simplicity; the parser ignores it if it
        # doesn't match OpenAI's per-chunk usage shape).
        final_chunk = {
            "id": "chatcmpl-mock-stream",
            "object": "chat.completion.chunk",
            "model": "mock-model",
            "choices": [],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": len(deltas),
                "total_tokens": 5 + len(deltas)
            }
        }
        self.wfile.write(f"data: {json.dumps(final_chunk)}\n\n".encode("utf-8"))
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()


class ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
    """Multi-threaded — so multiple concurrent stream opens work."""
    daemon_threads = True


def main():
    global PORT
    if "--port" in sys.argv:
        idx = sys.argv.index("--port")
        PORT = int(sys.argv[idx + 1])
    server = ThreadingHTTPServer(("127.0.0.1", PORT), StreamServerHandler)
    print(f"llm_stream test server listening on http://127.0.0.1:{PORT}")
    print(f"  POST /v1/chat/completions   → 200, SSE stream (5 chunks: {DELTAS!r})")
    print(f"  POST /v1/chat/non_stream    → 200, full JSON (text: {EXPECTED_TEXT!r})")
    print(f"  POST /v1/chat/error         → 500, JSON error")
    sys.stdout.flush()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    server.server_close()


if __name__ == "__main__":
    main()
