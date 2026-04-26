import * as fs from "fs";
import * as path from "path";
import {
  ExtensionContext,
  Uri,
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
  // Resolve order:
  //   1. `kestrel.lsp.path` setting (explicit override; absolute or PATH name)
  //   2. Bundled `server/<platform>-<arch>/kestrel-lsp` shipped with the .vsix
  //   3. `kestrel-lsp` on PATH (for users with a manually-built binary)
  const settingPath = (config.get<string>("lsp.path") ?? "").trim();
  const bundled = resolveBundledLsp(context);
  const command = settingPath || bundled || "kestrel-lsp";

  const serverOptions: ServerOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };

  const stdlibPath = config.get<string>("stdlibPath") ?? "";
  const flockCachePath = config.get<string>("flockCachePath") ?? "";

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "kestrel" }],
    synchronize: {
      fileEvents: [
        workspace.createFileSystemWatcher("**/flock.toml"),
        workspace.createFileSystemWatcher("**/flock.lock"),
        workspace.createFileSystemWatcher("**/*.ks"),
      ],
    },
    initializationOptions: {
      stdlibPath: stdlibPath || null,
      flockCachePath: flockCachePath || null,
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

  // Triggered by the "▶ Run" CodeLens above `func main()`. Opens (or
  // reuses) a terminal in the workspace folder owning the source file
  // and runs `flock run` there. The flock binary itself is resolved
  // against `kestrel.flockPath`, then PATH, then a workspace-local
  // `./flock`.
  context.subscriptions.push(
    commands.registerCommand("kestrel.runMain", async (uriString: string) => {
      let cwd: string | undefined;
      try {
        const fileUri = Uri.parse(uriString);
        const folder = workspace.getWorkspaceFolder(fileUri);
        cwd = folder
          ? folder.uri.fsPath
          : path.dirname(fileUri.fsPath);
      } catch {
        cwd = undefined;
      }

      const flock = resolveFlockBinary(cwd);
      if (!flock) {
        window.showErrorMessage(
          "Couldn't find a flock binary. Set 'kestrel.flockPath' or put `flock` on PATH.",
        );
        return;
      }
      const terminal =
        window.terminals.find((t) => t.name === "Kestrel Run") ??
        window.createTerminal({ name: "Kestrel Run", cwd });
      terminal.show(true);
      // Quote the path so spaces in it don't break the shell. `flock` is
      // the same project's own package manager — invoking it as `./flock`
      // when found in the workspace root keeps the relative form readable.
      const cmd = flock.includes(" ") ? `"${flock}"` : flock;
      terminal.sendText(`${cmd} run`, true);
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

/**
 * Locate the kestrel-lsp binary shipped with this extension. We bundle a
 * per-platform binary under `server/<platform>-<arch>/kestrel-lsp` so the
 * extension works out of the box without requiring users to build the LSP
 * themselves or have it on PATH.
 *
 * Returns `undefined` if no matching bundled binary exists — e.g. when
 * running an unbundled .vsix on an unsupported platform, or in the
 * Extension Development Host before `npm run server:build` has copied the
 * dev binary in.
 */
function resolveBundledLsp(context: ExtensionContext): string | undefined {
  const exeName = process.platform === "win32" ? "kestrel-lsp.exe" : "kestrel-lsp";
  const target = `${process.platform}-${process.arch}`;
  const candidate = path.join(context.extensionPath, "server", target, exeName);
  if (isExecutableFile(candidate)) {
    return candidate;
  }
  return undefined;
}

/**
 * Pick the flock binary to invoke. Order:
 *   1. `kestrel.flockPath` setting (absolute or relative to workspace cwd)
 *   2. `flock` on PATH — return as bare "flock" so the shell resolves it
 *   3. `./flock` next to the workspace root (the lang/flock package
 *      ships with a pre-built binary)
 */
function resolveFlockBinary(cwd: string | undefined): string | undefined {
  const settingPath = workspace
    .getConfiguration("kestrel")
    .get<string>("flockPath", "")
    .trim();
  if (settingPath) {
    return settingPath;
  }
  if (commandExists("flock")) {
    return "flock";
  }
  if (cwd) {
    const candidate = path.join(cwd, "flock");
    if (isExecutableFile(candidate)) {
      return candidate;
    }
  }
  return undefined;
}

function commandExists(cmd: string): boolean {
  const pathEnv = process.env.PATH ?? "";
  for (const dir of pathEnv.split(path.delimiter)) {
    if (!dir) continue;
    if (isExecutableFile(path.join(dir, cmd))) {
      return true;
    }
  }
  return false;
}

function isExecutableFile(p: string): boolean {
  try {
    const st = fs.statSync(p);
    if (!st.isFile()) return false;
    fs.accessSync(p, fs.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}
