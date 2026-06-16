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
        
        let serverExe = 'fox';
        if (process.env.FOX_PATH) {
            serverExe = path.join(process.env.FOX_PATH, 'target', 'debug', 'fox');
        } else if (workspacePath) {
            const possiblePath = path.join(workspacePath, 'target', 'debug', 'fox');
            if (require('fs').existsSync(possiblePath)) {
                serverExe = possiblePath;
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
