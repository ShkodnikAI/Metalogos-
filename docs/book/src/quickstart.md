# Quick Start — the Full CLI Tour

## Quick Start

```bash
# Build from source
git clone https://github.com/ShkodnikAI/Metalogos-.git
cd Metalogos-
cargo install --path .

# Run a program
mlog run examples/m1_hello.mlog

# Compile to bytecode, then run via VM
mlog compile examples/m1_hello.mlog
mlog run examples/m1_hello.mbc

# Interactive REPL
mlog repl

# Semantic check (no execution)
mlog check examples/p6_full_app.mlog

# Static security audit
mlog audit examples/p6_full_app.mlog

# Serve as web application
mlog serve app.mlog

# Start as MCP server (expose .mlog tools to external MCP clients)
#   --transport stdio (default) | http | sse; --bind; bearer auth via --auth-token
mlog mcp-serve app.mlog --allowlist my_tool.send,my_tool.get
mlog mcp-serve app.mlog --allowlist my_tool.send --transport http --bind 127.0.0.1:8770 --auth-token $TOKEN

# Run eval harness (test learnable patterns)
mlog eval examples/m3_classify.mlog
```

Pre-built Linux x86_64 binaries are available from [GitHub Actions](https://github.com/ShkodnikAI/Metalogos-/actions) artifacts.

### LSP Server

```bash
# Build LSP server (requires workspace)
cargo install --path mlog-lsp

# Use with VS Code — extension included in editors/vscode/
# Or run standalone:
mlog-lsp
```

### Package Manager

```bash
cargo install --path mlogpkg
mlogpkg init my-project
mlogpkg add dependency@1.0
```

---
