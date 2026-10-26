#!/usr/bin/env python3
"""
Fixture MCP stdio server for НАРЯД №805 (wave 66 — the gateway G4 demo):
the FILE upstream. Two tools, the minimal public-server shape:

  tools/call "read_file"   {path}            -> the file content (String)
  tools/call "write_file"  {path, content}   -> "written"

newline-delimited JSON-RPC 2.0 over stdin/stdout (the same convention as
tests/fixtures/mcp_classified_server.py). The demo policy classifies
`files.read_file` as Source "private" — a file server CAN read .env, so
everything it returns must be treated as private by the session.
"""

import json
import sys


def reply(msg_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": msg_id, "result": result}) + "\n")
    sys.stdout.flush()


def main():
    tools = [
        {
            "name": "read_file",
            "description": "Reads a file from the local filesystem (Source — may return secrets)",
            "inputSchema": {
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
            },
        },
        {
            "name": "write_file",
            "description": "Writes content to a local file (Sink)",
            "inputSchema": {
                "type": "object",
                "properties": {"path": {"type": "string"}, "content": {"type": "string"}},
                "required": ["path", "content"],
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
                "serverInfo": {"name": "fixture-file-server", "version": "0.1.0"},
            })
        elif method == "notifications/initialized":
            continue
        elif method == "tools/list":
            reply(msg_id, {"tools": tools})
        elif method == "tools/call":
            name = req.get("params", {}).get("name", "")
            args = req.get("params", {}).get("arguments", {}) or {}
            if name == "read_file":
                path = args.get("path", "")
                try:
                    with open(path, "r", encoding="utf-8", errors="replace") as f:
                        content = f.read()
                except OSError as e:
                    reply(msg_id, {"isError": True, "content": [{"type": "text", "text": f"read failed: {e}"}]})
                    continue
                reply(msg_id, {"isError": False, "content": [{"type": "text", "text": content}]})
            elif name == "write_file":
                path = args.get("path", "")
                try:
                    with open(path, "w", encoding="utf-8") as f:
                        f.write(args.get("content", ""))
                    reply(msg_id, {"isError": False, "content": [{"type": "text", "text": "written"}]})
                except OSError as e:
                    reply(msg_id, {"isError": True, "content": [{"type": "text", "text": f"write failed: {e}"}]})
            else:
                sys.stdout.write(json.dumps({
                    "jsonrpc": "2.0", "id": msg_id,
                    "error": {"code": -32602, "message": f"Unknown tool: {name}"},
                }) + "\n")
                sys.stdout.flush()


if __name__ == "__main__":
    main()
