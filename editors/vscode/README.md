# Kestrel for VSCode

Language support for the [Kestrel](https://github.com/jkpdino/kestrel) programming language: syntax highlighting plus a language server backed by the Kestrel compiler.

## Features

- TextMate syntax highlighting for `.ks` files
- Live diagnostics from the Kestrel compiler (lex, parse, type inference, analyzers)
- More IDE features (hover, go-to-definition, completion, rename, formatting) land in subsequent milestones — see `lib/kestrel-lsp/CHECKLIST.md`

## Setup

1. Build the language server:

   ```sh
   cargo build -p kestrel-lsp --release
   ```

2. Make `kestrel-lsp` discoverable. Either:
   - put `target/release/kestrel-lsp` on your `PATH`, or
   - set `kestrel.lsp.path` in VSCode settings to its absolute path.

3. Open any `.ks` file or a folder containing a `flock.toml`.

## Settings

| Key | Default | What it does |
|---|---|---|
| `kestrel.lsp.path` | `kestrel-lsp` | Path to the language server binary |
| `kestrel.lsp.trace.server` | `off` | LSP message tracing (`off` / `messages` / `verbose`) |
| `kestrel.stdlibPath` | `""` | Override the stdlib search path (empty = compiler default) |

## Local development

```sh
cd editors/vscode
npm install
npm run compile
code --extensionDevelopmentPath="$(pwd)" /path/to/kestrel/project
```
