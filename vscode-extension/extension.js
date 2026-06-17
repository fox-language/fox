const path = require('path');
const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;
const logChannel = vscode.window.createOutputChannel("Fox Extension Client");

function activate(context) {
    logChannel.appendLine("Fox Extension activating...");
    try {
        const workspacePath = vscode.workspace.workspaceFolders
            ? vscode.workspace.workspaceFolders[0].uri.fsPath
            : '';

        // Prefer a release build; fall back to debug. The extension ships
        // without a bundled binary, so this also lets a local `cargo build`
        // or `cargo build --release` automatically be picked up.
        let serverExe = 'fox';
        if (process.env.FOX_PATH) {
            const releasePath = path.join(process.env.FOX_PATH, 'target', 'release', 'fox');
            serverExe = releasePath;
        } else if (workspacePath) {
            const releasePath = path.join(workspacePath, 'target', 'release', 'fox');
            if (require('fs').existsSync(releasePath)) {
                serverExe = releasePath;
            }
        }

        // If the release binary isn't there, try debug.
        if (!require('fs').existsSync(serverExe)) {
            const debugPath = process.env.FOX_PATH
                ? path.join(process.env.FOX_PATH, 'target', 'debug', 'fox')
                : path.join(workspacePath, 'target', 'debug', 'fox');
            if (require('fs').existsSync(debugPath)) {
                serverExe = debugPath;
            }
        }

        logChannel.appendLine(`Resolved server executable to: ${serverExe}`);

        const serverOptions = {
            run: { command: serverExe, args: ['lsp'] },
            debug: { command: serverExe, args: ['lsp'] }
        };

        const clientOptions = {
            documentSelector: [{ scheme: 'file', language: 'fox' }],
            synchronize: {
                fileEvents: vscode.workspace.createFileSystemWatcher('**/*.fox')
            }
        };

        logChannel.appendLine("Initializing LanguageClient...");
        client = new LanguageClient(
            'foxLsp',
            'Fox Language Server',
            serverOptions,
            clientOptions
        );

        logChannel.appendLine("Starting LanguageClient...");
        client.start().catch(err => {
            logChannel.appendLine("LanguageClient failed to start: " + err.message);
        });
        logChannel.appendLine("LanguageClient started successfully!");
    } catch (err) {
        logChannel.appendLine("Failed to activate extension: " + err.stack);
    }
}

function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

module.exports = {
    activate,
    deactivate
};
