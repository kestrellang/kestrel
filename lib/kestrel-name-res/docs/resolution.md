# Name, Type, and Value Resolution

Three resolution queries handle different kinds of paths. All build on the scope chain walk.

## ResolveModulePath

`ResolveModulePath { path, root }` — resolves dotted module paths like `["std", "collections"]` to a module entity.

- First segment: BFS from root to find a module by name
- Remaining segments: direct child lookup at each step
- Returns `Option<Entity>`

Also provides `StdModules { root }` which collects all leaf submodules of `std` (used for auto-imports).

## ResolveName

`ResolveName { name, context, root }` — resolves a simple (single-segment) name by walking the scope chain upward.

### Resolution Order (per scope level)

At each scope in the chain, these are checked in priority order:

1. **Selective imports** — from `import A.B.(Foo)` or `import A.B as X`
2. **Local declarations** — children with matching names
3. **Wildcard imports** — visible members from wildcard import modules
4. **Extension type params** — if inside an extension, the target type's type parameters
5. **Protocol extension associated types** — if inside a protocol extension, the protocol's associated types
6. **Inherited protocol members** — if inside a protocol, walk the conformance hierarchy

If nothing is found at the current scope, walks to the parent scope and repeats.

### Result

```rust
enum NameResolution {
    Found(Vec<Entity>),     // one or more matches (multiple = function overloads)
    Ambiguous(Vec<Entity>), // multiple conflicting non-function matches
    NotFound,
}
```

Function overloading: multiple functions with the same name produce `Found(vec)`. Multiple non-function matches produce `Ambiguous`.

## ResolveTypePath

`ResolveTypePath { path, context, root }` — resolves type paths (from `AstType`) to type entities.

### Algorithm

- First segment resolved via `ResolveName`, filtered to entities with `Typed` marker or `TypeParameter` kind
- Remaining segments resolved by walking children via `VisibleChildrenByName`

### Special Cases

**`Self` keyword**: Bare `Self` returns `SelfType` (resolved contextually by the caller). Multi-segment `Self.Item` tries to resolve through a synthetic type parameter that the AST builder creates for protocol extensions.

**Type parameter associated types**: For paths like `T.Item` where `T` is a type parameter, the resolver:
1. Walks the ancestor chain collecting all where-clause constraints mentioning `T`
2. Searches protocol bounds for an associated type named `Item`
3. Handles nested paths like `T.Iter.Item` by walking through multiple associated type layers

**`lang.*` types**: Built-in types (`Int64`, `String`, etc.) are real entities seeded by the AST builder. They resolve normally through name resolution — no special cases in the resolver.

### Result

```rust
enum TypeResolution {
    Found(Entity),        // resolved to struct, enum, protocol, alias, or type param
    SelfType,             // bare `Self` keyword
    NotFound(String),     // name not found
    NotAType(Entity),     // resolved but entity isn't a type
}
```

## ResolveValuePath

`ResolveValuePath { path, context, root }` — resolves value paths (functions, variables, enum cases, fields).

### Algorithm

- Single-segment: resolved via `ResolveName`
- Multi-segment: first segment via `ResolveName`, then walk children for remaining segments

### Multi-Segment Walk

For each remaining segment:
1. If current entity is a **type alias**, resolve through to the underlying type (transparent)
2. Try **direct children** via `VisibleChildrenByName`
3. If last segment and no direct children, try **extension static methods**
4. Check for **enum cases** or **fields** as intermediate values

### Result

```rust
enum ValueResolution {
    Def(Entity),                               // single definition
    Overloaded(Vec<Entity>),                   // multiple overloaded functions
    Ambiguous(Vec<Entity>),                    // conflicting non-function matches
    TypeParameter(Entity),                     // type param used as value
    AssociatedType { entity, container },       // associated type in protocol
    EnumCaseValue { entity, resolved_index },   // enum case as intermediate value
    FieldValue { entity, resolved_index },      // field/getter as intermediate value
    NotFound(String),
}
```

Type aliases resolve transparently — `TypeAlias.staticMethod()` works by following the alias to its target.
