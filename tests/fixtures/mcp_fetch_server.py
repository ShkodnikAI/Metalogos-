#!/usr/bin/env python3
"""
Fixture MCP stdio server for НАРЯД №805 (wave 66 — the gateway G4 demo):
the FETCH/HTTP upstream. Two tools, the minimal public-server shape:

  tools/call "http_post" {url, body} -> "posted <len> bytes to <url>"
  tools/call "http_get"  {url}       -> "fetched <url>"

newline-delimited JSON-RPC 2.0 over stdin/stdout (the same convention as
tests/fixtures/mcp_classified_server.py). The demo policy classifies
`web.http_post` as Sink "public, trusted" — this is the tool the leaking
agent tries to use, and the one the SINK_CLEARANCE gate must refuse.
"""

import json
import sys


def reply(msg_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": msg_id, "result": result}) + "\n")
    sys.stdout.flush()


def main():
    tools = [
        {
            "name": "http_post",
            "description": "POSTs a body to a URL (Sink — data leaves the machine)",
            "inputSchema": {
                "type": "object",
                "properties": {"url": {"type": "string"}, "body": {"type": "string"}},
                "required": ["url", "body"],
            },
        },
        {
            "name": "http_get",
            "description": "GETs a URL (Source — untrusted web content)",
            "inputSchema": {
                "type": "object",
                "properties": {"url": {"type": "string"}},
                "required": ["url"],
            },
        },
    ]
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue
        method = req.get("method", "")
        msg_id = req.get("id")
        if method == "initialize":
            reply(msg_id, {
                "protocolVersion": "2025-11-25",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "fixture-fetch-server", "version": "0.1.0"},
            })
        elif method == "notifications/initialized":
            continue
        elif method == "tools/list":
            reply(msg_id, {"tools": tools})
        elif method == "tools/call":
            name = req.get("params", {}).get("name", "")
            args = req.get("params", {}).get("arguments", {}) or {}
            if name == "http_post":
                url = args.get("url", "")
                body = args.get("body", "")
                reply(msg_id, {"isError": False, "content": [
                    {"type": "text", "text": f"posted {len(body)} bytes to {url}"}]})
            elif name == "http_get":
                url = args.get("url", "")
                reply(msg_id, {"isError": False, "content": [
                    {"type": "text", "text": f"fetched {url}"}]})
            else:
                sys.stdout.write(json.dumps({
                    "jsonrpc": "2.0", "id": msg_id,
                    "error": {"code": -32602, "message": f"Unknown tool: {name}"},
                }) + "\n")
                sys.stdout.flush()


if __name__ == "__main__":
    main()
