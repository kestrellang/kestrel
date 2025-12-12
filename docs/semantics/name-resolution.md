# Name Resolution

Name resolution is the process of finding which declaration a name refers to. This applies to type references, function calls, and variable access.

## Overview

When code uses a name like `MyClass` or `process`, the compiler must determine which declaration it refers to. This involves:

1. Looking in the current scope
2. Checking imports
3. Walking up the scope chain
4. Applying visibility rules

## Scopes

### Scope Structure

Each scope contains:

```rust
struct Scope {
    symbol_id: SymbolId,                        // The symbol that owns this scope
    imports: HashMap<String, Vec<SymbolId>>,    // Names from imports
    declarations: HashMap<String, Vec<SymbolId>>, // Direct child declarations
    parent: Option<SymbolId>,                   // Parent scope
}
```

### Scope Hierarchy

```
Root (implicit)
└── Module scope
    ├── Class scope
    │   ├── Nested struct scope
    │   └── Method scope (future)
    ├── Struct scope
    │   └── Method scope (future)
    └── Function scope (future)
```

### Scope-Creating Symbols

| Symbol Kind | Creates Scope |
|-------------|---------------|
| Module | Yes |
| Struct | Yes |
| Struct | Yes |
| Protocol | Yes |
| SourceFile | Yes (transparent) |
| Function | Not yet |
| Field | No |
| TypeAlias | No |
| Import | No |

### Transparent Scopes

`SourceFile` symbols are **transparent**—name lookups skip through them and surface their children to the parent scope:

```kestrel
// file.kes
module MyApp

struct MyStruct { }    // MyClass is in SourceFile, but visible as MyApp.MyClass
```

## Resolution Algorithm

### Basic Name Resolution

```
resolve_name(name, context):
    current = Some(context)

    while let Some(scope_id) = current:
        scope = get_scope(scope_id)

        // 1. Check imports first (higher priority)
        if name in scope.imports:
            matches = scope.imports[name]
            if len(matches) == 1:
                return Found(matches[0])
            else:
                return Ambiguous(matches)

        // 2. Check local declarations
        if name in scope.declarations:
            matches = scope.declarations[name]
            if len(matches) == 1:
                return Found(matches[0])
            else:
                return Ambiguous(matches)

        // 3. Walk up to parent scope
        current = scope.parent

    return NotFound
```

### Resolution Result

```rust
enum NameResolution {
    Found(SymbolId),
    Ambiguous(Vec<SymbolId>),
    NotFound,
}
```

## Import Priority

Imports are checked BEFORE local declarations in the current scope. This allows imports to extend the namespace:

```kestrel
module MyApp

import Utils.(Logger)    // Logger added to imports

struct MyStruct {
    func log() {
        Logger.info()    // Found via imports
    }
}
```

However, whole-module imports cannot conflict with local declarations (this is an error, not shadowing).

## Shadowing Rules

### Parent Scope Shadowing

Names in inner scopes shadow names in outer scopes:

```kestrel
module MyApp

struct Logger { }    // Module-level Logger

struct Service {
    struct Logger { }    // Nested Logger shadows module-level

    func process() {
        // Logger refers to Service.Logger, not MyApp.Logger
    }
}
```

### Import Shadowing

Imports shadow names from parent scopes:

```kestrel
module MyApp

struct Helper { }

struct Service {
    // This import would shadow MyApp.Helper in this scope
    // (if imports inside structs were supported)
}
```

### No Local Declaration Shadowing

Whole-module imports cannot shadow local declarations—this is an error:

```kestrel
module MyApp

struct Logger { }

import Utils    // If Utils has Logger, this is an ImportConflictError
```

## Path Resolution

For qualified names like `A.B.C`:

### Two-Phase Resolution

**Phase 1: First Segment**
- Uses full name resolution (imports + declarations + parent chain)

**Phase 2: Subsequent Segments**
- Direct child lookup on the resolved symbol
- Visibility filtering applied

```
resolve_path(segments, context):
    // Phase 1: Resolve first segment via name resolution
    first = resolve_name(segments[0], context)
    if first is NotFound:
        return PathNotFound(segments[0], 0)

    current = first

    // Phase 2: Resolve remaining segments via child lookup
    for i in 1..len(segments):
        segment = segments[i]
        children = visible_children(current, context)
        matches = [c for c in children if c.name == segment]

        if len(matches) == 0:
            return PathNotFound(segment, i)
        if len(matches) > 1:
            return PathAmbiguous(segment, i, matches)

        current = matches[0]

    return PathResolved(current)
```

### Path Resolution Examples

