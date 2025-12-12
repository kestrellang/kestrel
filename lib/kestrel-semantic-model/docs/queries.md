# Query Reference

This document describes all queries available in `kestrel-semantic-model`.

## Overview

Queries are the primary way to retrieve information from a `SemanticModel`. Each query is a struct that implements the `Query` trait:

```rust
model.query(QueryName { field1: value1, field2: value2 })
```

Queries are:
- **Pure**: Same inputs + same model state = same output
- **Composable**: Queries can call other queries internally
- **Cacheable**: Results can be memoized (future enhancement)

## Query Categories

### Symbol Lookup

| Query | Output | Description |
|-------|--------|-------------|
| `SymbolFor` | `Option<Arc<dyn Symbol>>` | Get a symbol by ID |
| `ChildByName` | `Option<Arc<dyn Symbol>>` | Find child by name (no visibility check) |
| `AncestorOfKind` | `Option<SymbolId>` | Find nearest ancestor of a specific kind |

### Scope & Imports

| Query | Output | Description |
|-------|--------|-------------|
| `ScopeFor` | `Arc<Scope>` | Get scope (declarations + parent chain) for a symbol |
| `ImportsInScope` | `Vec<Arc<Import>>` | Get all imports declared in a symbol's scope |

### Visibility

| Query | Output | Description |
|-------|--------|-------------|
| `IsVisibleFrom` | `bool` | Check if target is visible from context |
| `VisibleChildren` | `Vec<Arc<dyn Symbol>>` | Get children visible from a context |
| `VisibleChildrenByName` | `Vec<Arc<dyn Symbol>>` | Get visible children matching a name |

### Extensions

| Query | Output | Description |
|-------|--------|-------------|
| `ExtensionsFor` | `Vec<Arc<ExtensionSymbol>>` | Get all extensions for a target type |

### Name Resolution

| Query | Output | Description |
|-------|--------|-------------|
| `ResolveName` | `SymbolResolution` | Resolve a single name in scope |
| `ResolveModulePath` | `Result<SymbolId, ModuleNotFoundError>` | Resolve a module path |
| `ResolveTypePath` | `TypePathResolution` | Resolve a type path to a `Ty` |
| `ResolveValuePath` | `ValuePathResolution` | Resolve a value path |

### Protocol Members

| Query | Output | Description |
|-------|--------|-------------|
| `InheritedProtocolMember` | `Option<SymbolId>` | Search inherited protocols for a member |

---

## Detailed Query Reference

### SymbolFor

Get a symbol by its ID.

```rust
pub struct SymbolFor {
    pub id: SymbolId,
}
// Output: Option<Arc<dyn Symbol<KestrelLanguage>>>
```

**Example:**
```rust
if let Some(symbol) = model.query(SymbolFor { id: some_id }) {
    println!("Found: {}", symbol.metadata().name().value);
}
```

---

### ExtensionsFor

Get all extensions registered for a target type.

```rust
pub struct ExtensionsFor {
    pub target_id: SymbolId,
}
// Output: Vec<Arc<ExtensionSymbol>>
```

**Example:**
```rust
let extensions = model.query(ExtensionsFor { target_id: struct_id });
for ext in extensions {
    // Process each extension
}
```

---

### ScopeFor

Get the scope for a symbol. The scope contains:
- `symbol_id`: The symbol this scope belongs to
- `imports`: Imported names (empty during initial construction)
- `declarations`: Non-import children mapped by name
- `parent`: Parent symbol's ID for scope chain lookup

```rust
pub struct ScopeFor {
    pub symbol_id: SymbolId,
}
// Output: Arc<Scope>
```

**Example:**
```rust
let scope = model.query(ScopeFor { symbol_id: fn_id });
if let Some(decls) = scope.declarations.get("foo") {
    // Found declarations named "foo"
}
```

---

### ImportsInScope

Get all imports declared in a symbol's scope. Returns import metadata extracted from `ImportDataBehavior`.

```rust
pub struct ImportsInScope {
    pub symbol_id: SymbolId,
}
// Output: Vec<Arc<Import>>
```

---

### ChildByName

Find a child symbol by name without visibility checking.

```rust
pub struct ChildByName {
    pub parent: SymbolId,
    pub name: String,
}
// Output: Option<Arc<dyn Symbol<KestrelLanguage>>>
```

**Example:**
```rust
if let Some(field) = model.query(ChildByName {
    parent: struct_id,
    name: "x".to_string()
}) {
    // Found field "x"
}
```

---

### VisibleChildren

Get all children of a parent that are visible from a given context.

```rust
pub struct VisibleChildren {
    pub parent: SymbolId,
    pub context: SymbolId,
}
// Output: Vec<Arc<dyn Symbol<KestrelLanguage>>>
```

**Composes:** `SymbolFor`, `IsVisibleFrom`

---

### VisibleChildrenByName

Find children matching a name that are visible from a context. Combines lookup with visibility checking.

```rust
pub struct VisibleChildrenByName {
    pub parent: SymbolId,
    pub name: String,
    pub context: SymbolId,
}
// Output: Vec<Arc<dyn Symbol<KestrelLanguage>>>
```

