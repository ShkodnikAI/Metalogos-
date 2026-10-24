#!/usr/bin/env python3
"""
Fixture MCP stdio server for НАРЯД №793 — the STATEFUL upstream (the
gateway G1): proves the session pool holds ONE process across calls.

newline-delimited JSON-RPC 2.0 over stdin/stdout (the same convention
as tests/fixtures/mcp_echo_server.py).

Protocol surface:
  initialize                -> result(protocolVersion, capabilities, serverInfo)
  notifications/initialized -> notification, no response
  tools/list                -> result.tools = [count, crash, ping]
  tools/call "count"        -> returns the in-memory counter value, THEN
                               increments it — three calls in one session
                               answer 0, 1, 2 (the state lives in the
                               process; a fresh process restarts at 0)
  tools/call "crash"        -> answers "bye" and exits(1) — the client
                               must see the death and reconnect LOUDLY
  tools/call "ping"         -> returns the OS pid (the one-process proof)
  tools/call <other>        -> JSON-RPC error -32602 (Unknown tool)

Env knobs (set by the Rust test):
  MCP_FIXTURE_VERSION=X    respond to initialize with protocolVersion X
                           REGARDLESS of the requested one (the version
                           mismatch test; the echo fixture's echo-form is
                           the negotiated-success case)
"""

import json
import os
import sys

TOOLS = [
    {
        "name": "count",
        "description": "Returns the in-memory counter, then increments it",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "crash",
        "description": "Answers 'bye' and exits(1) — the reconnect test",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "ping",
        "description": "Returns the OS pid as a string",
        "inputSchema": {"type": "object", "properties": {}},
    },
]

_counter = 0


def send(msg):
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def respond(msg_id, result):
    send({"jsonrpc": "2.0", "id": msg_id, "result": result})


def respond_error(msg_id, code, message):
    send({"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}})


def main():
    global _counter
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
            forced = os.environ.get("MCP_FIXTURE_VERSION")
            version = forced or params.get("protocolVersion", "2025-11-25")
            respond(
                msg_id,
                {
                    "protocolVersion": version,
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "mcp-stateful-fixture", "version": "0.1.0"},
                },
            )
        elif method == "notifications/initialized":
            pass
        elif method == "tools/list":
            respond(msg_id, {"tools": TOOLS})
        elif method == "tools/call":
            params = msg.get("params") or {}
            name = params.get("name", "")
            if name == "count":
                respond(
                    msg_id,
                    {
                        "content": [{"type": "text", "text": str(_counter)}],
                        "isError": False,
                    },
                )
                _counter += 1
            elif name == "crash":
                respond(
                    msg_id,
                    {"content": [{"type": "text", "text": "bye"}], "isError": False},
                )
                sys.stdout.flush()
                sys.exit(1)
            elif name == "ping":
                respond(
                    msg_id,
                    {
                        "content": [{"type": "text", "text": str(os.getpid())}],
                        "isError": False,
                    },
                )
            else:
                respond_error(msg_id, -32602, "Unknown tool: " + str(name))
        else:
            respond_error(msg_id, -32601, "Method not found: " + str(method))


if __name__ == "__main__":
    main()
