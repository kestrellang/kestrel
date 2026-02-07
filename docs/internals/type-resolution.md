# Type Resolution

Type resolution converts syntactic type expressions into resolved semantic types. This happens during the bind phase of compilation.

## Overview

During parsing, type annotations are stored as unresolved `Path` types:

```
// Syntax: let x: MyApp.Models.User
// Stored as: Ty::Path(["MyApp", "Models", "User"])
```

During binding, paths are resolved to concrete types:

```
// Resolved to: Ty::Struct { symbol: Arc<StructSymbol>, substitutions }  // where StructSymbol is User
```

## Type Resolution Algorithm

### Main Resolution Function

```
resolve_type(ty, context):
    match ty.kind:
        // Primitives: always resolved
        Unit | Never | Bool | String | Int(_) | Float(_):
            return ty

        // Already resolved nominal types
        Struct { .. } | Enum { .. } | Protocol { .. }:
            return ty

        // Type alias: currently returned as-is
        TypeAlias(_):
            return ty  // TODO: resolve to underlying type

        // Path: needs resolution
        Path(segments):
            return resolve_type_path(segments, context)

        // Tuple: resolve each element
        Tuple(elements):
            resolved = []
            for elem in elements:
                r = resolve_type(elem, context)
                if r is None:
                    return None
                resolved.push(r)
            return Tuple(resolved)

        // Function: resolve params and return type
        Function(params, return_type):
            resolved_params = []
            for param in params:
                r = resolve_type(param, context)
                if r is None:
                    return None
                resolved_params.push(r)
            resolved_return = resolve_type(return_type, context)
            if resolved_return is None:
                return None
            return Function(resolved_params, resolved_return)
```

### Path Resolution

```
resolve_type_path(segments, context):
    // Phase 1: Resolve first segment via name resolution
    first_result = resolve_name(segments[0], context)

    match first_result:
        NotFound:
            return TypePathResolution::NotFound(segments[0], 0)
        Ambiguous(candidates):
            return TypePathResolution::Ambiguous(segments[0], 0, candidates)
        Found(symbol):
            current = symbol

    // Phase 2: Resolve remaining segments via child lookup
    for i in 1..len(segments):
        segment = segments[i]
        matches = [c for c in visible_children(current)
                   if c.name == segment]

        match len(matches):
            0:
                return TypePathResolution::NotFound(segment, i)
            1:
                current = matches[0]
            _:
                return TypePathResolution::Ambiguous(segment, i, matches)

    // Phase 3: Extract type from final symbol
    typed_behavior = current.get_typed_behavior()

    if typed_behavior is None:
        return TypePathResolution::NotAType(current.id)

    return TypePathResolution::Resolved(typed_behavior.ty)
```

## Resolution Results

```rust
enum TypePathResolution {
    // Success: resolved to a concrete type
    Resolved(Ty),

    // Segment not found at given index
    NotFound {
        segment: String,
        index: usize,
    },

    // Multiple symbols match the segment
    Ambiguous {
        segment: String,
        index: usize,
        candidates: Vec<SymbolId>,
    },

    // Symbol exists but isn't a type
    NotAType {
        symbol_id: SymbolId,
    },
}
```

## Resolution Contexts

Types are resolved in the context of the symbol that contains them:

### Function Parameter Types

```kestrel
struct Container {
    func process(x: Item) { }
    //              ^^^^ resolved in context of Container
}
```

The type `Item` is resolved starting from `Container`'s scope.

### Function Return Types

```kestrel
func create() -> Widget { }
//               ^^^^^^ resolved in context of enclosing scope
```

### Field Types

```kestrel
struct Data {
    let value: Config    // resolved in context of Data
}
```

### Type Alias Definitions

```kestrel
type MyList = Container.List    // resolved in context of enclosing module
```

## Type Building

Before resolution, types are built from syntax:

```
build_type(syntax_node):
    match syntax_node:
        TyUnit:
            return Ty::unit(span)

        TyNever:
            return Ty::never(span)

        TyPath:
            segments = extract_path_segments(syntax_node)
            return Ty::path(segments, span)

        TyTuple:
            elements = [build_type(elem) for elem in syntax_node.elements]
            return Ty::tuple(elements, span)

        TyFunction:
            params = [build_type(p) for p in syntax_node.params]
            return_type = build_type(syntax_node.return_type)
            return Ty::function(params, return_type, span)
```

## Type Alias Resolution

Type aliases have special handling:

### Two TypedBehaviors

Type alias symbols have two typed behaviors:
1. **TypedBehavior** - The syntactic aliased type (for analysis)
2. **TypeAliasTypedBehavior** - The resolved type (added during binding)

### Resolution Priority

When resolving a path to a type alias, the TypeAlias type is returned (not the underlying type):

```kestrel
type MyInt = Int;

let x: MyInt    // Type is Ty::TypeAlias(MyInt), not Ty::Int
```

This preserves alias identity for:
- Error messages
- Cycle detection
- Future nominal type alias support

### Resolving Through Aliases

To get the underlying type, follow the alias chain:

