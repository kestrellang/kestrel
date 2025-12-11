# Query Style Guide

Conventions for semantic queries in `kestrel-semantic-model`.

## Core Principles

1. **Pure**: Same model state + same inputs = same output. No mutation, no side effects.
2. **Composable**: Queries should call other queries, not duplicate logic.
3. **Focused**: Each query answers one question.

## Naming

```rust
// Query structs: noun phrases, NO "Query" suffix
ScopeFor { symbol_id }
ExtensionsFor { target_id }
ResolveTypePath { path, context }
IsVisibleFrom { target, context }

// Always use named fields (not tuple structs)
// BAD: ResolveTypePath(Vec<String>, SymbolId) — which is which?

// Output enums: {What}Resolution for complex results
TypePathResolution { Resolved(Ty), NotFound { .. }, Ambiguous { .. } }
```

## Structure

```rust
/// Resolves a type path to its corresponding type.
pub struct ResolveTypePath {
    pub path: Vec<String>,
    pub context: SymbolId,
}

impl Query for ResolveTypePath {
    type Output = TypePathResolution;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        // Compose with other queries
        let scope = model.query(ScopeFor { symbol_id: self.context });
        // ...
    }
}
```

## Composition

```rust
// GOOD: Reuse queries
fn execute(self, model: &SemanticModel) -> Self::Output {
    let scope = model.query(ScopeFor { symbol_id: self.context });
    // Use scope...
}

// BAD: Duplicate scope logic
fn execute(self, model: &SemanticModel) -> Self::Output {
    let mut imports = HashMap::new();  // Don't reimplement!
    // ...
}
```

## Error Handling

Use result enums for queries where "not found" is normal:

```rust
// GOOD
enum TypePathResolution {
    Resolved(Ty),
    NotFound { segment: String, index: usize },
}

// BAD: Result implies "not found" is exceptional
type Output = Result<Ty, ResolutionError>;
```

## Dependency Levels

```
L0: SymbolById, ExtensionsFor
L1: ScopeFor (uses L0)
L2: IsVisibleFrom, VisibleChildrenOf (uses L0-L1)
L3: ResolveName, ResolveModulePath (uses L0-L2)
L4: ResolveTypePath, ResolveValuePath (uses L0-L3)
L5: ApplicableExtensions, ProtocolStaticMethods (uses L0-L4)
```

Higher levels compose lower levels. No circular dependencies.
