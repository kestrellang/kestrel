# Types

Kestrel has a static type system with primitive types, composite types, and nominal types.

## Type Syntax

```
Type → UnitType
     | NeverType
     | PathType
     | TupleType
     | FunctionType

UnitType → LPAREN RPAREN

NeverType → BANG

PathType → Identifier (DOT Identifier)*

TupleType → LPAREN Type (COMMA Type)* COMMA? RPAREN

FunctionType → LPAREN TypeList RPAREN ARROW Type

TypeList → (Type (COMMA Type)* COMMA?)?
```

### Tokens
- `LPAREN` / `RPAREN` - Parentheses `(` `)`
- `BANG` - Exclamation mark `!`
- `DOT` - Period `.`
- `COMMA` - Comma `,`
- `ARROW` - Arrow `->`

## Type Categories

### 1. Primitive Types

Built-in types with no dependencies.

| Type | Syntax | Description |
|------|--------|-------------|
| Unit | `()` | Empty tuple, represents "no value" |
| Never | `!` | Bottom type, represents divergence (never returns) |
| Bool | `Bool` | Boolean: `true` or `false` |
| String | `String` | Text string |
| Int8 | `Int8` or `I8` | 8-bit signed integer |
| Int16 | `Int16` or `I16` | 16-bit signed integer |
| Int32 | `Int32` or `I32` | 32-bit signed integer |
| Int64 | `Int64` or `I64` | 64-bit signed integer |
| Float32 | `Float32` or `F32` | 32-bit floating point |
| Float64 | `Float64` or `F64` | 64-bit floating point |

**Note:** `Int` may alias to a default integer size (typically `Int64`), and `Float` to a default float size (typically `Float64`).

### 2. Composite Types

Types built from other types.

#### Tuple Types

Ordered, fixed-size collection of types.

```kestrel
()                  // Unit (empty tuple)
(Int)               // Single-element tuple (different from Int)
(Int, String)       // Two-element tuple
(Int, String, Bool) // Three-element tuple
(Int,)              // Single-element with trailing comma
```

**Semantics:**
- Tuples are structural types (identity based on structure, not name)
- Element access by position (0-indexed)
- `()` is the unit type, representing absence of value

#### Function Types

Types representing callable functions.

```kestrel
() -> Int                    // No params, returns Int
(Int) -> String              // One param, returns String
(Int, String) -> Bool        // Two params, returns Bool
(Int, Int) -> ()             // Returns unit (void-like)
((Int) -> Bool) -> String    // Higher-order: takes function, returns String
```

**Semantics:**
- Function types are structural
- Parameter types are positional (labels not part of type)
- Return type follows the arrow

### 3. Nominal Types

Types defined by declarations, identified by name.

| Category | Created By | Example |
|----------|-----------|---------|
| Struct | `struct` declaration | `struct MyStruct { }` |
| Struct | `struct` declaration | `struct Point { }` |
| Protocol | `protocol` declaration | `protocol Drawable { }` |
| TypeAlias | `type` declaration | `type ID = Int` |

**Path Types:**

Nominal types are referenced by path:

```kestrel
MyClass                    // Simple name
MyModule.MyClass           // Qualified name
Outer.Inner.DeepType       // Deeply nested
```

## Type Resolution States

Types exist in two states during compilation:

### 1. Unresolved (Path)

During parsing and initial build phase, type references are stored as paths:

```
TyKind::Path(vec!["A", "B", "C"])
```

This represents the syntax `A.B.C` but doesn't yet know what it refers to.

### 2. Resolved (Concrete)

After the bind phase, paths are resolved to concrete types:

```
TyKind::Class(Arc<ClassSymbol>)
TyKind::Struct(Arc<StructSymbol>)
TyKind::Protocol(Arc<ProtocolSymbol>)
TyKind::TypeAlias(Arc<TypeAliasSymbol>)
```

## Type Kind Enumeration

Internal representation of all type kinds:

```rust
enum TyKind {
    // Primitives
    Unit,                           // ()
    Never,                          // !
    Bool,                           // Bool
    String,                         // String
    Int(IntBits),                   // Int8, Int16, Int32, Int64
    Float(FloatBits),               // Float32, Float64

    // Composites
    Tuple(Vec<Ty>),                 // (T1, T2, ...)
    Function {
        params: Vec<Ty>,
        return_type: Box<Ty>
    },                              // (P1, P2) -> R

    // Unresolved
    Path(Vec<String>),              // A.B.C (before resolution)

    // Nominal (resolved)
    Class(Arc<ClassSymbol>),
    Struct(Arc<StructSymbol>),
    Protocol(Arc<ProtocolSymbol>),
    TypeAlias(Arc<TypeAliasSymbol>),
}
```

## Type Equality

### Structural Types

Tuple and function types use structural equality:

```
(Int, String) = (Int, String)     // Equal: same structure
(Int, String) ≠ (String, Int)     // Not equal: different order
(Int) -> Bool = (Int) -> Bool     // Equal: same signature
```

### Nominal Types

Nominal types use identity equality (same declaration):

```
struct A { }
struct B { }
type C = A

A = A          // Equal: same declaration
A ≠ B          // Not equal: different declarations
A ≠ C          // Not equal: C is alias, not A itself (but C resolves to A)
```

## Type Contexts

Types appear in several contexts:

### Parameter Types

```kestrel
func process(x: Int, y: String) { }
//            ^^^^     ^^^^^^ parameter types
```

### Return Types

```kestrel
func compute() -> Int { }
//                ^^^ return type
```

### Field Types

```kestrel
struct Point {
    let x: Float    // field type
    let y: Float    // field type
}
```

### Type Alias Definitions

```kestrel
type Coordinate = (Float, Float)
//                ^^^^^^^^^^^^^^ aliased type
```

## Examples

### Primitive Types

```kestrel
let b: Bool
let s: String
let i: Int32
let f: Float64
```

### Composite Types

```kestrel
// Tuples
let point: (Int, Int)
let record: (String, Int, Bool)
let nested: ((Int, Int), String)

// Function types
let transform: (Int) -> String
let predicate: (String) -> Bool
let binary: (Int, Int) -> Int
let consumer: (String) -> ()
let producer: () -> Int
```

### Nominal Types

```kestrel
// Class type
struct User { }
let user: User

// Struct type
struct Config { }
let config: Config

// Qualified path
let model: MyApp.Models.User

// Type alias
type Handler = (Request) -> Response
let handler: Handler
```

## Formal Semantics

### Type Well-Formedness

A type `T` is well-formed if:

1. **Primitive:** Always well-formed
2. **Tuple:** All element types are well-formed
3. **Function:** All parameter types and return type are well-formed
4. **Path:** Resolves to a valid type declaration

### Type Resolution

See [Type Resolution](type-resolution.md) for the complete resolution algorithm.

```
resolve(T, context) =
    match T:
        Primitive      → T
        Tuple(elems)   → Tuple(map(resolve, elems))
        Function(p, r) → Function(map(resolve, p), resolve(r))
        Path(segments) → lookup_type(segments, context)
```

### Type Representation

Each type carries a source span for error reporting:

```rust
struct Ty {
    kind: TyKind,
    span: Span,    // Source location
}
```

## Source Location

- **Type definitions:** `lib/kestrel-semantic-tree/src/ty/kind.rs`
- **Type resolver:** `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs`
- **Nominal type lookup:** `lib/kestrel-semantic-model/src/queries/resolve_type_path.rs`