```
resolve_through_alias(ty):
    while ty.kind is TypeAlias(alias_symbol):
        ty = alias_symbol.aliased_type
    return ty
```

## Primitive Type Resolution

Primitive types don't need resolution—they're already concrete:

| Type | Resolution |
|------|------------|
| `()` | `Ty::Unit` - no resolution needed |
| `!` | `Ty::Never` - no resolution needed |
| `Bool` | Resolved via path lookup to built-in |
| `String` | Resolved via path lookup to built-in |
| `Int8`, etc. | Resolved via path lookup to built-in |
| `Float32`, etc. | Resolved via path lookup to built-in |

## Composite Type Resolution

### Tuple Resolution

Each element is resolved independently:

```kestrel
type Pair = (User, Config)
//           ^^^^  ^^^^^^
//           Both resolved in same context
```

If any element fails to resolve, the entire tuple resolution fails.

### Function Type Resolution

Parameters and return type are resolved:

```kestrel
type Handler = (Request, Context) -> Response
//              ^^^^^^^  ^^^^^^^     ^^^^^^^^
//              All resolved in same context
```

## Resolution During Binding

### Field Binding

```
bind_field(field):
    syntactic_type = field.typed_behavior.ty
    resolved = resolve_type(syntactic_type, field.context)
    if resolved:
        field.typed_behavior.set_resolved(resolved)
    else:
        emit_error(TypeNotFoundError)
```

### Function Binding

```
bind_function(func):
    // Resolve parameter types
    for param in func.parameters:
        resolved = resolve_type(param.ty, func.context)
        if resolved:
            param.ty = resolved

    // Resolve return type
    resolved_return = resolve_type(func.return_type, func.context)
    if resolved_return:
        func.return_type = resolved_return
```

### Type Alias Binding

```
bind_type_alias(alias):
    // Enter cycle detection
    cycle_detector.enter(alias.id)

    // Resolve the aliased type
    resolved = resolve_type(alias.syntactic_type, alias.context)

    // Check for cycles
    if contains_cycle(resolved, alias):
        emit_error(CircularTypeAliasError)
    else if resolved:
        alias.add_behavior(TypeAliasTypedBehavior(resolved))

    // Exit cycle detection
    cycle_detector.exit()
```

## Error Conditions

### Type Not Found

```
ERROR: TypePathResolution::NotFound
WHEN: A segment in the type path doesn't exist
EXAMPLE: let x: NonExistent.Type
```

### Ambiguous Type

```
ERROR: TypePathResolution::Ambiguous
WHEN: Multiple symbols match a segment
EXAMPLE: Multiple overloads of same name (unusual for types)
```

### Not a Type

```
ERROR: TypePathResolution::NotAType
WHEN: Path resolves to a non-type symbol (e.g., a function)
EXAMPLE: let x: someFunction  // where someFunction is a func, not a type
```

### Circular Type Alias

```
ERROR: CircularTypeAliasError
WHEN: Type alias chain forms a cycle
EXAMPLE: type A = B; type B = A
```

## Examples

### Simple Resolution

```kestrel
module App

struct User { }

func process(u: User) { }
//              ^^^^ Path(["User"])
//              Resolves to: Struct(UserSymbol)
```

### Qualified Path Resolution

```kestrel
module App

import Models

func process(u: Models.User) { }
//              ^^^^^^^^^^^ Path(["Models", "User"])
//              Phase 1: "Models" found via import
//              Phase 2: "User" found as child of Models
//              Resolves to: Struct(UserSymbol)
```

### Composite Type Resolution

```kestrel
module App

struct Request { }
struct Response { }

type Handler = (Request) -> Response
//             ^^^^^^^^^ resolve_type recursively:
//             - Function type container
//             - Param: Path(["Request"]) -> Struct(RequestSymbol)
//             - Return: Path(["Response"]) -> Struct(ResponseSymbol)
```

### Nested Type Resolution

```kestrel
module App

struct Container {
    struct Item { }
}

func get() -> Container.Item { }
//            ^^^^^^^^^^^^^^ Path(["Container", "Item"])
//            Phase 1: "Container" found in module scope
//            Phase 2: "Item" found as nested struct in Container
```

## Formal Semantics

### Type Resolution Predicate

```
resolves(T, context) = R where:

    T = Unit        → R = Unit
    T = Never       → R = Never
    T = Int(bits)   → R = Int(bits)
    T = Float(bits) → R = Float(bits)
    T = Bool        → R = Bool
    T = String      → R = String

    T = Path(segs)  → R = resolve_path(segs, context)

    T = Tuple(elems) → R = Tuple([resolves(e, context) for e in elems])
                       iff all elements resolve

    T = Function(params, ret) →
        R = Function([resolves(p, context) for p in params],
                     resolves(ret, context))
        iff all components resolve
```

### Well-Typed Predicate

```
well_typed(declaration) iff:
    all type annotations in declaration resolve successfully
```

## Source Location

- **Type resolver:** `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs`
- **Path resolution:** `lib/kestrel-semantic-model/src/queries/resolve_type_path.rs`
- **Type kinds:** `lib/kestrel-semantic-tree/src/ty/kind.rs`
