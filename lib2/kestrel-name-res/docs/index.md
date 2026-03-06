# kestrel-name-res

Name resolution for the Kestrel ECS-based compiler pipeline. Resolves textual names and paths to their corresponding ECS entities, sitting between AST building and HIR lowering.

## Design Principles

**Query-based**: All resolution is implemented as `QueryFn` queries against the `kestrel-hecs` world. Queries are cached, composable, and enable incremental computation.

**ECS-native**: No separate symbol table. The module tree, type nesting, and all relationships are represented as entities with components (`Name`, `Vis`, `NodeKind`, etc.) and parent pointers. Resolution walks these structures directly.

**Lazy scopes**: Scopes are computed per-entity on demand (not eagerly during AST building), then cached as query results.

## Pipeline Position

```
Parsing → AST Building → Name Resolution → HIR Lowering → Type Inference → Codegen
                              ^^^
                           this crate
```

AST building creates declaration entities in the HECS world. Name resolution provides queries that map names/paths to those entities. HIR lowering calls these queries to produce entity-based IR.

## Modules

| Module | Purpose | Key Queries |
|--------|---------|-------------|
| [scope](scope.md) | Build resolved scopes per entity | `ScopeFor` |
| [visibility](visibility.md) | Visibility modifier checks | `IsVisibleFrom`, `VisibleChildrenByName` |
| [resolution](resolution.md) | Resolve names, types, and values | `ResolveName`, `ResolveTypePath`, `ResolveValuePath`, `ResolveModulePath` |
| [extensions](extensions.md) | Extension target resolution | `ExtensionTargetEntity`, `ExtensionsFor` |

## Dependencies

- **kestrel-hecs**: ECS world and query context
- **kestrel-ast-builder**: Components (`Name`, `NodeKind`, `Vis`, `TypeParams`, `Conformances`, `ExtensionTarget`, `ImportItems`, `ModulePath`, etc.)
- **kestrel-ast**: AST types (`AstType`, `PathSegment`)

## Source Files

```
src/
├── lib.rs              # Public API re-exports
├── scope.rs            # Scope construction
├── visibility.rs       # Visibility checks
├── resolve_module.rs   # Module path resolution
├── resolve_name.rs     # Simple name resolution (scope chain walk)
├── resolve_type.rs     # Type path resolution
├── resolve_value.rs    # Value path resolution
├── extensions.rs       # Extension target resolution
└── helpers.rs          # Common hierarchy walking utilities
```
