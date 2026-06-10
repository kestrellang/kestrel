# kestrel-name-res Architecture

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
| [resolution](resolution.md) | Resolve names, types, values, and modules | `ResolveName`, `ResolveTypePath`, `ResolveValuePath`, `ResolveModulePath`, `StdModules` |
| [extensions](extensions.md) | Extension target resolution | `ExtensionTargetEntity`, `ExtensionsFor` |
| conformances | Transitive protocol conformance | `ConformingProtocols`, `ConformingProtocolInstantiations` |
| member discovery | Name-indexed member maps for types and protocols | `TypeMembers`, `TypeMembersByName`, `ProtocolMembers`, `ProtocolMembersByName`, `ProtocolAssociatedTypes` |
| builtins | `@builtin` attribute ↔ entity mapping | `EntityBuiltin`, `BuiltinIndex`, `ResolveBuiltin` |

The first four modules have dedicated docs; the conformance, member-discovery, and builtin queries (plus `StdModules`) are described below.

## Conformances

`ConformingProtocols { entity, root }` collects every protocol a type transitively conforms to, walking: direct `Conformances` on the type, conformances declared on extensions of the type, and the protocol-inheritance closure — including conformances added via `extend P: Q`. Returns a deduplicated `Vec<Entity>`; callers check membership.

`ConformingProtocolInstantiations { entity, root }` is the witness-generation variant: it preserves each conformance's AST type args so distinct instantiations stay separate — `Int64: Convertible[Int8], Convertible[Int16]` yields two entries. Each entry is `(protocol, source, type_args)` where `source` is the entity that declared the conformance (the type itself, or an extension), the correct scope for resolving type-arg names. The dedup key also includes the source's implementing-type args, so overlapping specializations (`extend Box[T]: P` vs `extend Box[lang.i64]: P`) are kept distinct for witness lowering.

`conformances.rs` also exposes non-query helpers: `expand_protocol_closure` / `expand_protocol_closure_in_place` (complete a seed set of protocols — e.g. where-clause bounds — into its transitive closure; used by `kestrel-type-infer`), `find_protocol_witness_init` (conformance-scoped init lookup for literal protocols), and `extract_ast_type_args`.

## Member Discovery

Single source of truth for "what members does this type / protocol have?" Both query families share one traversal — `collect_members_transitive` in `traversal.rs` — and one output shape, the `Arc`'d name-indexed `MemberMap`.

**Traversal order (load-bearing)**: direct children of the queried entity first, then children of every extension targeting it, then each protocol it transitively conforms to (in `ConformingProtocols` order) by the same rule. Protocols additionally surface parent protocols' *direct* children (inheritance pulls in requirements); types only see protocol *extension* members. The order is the precedence guarantee: a direct declaration is always emitted before a same-named extension member, which precedes conformed-protocol extension members — callers folding a bucket (insert-overwrite or first-wins) rely on it.

**Output shape**: the full-member queries return `Arc<MemberMap<M>>` — the flat member list in emission order plus a build-time name index (a `BTreeMap`, so the derived `Hash` is deterministic). `map.named(name)` is a bucket lookup that preserves emission order, replacing the old per-name full re-scan. Nameless inits and subscripts bucket under the reserved keyword sentinels `"init"` (Initializer NodeKind) and `"subscript"` (Subscript marker); keywords can't collide with user-declared member names. `member_lookup_name` in `helpers.rs` is the single source of truth for that naming rule.

- `TypeMembers { type_entity, root }` → `Arc<TypeMemberMap>` — every candidate member of a type (functions, inits, subscripts, fields, type aliases, enum cases) with `TypeMemberSource` provenance (`Direct` / `Extension` / `ProtocolExtension`). Not name-filtered, not visibility-filtered, no where-clause entailment — entailment is the caller's job (see `kestrel_type_infer::entailment`).
- `ProtocolMembers { protocol, root }` → `Arc<ProtocolMemberMap>` — every callable/gettable member reachable from a protocol, with `declaring_protocol` + `extension` provenance. Not visibility-filtered — witnesses dispatch private methods too.
- `ProtocolAssociatedTypes { protocol, root }` → `Vec<ProtocolMember>` — same traversal filtered to unqualified `TypeAlias` children; qualified forms (`type Equal.Output = Bool`) are excluded so concrete bindings don't leak into generic `T.Output` lookups.
- `TypeMembersByName` / `ProtocolMembersByName` → `Vec<…>` — compose the map queries with the sentinel-aware name bucket plus an `IsVisibleFrom` check. The visibility check is per-`context`, so it can't be baked into the cached map.

## Builtins

Three queries connect the `@builtin(.Feature)` attribute system to entities:

- `EntityBuiltin { entity }` — forward lookup: extract the `Builtin` from an entity's `Attributes` component, if any.
- `BuiltinIndex { root }` — scans the whole hierarchy into an `Arc<BuiltinMap>` (`Builtin → Entity`); cached per revision so the scan runs at most once.
- `ResolveBuiltin { builtin, root }` — reverse lookup: tries name-based `ResolveTypePath` first (auto-imported types like `Addable`, `Bool`), then falls back to the attribute index for features not importable by name (e.g. `OptionalEnum`).

## Std Modules

`StdModules { root }` (in `resolve_module.rs`) collects every submodule of `std` — including non-leaf parents like `std.text`, which contain declarations of their own — for auto-importing stdlib declarations into user code.

## Shared Helpers

`helpers.rs` holds cross-query utilities that record dependencies through `QueryContext` automatically: hierarchy walks (`ancestor_module`, `is_ancestor_of`, `find_children_by_name`, `is_in_std_module`), the member-name rule (`member_lookup_name` / `member_name_matches`, including the init/subscript sentinels), and shared filters (`filter_members_by_name` behind the `*MembersByName` queries, `find_in_extensions` for value-path resolution). `traversal.rs` holds the shared member walk and the `MemberMap` type.

## Dependencies

- **kestrel-hecs**: ECS world and query context
- **kestrel-ast-builder**: Components (`Name`, `NodeKind`, `Vis`, `TypeParams`, `Conformances`, `ExtensionTarget`, `ImportItems`, `ModulePath`, etc.)
- **kestrel-ast**: AST types (`AstType`, `PathSegment`)
- **kestrel-hir**: `Builtin` enum (resolve_builtin)

## Source Files

```
src/
├── lib.rs              # Public API re-exports
├── scope.rs            # Scope construction
├── visibility.rs       # Visibility checks
├── resolve_module.rs   # Module path resolution + StdModules
├── resolve_name.rs     # Simple name resolution (scope chain walk)
├── resolve_type.rs     # Type path resolution
├── resolve_value.rs    # Value path resolution
├── resolve_builtin.rs  # @builtin attribute ↔ entity mapping
├── extensions.rs       # Extension target resolution
├── conformances.rs     # Transitive protocol conformance
├── type_members.rs     # Type member discovery
├── protocol_members.rs # Protocol member discovery
├── traversal.rs        # Shared member walk + MemberMap
└── helpers.rs          # Common hierarchy walking utilities
```
