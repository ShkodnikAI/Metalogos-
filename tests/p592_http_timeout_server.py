#!/usr/bin/env python3
"""Test HTTP server for НАРЯД №592 — the per-call timeout outcome taxonomy.

Modes (via path):
  /slow?sec=N      — sleeps N seconds, then responds 200 "late" (the
                     timeout DoD row: the deadline fires within ±10%)
  /break           — sends response headers declaring Content-Length: 100
                     but writes only 10 body bytes, then closes the socket
                     (the mid-response break → the typed [HTTP_CONNECT])
  /status?code=N   — responds with the given status code (the typed
                     [HTTP_STATUS] outcome)
  /ok              — 200 "fine" (the success path)

Usage: python3 tests/p592_http_timeout_server.py --port PORT
"""

import argparse
import sys
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        pass  # silence the stderr noise

    def do_GET(self):
        u = urlparse(self.path)
        mode = u.path
        q = parse_qs(u.query)

        if mode == "/slow":
            sec = float(q.get("sec", ["2"])[0])
            time.sleep(sec)
            body = b"late"
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif mode == "/break":
            # declare 100 bytes, deliver 10, then slam the socket shut —
            # the client's .text() read must surface a typed failure,
            # not a silent empty body
            self.send_response(200)
            self.send_header("Content-Length", "100")
            self.end_headers()
            self.wfile.write(b"PARTIAL-10")
            self.wfile.flush()
            self.close_connection = True
            # force-close mid-body: shutdown the underlying socket
            try:
                self.connection.shutdown(__import__("socket").SHUT_RDWR)
            except OSError:
                pass
        elif mode == "/status":
            code = int(q.get("code", ["404"])[0])
            body = b"nope"
            self.send_response(code)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif mode == "/ok":
            body = b"fine"
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0") or 0)
        _ = self.rfile.read(length)
        self.do_GET()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=18792)
    args = ap.parse_args()
    # ThreadingHTTPServer: the break mode must not block other connections
    from http.server import ThreadingHTTPServer

    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    print(f"listening on {args.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        sys.exit(0)


if __name__ == "__main__":
    main()
