import * as path from "path";
import {
  ExtensionContext,
  workspace,
  window,
  commands,
} from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration("kestrel");
  const command = config.get<string>("lsp.path") ?? "kestrel-lsp";

  const serverOptions: ServerOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "kestrel" }],
    synchronize: {
      fileEvents: [
        workspace.createFileSystemWatcher("**/flock.toml"),
        workspace.createFileSystemWatcher("**/flock.lock"),
        workspace.createFileSystemWatcher("**/*.ks"),
      ],
    },
    outputChannel: window.createOutputChannel("Kestrel Language Server"),
  };

  client = new LanguageClient(
    "kestrel",
    "Kestrel Language Server",
    serverOptions,
    clientOptions,
  );

  context.subscriptions.push(
    commands.registerCommand("kestrel.restartServer", async () => {
      if (client) {
        await client.stop();
        await client.start();
      }
    }),
  );

  try {
    await client.start();
  } catch (err) {
    window.showErrorMessage(
      `Failed to start kestrel-lsp ('${command}'). ` +
        `Set 'kestrel.lsp.path' to a built binary or run 'cargo build -p kestrel-lsp'. ` +
        `Error: ${err}`,
    );
  }
}

export async function deactivate() {
  if (client) {
    await client.stop();
  }
}
