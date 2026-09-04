// ── METALOGOS VS Code extension — minimal LSP client ─────────────────
//
// Наряд №168: connect VS Code to the existing `mlog-lsp` binary
// (src/mlog-lsp/, stdio transport via tower-lsp). The LSP server
// already implements diagnostics, go-to-definition, hover. This
// file is just the VS Code-side glue: spawn the binary, wire up
// LanguageClient, register the document selector for `mlog`.
//
// The `mlog-lsp` binary must be on PATH or specified via the
// `mlog-lsp.server.path` setting. The extension does NOT bundle the
// binary — users install it separately via `cargo install --path
// mlog-lsp` (or `cargo build --release -p mlog-lsp` and add to PATH).
//
// This is intentionally minimal. Once the project stabilises, this
// file can grow to handle: bundling the binary, multi-platform
// prebuilds, telemetry, crash recovery. For now: get the LSP
// connected, let the server do the work.

import * as path from "path";
import * as vscode from "vscode";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from "vscode-languageclient/node";

/** Activate the extension: spawn mlog-lsp, register LanguageClient. */
export function activate(context: vscode.ExtensionContext): void {
    // Resolve the mlog-lsp binary path.
    // Priority: setting > bundled > PATH lookup.
    const config = vscode.workspace.getConfiguration("mlog-lsp.server");
    const configuredPath: string = config.get<string>("path") ?? "";

    const serverModule = configuredPath || "mlog-lsp";

    // If the path is a relative file path, resolve against workspace.
    // (Bundled binary scenario — not implemented yet, but the path
    // resolution logic is here so the contract is clear.)
    const serverCommand =
        path.isAbsolute(serverModule) || configuredPath === ""
            ? serverModule
            : path.join(
                  vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? "",
                  serverModule,
              );

    // Server options: spawn `mlog-lsp` as a child process communicating
    // over stdio. The mlog-lsp binary uses tower-lsp's
    // `Server::new(stdin, stdout, socket).serve(service)` — this matches
    // VS Code's `TransportKind.stdio`.
    const serverOptions: ServerOptions = {
        run: { command: serverCommand, transport: TransportKind.stdio },
        debug: { command: serverCommand, transport: TransportKind.stdio },
    };

    // Client options: which documents does the LSP server handle?
    // `mlog` language id is registered in package.json
    // (contributes.languages[].id = "mlog", extensions [".mlog"]).
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "mlog" }],
        synchronize: {
            // Notify the server about file changes to .mlog files in
            // the workspace — the LSP server can use this to refresh
            // diagnostics on external edits.
            fileEvents: vscode.workspace.createFileSystemWatcher("**/*.mlog"),
        },
        // Pass trace level through from the `mlog-lsp.trace.server` setting.
        // (VS Code's LanguageClient has built-in trace support; this just
        // enables the LSP logs panel.)
        outputChannelName: "METALOGOS LSP",
    };

    // Create the language client. We don't `start()` it inline — instead
    // we register a Disposable that starts the client on activation and
    // stops it on deactivation. This is the pattern recommended by
    // vscode-languageclient v9: `client.start()` returns `Promise<void>`
    // (not a Disposable as in older versions), so we wrap it.
    const client = new LanguageClient(
        "mlog-lsp",
        "METALOGOS Language Server",
        serverOptions,
        clientOptions,
    );

    // Register the client lifecycle as a Disposable. On dispose, the
    // client is stopped (sends `shutdown` + `exit` to the LSP server,
    // then kills the child process).
    context.subscriptions.push({
        dispose: () => {
            // `client.stop()` is async but VS Code disposes synchronously.
            // We fire-and-forget the stop — the LanguageClient handles
            // graceful shutdown internally and will log if the process
            // is already gone (which is fine during deactivation).
            void client.stop();
        },
    });

    // Start the client. This spawns `mlog-lsp` as a child process and
    // performs the LSP `initialize` handshake. If the binary is not
    // found, VS Code will show an error notification — that's the
    // intended user-facing failure mode (no silent swallowing).
    //
    // We don't await here: activation must return promptly. The LSP
    // handshake runs in the background and VS Code queues any requests
    // until the client is ready.
    void client.start();

    // Log activation — helps users debug "why doesn't my LSP work?"
    console.log("[mlog-lsp] Extension activated. Server command:", serverCommand);
}

/** Deactivate: the LanguageClient handles its own shutdown via
 *  context.subscriptions disposal; nothing extra to do here.
 */
export function deactivate(): Thenable<void> | undefined {
    // LanguageClient is registered in context.subscriptions, VS Code
    // calls its dispose() automatically. No explicit teardown needed.
    return undefined;
}