```kestrel
module App

import Lib

struct Container {
    struct Item { }
}

// Path resolution:
// "Container" -> resolve_name finds App.Container
// "Container.Item" -> resolve_name finds Container, then child lookup finds Item
// "Lib.Something" -> resolve_name finds Lib (via import), then child lookup
```

## Scope Computation

Scopes are computed lazily and cached:

```
compute_scope(symbol):
    scope = Scope {
        symbol_id: symbol.id,
        imports: {},
        declarations: {},
        parent: symbol.parent?.id,
    }

    for child in symbol.children:
        if child.kind == Import:
            process_import(child, scope)
        else:
            scope.declarations[child.name].push(child.id)

    return scope
```

### Import Processing

```
process_import(import_symbol, scope):
    import_data = import_symbol.import_data_behavior

    // Specific items
    for item in import_data.items:
        if item.target_id:
            alias = item.alias or item.name
            scope.imports[alias].push(item.target_id)

    // Module alias
    if import_data.alias:
        module = resolve_module_path(import_data.path)
        scope.imports[import_data.alias].push(module.id)

    // Whole-module import (no alias, no items)
    if !import_data.alias and !import_data.items:
        module = resolve_module_path(import_data.path)
        module_scope = get_scope(module.id)
        for (name, ids) in module_scope.declarations:
            for id in ids:
                if is_visible(id, scope.symbol_id):
                    scope.imports[name].push(id)
```

## Visibility Filtering

During name resolution, visibility is considered:

```
visible_children(symbol, context):
    return [child for child in symbol.children
            if is_visible_from(child, context)]
```

See [Visibility](visibility.md) for visibility rules.

## Ambiguity

### When Ambiguity Occurs

Names can be ambiguous when:

1. Multiple declarations with same name in same scope (e.g., function overloads)
2. Multiple imports bring in the same name

### Handling Ambiguity

For types, ambiguity is usually an error:

```kestrel
import A.(Thing)
import B.(Thing)

let x: Thing    // ERROR: ambiguous - could be A.Thing or B.Thing
```

For functions, overload resolution (future) will select based on arguments.

## Examples

### Simple Name Resolution

```kestrel
module App

struct Helper { }

struct Service {
    func process() {
        // "Helper" resolved:
        // 1. Check Service scope imports: not found
        // 2. Check Service scope declarations: not found
        // 3. Check parent (App module) scope imports: not found
        // 4. Check parent (App module) scope declarations: found!
        let h: Helper
    }
}
```

### Import Resolution

```kestrel
module App

import Utils.(Logger)

struct Service {
    func process() {
        // "Logger" resolved:
        // 1. Check Service scope imports: not found
        // 2. Check Service scope declarations: not found
        // 3. Check parent (App module) scope imports: found!
        Logger.info()
    }
}
```

### Qualified Path Resolution

```kestrel
module App

import Lib

struct Service {
    func process() {
        // "Lib.Helper" resolved:
        // Phase 1: "Lib" found via import
        // Phase 2: "Helper" found as child of Lib module
        let h: Lib.Helper
    }
}
```

### Shadowing Example

```kestrel
module App

struct Config { }

struct Service {
    struct Config { }    // Shadows App.Config

    func process() {
        // "Config" refers to Service.Config (inner scope wins)
        let c: Config
    }
}
```

## Formal Semantics

### Resolution Ordering

```
Priority order for name N in scope S:
    1. S.imports[N]           // Highest priority
    2. S.declarations[N]
    3. resolve_name(N, S.parent)  // Recursively check parent
```

### Visibility Predicate

```
visible_from(symbol, context) iff:
    symbol.visibility == Public OR
    (symbol.visibility == Private AND
     (context == symbol.visibility_scope OR
      is_ancestor(context, symbol.visibility_scope)))
```

### Path Resolution Predicate

```
path_resolves(p₁.p₂...pₙ, context) iff:
    resolve_name(p₁, context) = s₁ AND
    ∀i ∈ [2,n]: sᵢ ∈ visible_children(sᵢ₋₁, context) AND sᵢ.name = pᵢ
```

## Source Location

- **Name resolution:** `lib/kestrel-semantic-model/src/queries/resolve_name.rs`
- **Scope computation:** `lib/kestrel-semantic-model/src/queries/scope_for.rs`
- **Imports in scope:** `lib/kestrel-semantic-model/src/queries/imports_in_scope.rs`
- **Value path resolution:** `lib/kestrel-semantic-model/src/queries/resolve_value_path.rs`
- **Visibility checking:** `lib/kestrel-semantic-model/src/queries/is_visible_from.rs`
