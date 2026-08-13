#!/usr/bin/env python3
"""
Test HTTP server for НАРЯД №71 — retry/backoff contract testing.

Scenarios (controlled via query parameter "scenario"):
  1. ?scenario=retry_503  — responds 503 twice, then 200 (tests retry + backoff)
  2. ?scenario=fatal_400  — responds 400 immediately (tests no-retry on fatal error)
  3. ?scenario=ok_200     — responds 200 immediately (tests fast path, no retries)

State is tracked per-scenario in-memory. Use ?reset=1 to reset counters.

Usage:
  python3 tests/p71_http_retry_server.py [--port PORT] [--reset]
  # Then run: mlog examples/p71_http_retry.mlog
"""

import json
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

PORT = 18771
counters = {}  # scenario -> request_count


class RetryTestHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Suppress default stderr logging
        pass

    def do_GET(self):
        parsed = urlparse(self.path)
        params = parse_qs(parsed.query)
        scenario = params.get("scenario", ["ok_200"])[0]

        if "reset" in params:
            counters.clear()
            self._respond(200, "reset ok")
            return

        count = counters.get(scenario, 0)
        counters[scenario] = count + 1

        if scenario == "retry_503":
            if count < 2:
                self._respond(503, f"Service Unavailable (attempt {count + 1})")
            else:
                self._respond(200, f"ok after {count} retries")
        elif scenario == "fatal_400":
            self._respond(400, "Bad Request - no retry expected")
        elif scenario == "ok_200":
            self._respond(200, "immediate ok")
        else:
            self._respond(404, f"unknown scenario: {scenario}")

    def do_POST(self):
        # Same logic as GET for testing purposes
        parsed = urlparse(self.path)
        params = parse_qs(parsed.query)
        scenario = params.get("scenario", ["ok_200"])[0]

        if "reset" in params:
            counters.clear()
            self._respond(200, "reset ok")
            return

        count = counters.get(scenario, 0)
        counters[scenario] = count + 1

        # Read body
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8") if content_length > 0 else ""

        if scenario == "retry_503":
            if count < 2:
                self._respond(503, f"Service Unavailable (attempt {count + 1})")
            else:
                self._respond(200, f"ok after {count} retries")
        elif scenario == "fatal_400":
            self._respond(400, "Bad Request - no retry expected")
        elif scenario == "ok_200":
            self._respond(200, "immediate ok")
        else:
            self._respond(404, f"unknown scenario: {scenario}")

    def _respond(self, status, body):
        self.send_response(status)
        self.send_header("Content-Type", "text/plain")
        self.end_headers()
        self.wfile.write(body.encode("utf-8"))


def main():
    global PORT
    if "--port" in sys.argv:
        idx = sys.argv.index("--port")
        PORT = int(sys.argv[idx + 1])

    if "--reset" in sys.argv:
        # Just send reset request
        import urllib.request
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{PORT}/?reset=1")
        except:
            pass
        return

    server = HTTPServer(("127.0.0.1", PORT), RetryTestHandler)
    print(f"Retry test server listening on http://127.0.0.1:{PORT}")
    print(f"  GET /?scenario=retry_503  → 503, 503, 200")
    print(f"  GET /?scenario=fatal_400  → 400")
    print(f"  GET /?scenario=ok_200     → 200")
    print(f"  GET /?reset=1             → reset counters")
    print("Press Ctrl+C to stop")
    sys.stdout.flush()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    server.server_close()


if __name__ == "__main__":
    main()
