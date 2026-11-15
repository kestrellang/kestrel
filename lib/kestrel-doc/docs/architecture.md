# kestrel-doc Architecture

Generates the stdlib reference. It compiles a Kestrel source tree, walks the
public declarations, and writes JSON for the website and markdown for
`docs/stdlib/` (which Context7 indexes). The `stdlib-docs` skill says when and
how to run it.

## Pipeline Position

```
lang/std ──► kestrel-compiler (front end only) ──► World ──► extract ──► JSON pages  (website)
                                                                   └──► markdown    (docs/stdlib/)
```

It only needs the front end: names, signatures and `///` comments. Nothing is
lowered to MIR.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `ModuleIndex` / `ModuleSummary` | `lib.rs` | The list of modules |
| `ModulePage` | `lib.rs` | One module's items |
| `Item` | `lib.rs` | One documented declaration: kind, signature, doc text, members |
| `MemberGroup` | `lib.rs` | Members grouped by kind (inits, methods, subscripts, …) |

Entry point: `extract(world, root) -> (ModuleIndex, Vec<ModulePage>)`.

## Module Map

| File | Responsibility |
|------|---------------|
| `main.rs` | CLI: `--src`, `--out` (JSON), `--md-out`, `--format json|markdown|both` |
| `lib.rs` | The walk and extraction |
| `signature.rs` | Structured signatures for every declaration kind |
| `markdown.rs` | One `<module>.md` per page |
| `repro_test.rs` | Regression tests for extraction |

## Key Design Decisions

- **One walk, two outputs.** JSON and markdown come from the same `ModulePage`s,
  so the website and `docs/stdlib/` can't drift. Regenerate both together when
  the website checkout is present.
- **Fix rendering here, not in the source.** If an item renders badly, change
  `markdown.rs` or `signature.rs` rather than reshaping the `.ks` comment.
- `@name` tags on inits and subscripts give each overload its own label.

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-compiler` | Building the world |
| `kestrel-ast`, `kestrel-ast-builder`, `kestrel-hecs`, `kestrel-name-res`, `kestrel-syntax-tree` | Reading declarations, names and doc comments |
| `clap`, `serde`, `serde_json` | CLI and JSON output |
