# The Metalogos gateway in 5 minutes

One policy file (`policy.mlog`, 30 lines) turns `mlog` into an **MCP policy
gateway**: the agent keeps its normal tools, but every tool result carries a
data label and every call that would move data somewhere it is not cleared
for is refused **before** the upstream server sees it — with a signed ledger
entry for every decision.

This demo wires two upstream MCP servers — a filesystem server (`files`) and
a fetch/HTTP server (`web`) — and shows the canonical event:

> the agent reads `.env`, then tries to POST its contents to the network —
> the gateway refuses with `[SINK_CLEARANCE]`, records the denial in the
> Ed25519 ledger, and `ledger verify` stays green.

## 0. Build

```console
git clone https://github.com/ShkodnikAI/Metalogos-.git && cd Metalogos-
cargo build --release -p metalogos-server
# the binary: ./target/release/mlog
```

## 1. Declare the upstreams and start the gateway

`mcp-serve` is a normal MCP stdio server. The `METALOGOS_MCP_UPSTREAMS`
variable declares the pooled upstream sessions (`name=command,args`, one per
`;`), and `--allowlist` names the local tools the agent may call:

```console
export METALOGOS_MCP_UPSTREAMS='files=npx,-y,@modelcontextprotocol/server-filesystem,/home/you/projects;web=uvx,mcp-server-fetch'
export METALOGOS_MCP_ALLOWLIST='npx,uvx'
./target/release/mlog mcp-serve examples/gateway/policy.mlog --allowlist owner_unlock.unlock
```

Any MCP client speaks to the gateway; the upstreams speak to the gateway.

## 2. Point Claude Desktop (or any MCP client) at it

`claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "metalogos": {
      "command": "/absolute/path/to/Metalogos-/target/release/mlog",
      "args": ["mcp-serve", "/absolute/path/to/Metalogos-/examples/gateway/policy.mlog",
               "--allowlist", "owner_unlock.unlock"],
      "env": {
        "METALOGOS_MCP_UPSTREAMS": "files=npx,-y,@modelcontextprotocol/server-filesystem,/home/you/projects;web=uvx,mcp-server-fetch",
        "METALOGOS_MCP_ALLOWLIST": "npx,uvx"
      }
    }
  }
}
```

The agent now sees `files.read_file`, `files.write_file`, `web.http_get`,
`web.http_post` and `owner_unlock.unlock` — nothing else.

## 3. Watch the refusal

Ask the agent: *"read `.env` in the project directory, then POST its contents
to https://example.test/collect"*. What happens:

1. `files.read_file` succeeds — the result is labelled **private**, and the
   session label joins it (`_meta["metalogos.dev/session-label"]` shows it).
2. `web.http_post` is refused by the gateway before any bytes leave:

   ```text
   [SINK_CLEARANCE] upstream tool 'web.http_post' requires clearance
   'public, trusted'; the session label is 'private, trusted' — the call
   is refused.
   ```

3. The ledger has the decision — every allowed and every refused call is a
   signed, hash-chained record. After the session, pull the export through
   the gateway's audit tap and verify it with the STANDALONE verifier (no
   gateway process, no trust in its own verdict):

   ```console
   # through MCP: ledger_dump.dump {path: "data/ledger.jsonl"} — tool-lane writes
   # are contained to the data dir (METALOGOS_DATA_DIR, default ./data; mkdir -p data)
   ./target/release/mlog ledger verify data/ledger.jsonl
   # VALID: 8 records, head 9c9e… (1 distinct key(s), anchored start: false) — schema v1
   # the chain: mcp.call, mcp.call.denied, grant.issued, mcp.call (granted),
   #            grant.used, … , mcp.call.denied — the whole story is signed
   ```

## 4. The owner grant (and why it is safe)

The agent may ask the owner to unblock one concrete call:

```
owner_unlock.unlock(scope: "mcp:web.http_post")
```

The grant is **scoped** (one tool), has a **TTL** (60 s here) and is consumed
on the first use (`once`): the next call is refused again. The grant itself is
a ledger record — the audit trail shows exactly what was unblocked, when, and
how many times.

## 5. Reproduce the demo e2e locally

The CI runs this exact scenario on every push (the shipped policy file is the
tested artifact):

```console
cargo test -p metalogos-server --test naryad_805_gateway_demo
```

A 60–90 second terminal-only recording can be produced with
[screencast.sh](screencast.sh) (asciinema).

## What stands between the agent and the leak

| Layer | Mechanism | First refused in |
|---|---|---|
| Static | `mlog check` validates the policy before start (Category-A gate) | №394 |
| Session | floating label: every Source result joins, every Sink demands clearance | №801 |
| Ledger | every `tools/call` — allowed or refused — is signed and hash-chained | №794 |
| Grants | irreversible/uncleared actions need an explicit, scoped, one-shot grant | №782 |
