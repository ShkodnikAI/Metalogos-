#!/usr/bin/env python3
"""
Fixture MCP stdio server for НАРЯД №268 — contract tests for mcp_call /
mcp_list_tools (ADR-0132). Minimal legacy-dialect MCP server:
newline-delimited JSON-RPC 2.0 over stdin/stdout, mirroring the
p71/p76 python-fixture convention.

Protocol surface spoken (legacy dialect, ADR-0132 D1):
  initialize                     -> result(protocolVersion, capabilities, serverInfo)
  notifications/initialized      -> notification, no response (per spec)
  tools/list                     -> result.tools = [echo, fail, boom, sleep]
  tools/call "echo"              -> result.content = [{type:"text", text:"echo: ..."}]
  tools/call "fail"              -> result.isError = true + text detail
  tools/call "boom"              -> JSON-RPC error -32603 (internal error)
  tools/call <anything else>     -> JSON-RPC error -32602 (Unknown tool)
  tools/call "sleep"             -> time.sleep(arguments.seconds), then answers

Env knobs (set by the Rust test, read here):
  MCP_FIXTURE_GARBAGE=1   print a NON-JSON garbage line to stdout before the
                          next response — client must fail LOUDLY with
                          [MCP_PROTOCOL_ERROR] (framing robustness)
  MCP_FIXTURE_CRASH=1     exit(1) immediately after receiving `initialize`,
                          BEFORE responding — client must see a broken stream
                          and report [MCP_IO_ERROR] phase=handshake

Spec notes honored: stdout carries ONLY valid MCP messages (unless the test
explicitly requests garbage); stderr is unused; shutdown = stdin close.
"""

import json
import os
import sys
import time

TOOLS = [
    {
        "name": "echo",
        "description": "Returns its input text back, prefixed with 'echo:'",
        "inputSchema": {
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"],
        },
    },
    {
        "name": "fail",
        "description": "Always answers isError=true with a detail text",
        "inputSchema": {"type": "object"},
    },
    {
        "name": "boom",
        "description": "Always answers with a JSON-RPC internal error (-32603)",
        "inputSchema": {"type": "object"},
    },
    {
        "name": "sleep",
        "description": "Sleeps `seconds` (default 10) then answers 'woke up'",
        "inputSchema": {
            "type": "object",
            "properties": {"seconds": {"type": "number"}},
        },
    },
]


def send(msg):
    """One JSON message per line; flush is essential for pipe interop."""
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def respond(msg_id, result):
    send({"jsonrpc": "2.0", "id": msg_id, "result": result})


def respond_error(msg_id, code, message):
    send({"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}})


def maybe_garbage():
    """Emit one non-JSON line BEFORE the next response when asked."""
    if os.environ.get("MCP_FIXTURE_GARBAGE") == "1":
        sys.stdout.write("::garbage:: this line is not JSON\n")
        sys.stdout.flush()


def main():
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except ValueError:
            # A spec-compliant server never sees invalid JSON on stdin;
            # the fixture stays silent rather than inventing behavior.
            continue
        method = msg.get("method")
        msg_id = msg.get("id")

        if method == "initialize":
            if os.environ.get("MCP_FIXTURE_CRASH") == "1":
                # Die BEFORE responding: the client must report a broken
                # stream, not a protocol error.
                sys.exit(1)
            params = msg.get("params") or {}
            maybe_garbage()
            respond(
                msg_id,
                {
                    "protocolVersion": params.get("protocolVersion", "2025-03-26"),
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "mcp-echo-fixture", "version": "0.1.0"},
                },
            )
        elif method == "notifications/initialized":
            pass  # notification: never answered (spec)
        elif method == "tools/list":
            maybe_garbage()
            respond(msg_id, {"tools": TOOLS})
        elif method == "tools/call":
            params = msg.get("params") or {}
            name = params.get("name", "")
            args = params.get("arguments") or {}
            maybe_garbage()
            if name == "echo":
                text = args.get("text", "")
                respond(
                    msg_id,
                    {
                        "content": [{"type": "text", "text": "echo: " + str(text)}],
                        "isError": False,
                    },
                )
            elif name == "fail":
                respond(
                    msg_id,
                    {
                        "content": [
                            {
                                "type": "text",
                                "text": "fixture failure detail: code=FIXTURE_FAIL",
                            }
                        ],
                        "isError": True,
                    },
                )
            elif name == "boom":
                respond_error(msg_id, -32603, "fixture internal error")
            elif name == "sleep":
                time.sleep(float(args.get("seconds", 10)))
                respond(msg_id, {"content": [{"type": "text", "text": "woke up"}]})
            else:
                respond_error(msg_id, -32602, "Unknown tool: " + str(name))
        else:
            respond_error(msg_id, -32601, "Method not found: " + str(method))


if __name__ == "__main__":
    main()
