const path = require('path');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

let client;

function activate(context) {
    // The extension is in .vscode-extension/, so the project root is one level up
    const projectRoot = path.resolve(__dirname, '..');

    // Use debug binary by default, release if it exists
    let serverPath = path.join(projectRoot, 'target', 'debug', 'kestrel-lsp');
    const releasePath = path.join(projectRoot, 'target', 'release', 'kestrel-lsp');

    const fs = require('fs');
    if (fs.existsSync(releasePath)) {
        serverPath = releasePath;
    }

    const serverOptions = {
        command: serverPath,
        args: [],
        options: {
            cwd: projectRoot
        }
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'kestrel' }],
        synchronize: {
            fileEvents: null
        }
    };

    client = new LanguageClient(
        'kestrel-lsp',
        'Kestrel Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
    console.log('Kestrel LSP client started');
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
