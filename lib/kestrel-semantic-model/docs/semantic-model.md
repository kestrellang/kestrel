# Semantic Model

The `SemanticModel` is the central interface for querying semantic information about Kestrel code.

## Overview

```
Source Files → BUILD → SemanticTree → BIND → SemanticModel
                                                   ↓
                                            model.query(...)
```

- **BUILD phase**: Constructs symbol tree from syntax (parent-child, names, spans)
- **BIND phase**: Resolves references, registers extensions, validates
- **SemanticModel**: Owns the bound tree, answers queries

## What It Owns

```rust
pub struct SemanticModel {
    root: Arc<dyn Symbol<KestrelLanguage>>,    // Symbol tree
    syntax_map: HashMap<SymbolId, SyntaxNode>, // For syntax lookup
    sources: HashMap<String, String>,          // Source code by file
    registry: SymbolRegistry,                  // Symbol index by ID and name
    extension_registry: ExtensionRegistry,     // Extensions by target type
}
```

## Query Interface

```rust
// Ask questions via queries
let scope = model.query(ScopeFor { symbol_id });
let ty = model.query(ResolveTypePath { path: vec!["Option"], context: fn_id });
let extensions = model.query(ExtensionsFor { target_id: struct_id });
```

Queries are:
- **Pure**: Same inputs → same output
- **Composable**: Higher-level queries call lower-level ones
- **Typed**: Each query struct defines its output type

## Architecture

```
kestrel-semantic-tree          # Symbol/Behavior definitions
        ↑
kestrel-semantic-model         # SemanticModel + Query trait + queries
        ↑
kestrel-semantic-tree-builder  # BUILD + BIND phases
        ↑
kestrel-compiler               # High-level Compilation API
```

## Registries

**SymbolRegistry**: Indexes all symbols for O(1) lookup
- By `SymbolId`
- By `(Kind, Name)` for module path resolution

**ExtensionRegistry**: Maps target types to their extensions
- `StructId → Vec<ExtensionSymbol>`

These are implementation details. External code uses queries.