**Composes:** `SymbolFor`, `IsVisibleFrom`

---

### IsVisibleFrom

Check if a target symbol is visible from a context. Applies visibility rules:
- `public`: Always visible
- `private`: Only visible within the declaring scope and descendants
- `internal`: Visible within the same module
- `fileprivate`: Visible within the declaring file scope

```rust
pub struct IsVisibleFrom {
    pub target: SymbolId,
    pub context: SymbolId,
}
// Output: bool
```

**Composes:** `SymbolFor`, `AncestorOfKind`

---

### AncestorOfKind

Find the nearest ancestor of a specific kind. Walks up the symbol tree.

```rust
pub struct AncestorOfKind {
    pub symbol_id: SymbolId,
    pub kind: KestrelSymbolKind,
}
// Output: Option<SymbolId>
```

**Example:**
```rust
// Find the module containing this symbol
let module_id = model.query(AncestorOfKind {
    symbol_id: fn_id,
    kind: KestrelSymbolKind::Module,
});
```

---

### InheritedProtocolMember

Search inherited protocols for a member (e.g., associated type). Given a protocol and a name, searches parent protocols via conformances.

```rust
pub struct InheritedProtocolMember {
    pub protocol_id: SymbolId,
    pub name: String,
}
// Output: Option<SymbolId>
```

---

### ResolveName

Resolve a single name in a given scope context. Walks up the scope chain checking:
1. Imports
2. Declarations
3. Extension type parameters (for extension contexts)
4. Inherited protocol members (for protocol contexts)

```rust
pub struct ResolveName {
    pub name: String,
    pub context: SymbolId,
}
// Output: SymbolResolution
```

**Output variants:**
- `Found(Vec<SymbolId>)`: Successfully resolved
- `NotFound`: Name not found in any scope
- `Ambiguous(Vec<SymbolId>)`: Multiple candidates

**Composes:** `ScopeFor`, `SymbolFor`, `InheritedProtocolMember`

---

### ResolveModulePath

Resolve a module path like `["A", "B", "C"]` to a module symbol.

```rust
pub struct ResolveModulePath {
    pub path: Vec<String>,
    pub context: SymbolId,
}
// Output: Result<SymbolId, ModuleNotFoundError>
```

The first segment is looked up via the kind+name index, subsequent segments are resolved as visible children.

---

### ResolveTypePath

Resolve a type path to a `Ty`. Handles:
- Primitive types (`Int`, `Bool`, `String`, etc.)
- User-defined types via scope resolution
- Type parameters
- Associated types (including `T.Item` style)

```rust
pub struct ResolveTypePath {
    pub path: Vec<String>,
    pub context: SymbolId,
}
// Output: TypePathResolution
```

**Output variants:**
- `Resolved(Ty)`: Successfully resolved to a type
- `NotFound { segment, index }`: Segment not found at index
- `Ambiguous { segment, index, candidates }`: Multiple candidates
- `NotAType { symbol_id }`: Symbol exists but isn't a type

**Composes:** `ResolveName`, `SymbolFor`, `VisibleChildrenByName`, `InheritedProtocolMember`

---

### ResolveValuePath

Resolve a value path (variable, function, static method). Handles:
- Variables and constants
- Functions (including overloads)
- Static methods on types (including from extensions)
- Type parameters (for static method calls like `T.create()`)

```rust
pub struct ResolveValuePath {
    pub path: Vec<String>,
    pub context: SymbolId,
}
// Output: ValuePathResolution
```

**Output variants:**
- `Symbol { symbol_id, ty }`: Resolved to a single symbol with type
- `Overloaded { candidates }`: Multiple function overloads
- `TypeParameter { symbol_id }`: Resolved to a type parameter
- `NotFound { segment, index }`: Segment not found
- `Ambiguous { segment, index, candidates }`: Non-function ambiguity
- `NotAValue { symbol_id }`: Symbol exists but isn't a value

**Composes:** `ResolveName`, `SymbolFor`, `ExtensionsFor`, `VisibleChildrenByName`, `IsVisibleFrom`

---

## Query Dependency Graph

```
Level 0 (Direct Lookup):
├── SymbolFor
└── ExtensionsFor

Level 1 (Scope):
├── ScopeFor          → SymbolFor
└── ImportsInScope    → SymbolFor

Level 2 (Visibility & Hierarchy):
├── AncestorOfKind    → SymbolFor
├── IsVisibleFrom     → SymbolFor, AncestorOfKind
├── ChildByName       → SymbolFor
├── VisibleChildren   → SymbolFor, IsVisibleFrom
└── VisibleChildrenByName → SymbolFor, IsVisibleFrom

Level 3 (Name Resolution):
├── ResolveName       → ScopeFor, SymbolFor, InheritedProtocolMember
├── ResolveModulePath → (registry lookup)
└── InheritedProtocolMember → SymbolFor

Level 4 (Path Resolution):
├── ResolveTypePath   → ResolveName, SymbolFor, VisibleChildrenByName, InheritedProtocolMember
└── ResolveValuePath  → ResolveName, SymbolFor, ExtensionsFor, VisibleChildrenByName, IsVisibleFrom
```
