#!/usr/bin/env python3
"""
Fixture MCP stdio server for НАРЯД №801 — the CLASSIFIED upstream (the
gateway G2): four tools with distinct security classes, so the floating
session-label policy (Source/Sink) has something to bite on.

newline-delimited JSON-RPC 2.0 over stdin/stdout (the same convention as
tests/fixtures/mcp_stateful_server.py).

Protocol surface:
  initialize                -> result(protocolVersion, capabilities, serverInfo)
  notifications/initialized -> notification, no response
  tools/list                -> result.tools = [read_secret, fetch_page,
                               send_email, run_cmd]
  tools/call "read_secret"  -> "SECRET-42"      (the policy: Source "private")
  tools/call "fetch_page"   -> "WEB-CONTENT"    (the policy: Source "public, untrusted")
  tools/call "send_email"   -> "sent"           (the policy: Sink "public")
  tools/call "run_cmd"      -> "ran"            (the policy: Sink "public, trusted")
  tools/call <other>        -> JSON-RPC error -32602 (Unknown tool)
"""

import json
import sys

TOOLS = [
    {
        "name": "read_secret",
        "description": "Returns the fixture secret (Source, private)",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "fetch_page",
        "description": "Returns fixture web content (Source, untrusted)",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "send_email",
        "description": "Sends the fixture email (Sink, public)",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "run_cmd",
        "description": "Runs the fixture command (Sink, public+trusted)",
        "inputSchema": {"type": "object", "properties": {}},
    },
]

ANSWERS = {
    "read_secret": "SECRET-42",
    "fetch_page": "WEB-CONTENT",
    "send_email": "sent",
    "run_cmd": "ran",
}


def send(msg):
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def respond(msg_id, result):
    send({"jsonrpc": "2.0", "id": msg_id, "result": result})


def respond_error(msg_id, code, message):
    send({"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}})


def main():
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        method = msg.get("method")
        msg_id = msg.get("id")

        if method == "initialize":
            params = msg.get("params") or {}
            respond(
                msg_id,
                {
                    "protocolVersion": params.get("protocolVersion", "2025-11-25"),
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "mcp-classified-fixture", "version": "0.1.0"},
                },
            )
        elif method == "notifications/initialized":
            pass
        elif method == "tools/list":
            respond(msg_id, {"tools": TOOLS})
        elif method == "tools/call":
            params = msg.get("params") or {}
            name = params.get("name", "")
            if name in ANSWERS:
                respond(
                    msg_id,
                    {
                        "content": [{"type": "text", "text": ANSWERS[name]}],
                        "isError": False,
                    },
                )
            else:
                respond_error(msg_id, -32602, "Unknown tool: " + str(name))
        else:
            respond_error(msg_id, -32601, "Method not found: " + str(method))


if __name__ == "__main__":
    main()
