# Install the LSP Extension

The Kestrel language server gives your editor everything you'd expect from a modern setup: real-time diagnostics, completion, hover, jump-to-definition, find-references, rename, document outline. Install once per editor.

## VS Code

Search for **"Kestrel"** in the Extensions sidebar (or run **Extensions: Install Extensions** and search there). The official extension is published as `kestrel-lang.kestrel`.

After installing, open any `.ks` file. The status bar will show "Kestrel" once the language server is up — usually within a second or two.

## Other editors

Anything that speaks LSP can drive the Kestrel server directly. The binary is `kestrel-lsp`, installed alongside the rest of the toolchain.

**Neovim** (with `nvim-lspconfig`):

```lua
require('lspconfig').kestrel.setup {
  cmd = { "kestrel-lsp" },
  filetypes = { "kestrel" },
  root_dir = require('lspconfig.util').root_pattern("flock.toml"),
}
```

**Helix** (`languages.toml`):

```toml
[[language]]
name = "kestrel"
file-types = ["ks"]
language-server = { command = "kestrel-lsp" }
roots = ["flock.toml"]
```

**Zed**: install the official Kestrel extension from the Extensions panel — same shape as VS Code.

## Verifying

Open any `.ks` file and trigger completion (the editor's normal shortcut, usually Ctrl-Space). If you see suggestions from the standard library, you're set.

If completion stays empty, the language server isn't reaching your project. Common causes: no `flock.toml` at the project root, or `kestrel-lsp` isn't on your `PATH`. Check both before assuming it's a bug.

## Deeper LSP coverage

For everything the language server can do — code lens, semantic tokens, signature help, diagnostics tuning — see [Tooling → Kestrel LSP](../tooling/kestrel-lsp.md). This page is just the install.

---

[← Kestrel Skill](kestrel-skill.md) · [↑ Getting Started](index.md) · [A Tour of Kestrel →](../tour/index.md)
