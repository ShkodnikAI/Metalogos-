#!/usr/bin/env python3
"""Helper server for p51 concurrency test. Listens on port 10098, responds after 5s delay."""
import http.server
import time
import sys

class SleepHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        time.sleep(5)
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(b'{"status":"slept"}')
    
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain')
        self.end_headers()
        self.wfile.write(b'ok')
    
    def log_message(self, format, *args):
        pass  # suppress logs

if __name__ == '__main__':
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 10098
    server = http.server.HTTPServer(('127.0.0.1', port), SleepHandler)
    print(f'Helper server on port {port}', flush=True)
    server.serve_forever()
