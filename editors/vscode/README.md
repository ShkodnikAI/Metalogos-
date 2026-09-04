# METALOGOS Language Support for VS Code

LSP-based language support for METALOGOS (`.mlog`) files.

## Features

- **Diagnostics** — real-time semantic analysis (errors, warnings) via `mlog-lsp`
- **Go-to-definition** — jump to entity / pattern / flow declarations
- **Hover** — type info and confidence metadata on hover
- **Syntax highlighting** — TextMate grammar for `.mlog` files

The LSP server (`mlog-lsp`) is a separate Rust binary — this extension
is the VS Code client that spawns it as a child process and wires up
the language client.

## Prerequisites

1. **Rust toolchain** (to build the LSP server)
2. **`mlog-lsp` binary on PATH** — build it from the repo root:

   ```bash
   cargo build --release -p mlog-lsp
   # The binary is at target/release/mlog-lsp
   # Add it to PATH or set the mlog-lsp.server.path setting (see below)
   ```

   Or install to `~/.cargo/bin`:

   ```bash
   cargo install --path mlog-lsp
   ```

## Build the extension

This is only needed if you're developing the extension itself. End
users install via `.vsix` (published) or VS Code Marketplace.

```bash
cd editors/vscode
npm ci          # install dev dependencies (typescript, vscode-languageclient)
npm run compile # compile src/extension.ts → out/extension.js
```

The `package.json` field `"main": "./out/extension.js"` must point to
a real file. CI checks this via the `vscode-extension (blocking)` job
in `.github/workflows/ci.yml`.

## Configuration

| Setting | Default | Description |
|---|---|---|
| `mlog-lsp.server.path` | `""` | Path to the `mlog-lsp` binary. If empty, the extension looks for `mlog-lsp` on PATH. |
| `mlog-lsp.trace.server` | `"off"` | Trace level for LSP communication (`off` / `messages` / `verbose`). Opens an output channel for debugging. |

## Development notes

### Why the extension doesn't bundle the binary

The `mlog-lsp` binary is platform-specific (Linux/macOS/Windows) and
built from Rust. Bundling it would require prebuilt binaries for every
platform — that's a future enhancement (see ADR-0110 §2: contract
before code; for now, the contract is "user has `mlog-lsp` on PATH").

### CI verification

The `vscode-extension (blocking)` CI job runs:

1. `npm ci` — install dependencies from `package-lock.json`
2. `npm run compile` — TypeScript → JavaScript
3. Verify `out/extension.js` exists (the `main` field target)

If you add a new dependency, run `npm install` locally to update
`package-lock.json`, then commit both `package.json` and
`package-lock.json`. CI uses `npm ci` which requires the lockfile to be
in sync.

### File layout

```
editors/vscode/
├── package.json                    # Extension manifest (contributes, main, deps)
├── package-lock.json               # Locked dependency versions for reproducible CI
├── tsconfig.json                   # TypeScript config (target ES2022, strict)
├── .gitignore                      # Ignores node_modules/ and out/
├── src/
│   └── extension.ts                # Minimal LSP client (~110 lines)
├── syntaxes/
│   └── mlog.tmLanguage.json        # TextMate grammar for syntax highlighting
└── language-configuration.json    # Brackets, autoclosing pairs, comments
```
