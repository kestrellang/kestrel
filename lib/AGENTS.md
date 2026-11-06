# lib Crate Documentation Guide

Every crate in `lib/` has a `docs/` folder containing its documentation. This file describes the conventions for writing and maintaining these docs.

## Documentation Structure

Every crate MUST have:

```
lib/kestrel-<name>/
└── docs/
    ├── architecture.md    ← required: concise overview and entry point
    └── <topic>.md         ← optional: detailed docs for specific subsystems
```

### `architecture.md` — The Entry Point

This is the first file someone reads. It answers: what does this crate do, where does it sit in the pipeline, and what are the key types?

**Required sections:**

```markdown
# kestrel-<name> Architecture

One-line description of the crate's purpose.

## Pipeline Position

Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → Codegen
                                                       ^^^
                                                    this crate

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| ... | ... | ... |

## Module Map

| File | Responsibility |
|------|---------------|
| ... | ... |

## Dependencies

| Crate | Usage |
|-------|-------|
| ... | ... |
```

**Optional sections** (add when relevant):

- **Algorithm** — step-by-step description of the core processing logic
- **Key Design Decisions** — non-obvious choices with rationale
- **Lifecycle** — for stateful crates (e.g., kestrel-hecs world phases)
- **Queries** — for crates that define HECS queries (input → output table)
- **Known Limitations** — incomplete features or known issues

### Topic Documents

Additional docs cover specific subsystems in depth. Each crate chooses topics based on what's complex enough to warrant separate documentation.

**Common topic patterns:**

| Pattern | When to use | Example |
|---------|-------------|---------|
| Type catalog | Crate defines a major enum with many variants | `ast-expressions.md`, `ast-patterns.md` |
| Algorithm deep dive | Complex multi-step resolution or solving | `resolution.md`, `scope.md` |
| Design document | Comprehensive rationale for the crate's approach | `design.md` |
| Component reference | ECS crate with many component types | `components.md`, `entity-mapping.md` |

### Current Documentation Map

| Crate | Docs |
|-------|------|
| `kestrel-span` | architecture |
| `kestrel-debug` | architecture |
| `kestrel-lexer` | architecture |
| `kestrel-syntax-tree` | architecture |
| `kestrel-parser` | architecture |
| `kestrel-ast` | architecture, ast-types |
| `kestrel-hecs` | architecture, snapshots |
| `kestrel-ast-builder` | architecture, components, entity-mapping |
| `kestrel-name-res` | architecture, scope, visibility, resolution, extensions |
| `kestrel-hir` | architecture, expressions, desugaring, types-and-resolution |
| `kestrel-hir-lower` | architecture, design |
| `kestrel-type-infer` | architecture, design |
| `kestrel-compiler` | architecture |
| `kestrel-target` | architecture |

## Writing Guidelines

### Format

- **Tables over prose.** Use markdown tables for type references, module maps, and variant listings.
- **Pipeline context first.** Always show where the crate sits in the compilation pipeline.
- **Examples before explanation.** Show what something looks like, then explain why.
- **ASCII diagrams.** Use simple ASCII art for flow charts and architecture diagrams — no Mermaid or images.

### Tone

- Direct and concise. Architecture docs should be 50–100 lines. Topic docs can be longer.
- Write for a developer who knows Rust but not this codebase.
- Explain the "what" and "why", not the "how" (that's in the code).

### What NOT to Document

- Implementation details that are obvious from reading the code
- Information that changes frequently (use code comments instead)
- Anything already derivable from `cargo doc` output (API signatures, trait implementations)

### Keeping Docs Current

- When adding a new crate: create `docs/architecture.md` following the template above
- When adding a major subsystem to an existing crate: add a topic document
- When renaming or removing a crate: update this map
- When changing a crate's pipeline position or core types: update its architecture doc

## Formatting

Run `rustfmt --edition 2024 <file>` on the specific files you changed, not
`cargo fmt` and not `rustfmt` on a crate's `lib.rs`: rustfmt follows `mod`
declarations, so formatting a `lib.rs` also reformats every child module, and
several files in the tree are not rustfmt-clean today. Formatting them in a
feature commit buries the real change in unrelated hunks.

## Compiler-generated protocol calls name their requirement — never a string

A call the compiler emits on the user's behalf (desugarings like `for`, `try`,
`?.`, operators and interpolation; promotion; `boolValue`; `@main`'s
`report()`; closure `clone()`; box members) calls one `@builtin`-tagged
**requirement entity**, never "whatever member is named `iter`". A by-name
lookup picks up a same-named witness of a different protocol (E103 ambiguity,
or silently the wrong method — `try` once segfaulted this way).

| Layer | Helper |
|---|---|
| HIR lower | `protocol_requirement(protocol, Builtin)` → `ProtocolCall.requirement` / `ImplicitMember.requirement` |
| Type infer | `requirement_call` (typed from the requirement, `Self` = receiver/expected type); `requirement_breaks_tie` for abstract receivers |
| MIR lower | `protocol_call_target` / `LowerCtx::builtin_witness_key(Builtin)` → `Callee::Witness` |
| Analyzer | `util::protocol_call_method(cx, expr)` |
| Assoc types | name-res `AssociatedTypeBinding` — the only lookup |

- A new requirement tag needs arms in all 4 tables in `kestrel-hir/src/builtin.rs`.
- An **init** requirement (`Interpolatable.init`, `init(interpolation:)`)
  produces `Self` (its declared return is `()`), and MIR must call it with the
  init convention (`emit_init_call`: out-pointer `self` first), not as a plain call.
- Deliberate exception: interpolation's `appendInterpolation` is not a
  requirement — each accumulator declares its own overloads (which hole types
  it accepts), so it is an ordinary method call on the concrete accumulator.
