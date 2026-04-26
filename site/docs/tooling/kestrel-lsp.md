# Kestrel LSP

The language server (`kestrel-lsp`) implements the standard LSP protocol. Anything it offers, your editor will surface — the exact UI depends on the editor. For the install, see [Getting Started → Install the LSP Extension](../getting-started/lsp-extension.md).

## What the server provides

- **Diagnostics.** Errors and warnings as you type, with E-codes that link to [Reference → Diagnostics](../reference/diagnostics.md).
- **Completion.** Names in scope, member access (`obj.`), and module paths (`std.io.`). Completions are HIR-driven, so they reflect what would actually compile, not just what's lexically near.
- **Hover.** Type signatures, doc comments, and the resolved type of any expression.
- **Jump-to-definition.** Works across modules, including stdlib and dependencies.
- **Find-references.** All call sites for a function, all reads of a field, all conformances of a protocol.
- **Rename.** Symbol-aware. Renames the declaration and every reference.
- **Document symbols.** The outline pane in your editor.
- **Semantic tokens.** Better syntax highlighting than a regex-based grammar — the server tells the editor exactly what each token resolves to.
- **Signature help.** Inline parameter hints when you're typing a call.
- **Code lens.** Inline run/test/debug actions next to functions and tests.

## Configuration

The server reads `kestrel-lsp.toml` from the project root if present:

```toml
[diagnostics]
unused-binding = "warn"   # off | warn | error

[completion]
include-stdlib = true
auto-import = true
```

Most users don't need to configure anything. The defaults are tuned for typical projects.

## Performance

The server is incremental — edits trigger only the analysis affected by the change, not a full re-typecheck. Large projects (thousands of files) stay responsive because the dependency graph is tracked at the symbol level.

## Troubleshooting

- **No completions, no diagnostics**: the server isn't reaching the project. Check that `flock.toml` exists at the root and `kestrel-lsp` is on your `PATH`.
- **Stale diagnostics**: the file system watcher missed an event. Reload the file or restart the LSP from your editor's command palette.
- **High CPU**: usually means an analysis loop. Capture the issue with `KESTREL_LSP_LOG=trace` and file a bug.

---

[← Flock](flock.md) · [↑ Tooling](index.md) · [Jessup →](jessup.md)
