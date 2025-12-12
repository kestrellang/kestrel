# Structs

Structs are value types that can contain fields, methods, and nested type declarations. Unlike structs are typically copied when assigned or passed.

## Syntax

```
StructDeclaration → Visibility? STRUCT Identifier LBRACE StructMember* RBRACE

StructMember → FunctionDeclaration
             | FieldDeclaration
             | ClassDeclaration
             | StructDeclaration
             | ProtocolDeclaration
```

### Tokens
- `STRUCT` - The `struct` keyword
- `LBRACE` / `RBRACE` - Curly braces `{` `}`
- `Visibility` - Optional visibility modifier

## Examples

### Empty Struct

```kestrel
struct Empty { }
```

### Data Struct

```kestrel
struct Point {
    let x: Float
    let y: Float
}

struct Rectangle {
    let origin: Point
    let width: Float
    let height: Float
}
```

### Struct with Methods

```kestrel
struct Vector {
    let x: Float
    let y: Float

    func magnitude() -> Float { }
    func normalized() -> Vector { }

    static func zero() -> Vector { }
    static func add(a: Vector, b: Vector) -> Vector { }
}
```

### Struct with Mixed Members

```kestrel
struct Config {
    // Fields
    let name: String
    var value: Int
    private var cached: Bool

    // Methods
    func isValid() -> Bool { }
    func reset() { }

    // Static members
    static let DEFAULT_VALUE: Int
    static func load(path: String) -> Config { }
}
```

### Nested Types

```kestrel
struct Container {
    struct Item {
        let id: Int
        let data: String
    }

    struct Reference {
        let target: Item
    }

    let items: (Item, Item, Item)
}
```

### Struct with Visibility

```kestrel
public struct PublicData {
    public let id: Int
    internal var state: String
    private var internal: Bool

    public func getData() -> String { }
    private func validate() -> Bool { }
}
```

## Semantic Rules

### Rule 1: No Duplicate Type Names

Within a struct, nested type names must be unique.

```
ERROR: DuplicateSymbolPass error
WHEN: Two types with the same name in the same struct
WHY: Ambiguous type reference
```

**Example (invalid):**
```kestrel
struct Container {
    struct Inner { }
    struct Inner { }    // ERROR: duplicate type 'Inner'
}
```

### Rule 2: No Duplicate Member Names (Non-Functions)

Fields must have unique names. A field cannot share a name with a function.

```
ERROR: DuplicateSymbolPass error
WHEN: Two fields with the same name, or field and function with same name
WHY: Ambiguous member reference
```

**Example (invalid):**
```kestrel
struct Bad {
    let value: Int
    var value: String    // ERROR: duplicate member 'value'
}

struct AlsoBad {
    let compute: Int
    func compute() { }   // ERROR: duplicate member 'compute'
}
```

### Rule 3: Function Overloading Allowed

Multiple methods with the same name are allowed if they have different signatures.

```kestrel
struct Math {
    func calculate(x: Int) -> Int { }
    func calculate(x: Float) -> Float { }       // OK: different type
    func calculate(x: Int, y: Int) -> Int { }   // OK: different arity
}
```

### Rule 4: Methods Must Have Bodies

All methods in a struct must have implementations.

```
ERROR: FunctionBodyPass error
WHEN: Method declared without body
WHY: Structs require concrete implementations
```

**Example (invalid):**
```kestrel
struct Service {
    func process()    // ERROR: function 'process' requires a body
}
```

### Rule 5: Static Members Allowed

Structs can have static fields and methods.

```kestrel
struct Constants {
    static let PI: Float
    static let E: Float

    static func square(x: Float) -> Float { }
}
```

## Struct vs Class

| Aspect | Struct | Struct |
|--------|--------|-------|
| Semantics | Value type | Reference type |
| Copy behavior | Copied on assignment | Reference shared |
| Members | Fields, methods, nested types | Fields, methods, nested types |
| Static members | Allowed | Allowed |
| Inheritance | Not supported | Not yet implemented |

## Struct as a Type

Structs create nominal types:

```kestrel
struct MyStruct { }

let instance: MyStruct    // MyStruct is a type
func accept(s: MyStruct) { }
```

The struct type is created during the build phase and attached via `TypedBehavior`.

## Struct Scope

Structs create a scope containing:
- Fields (instance and static)
- Methods (instance and static)
- Nested types (structs, protocols)

### Scope Hierarchy

```
Module scope
└── Struct scope
    ├── Nested struct scope
    ├── Nested struct scope
    └── Nested protocol scope
```

### Visibility Scope

For private members, the struct is the visibility scope:

```kestrel
struct Outer {
    private let internal: Int    // visibility_scope = Outer

    struct Inner {
        // Can access Outer.internal because Inner is inside Outer
    }
}
```

## Member Access

Members are accessed via dot notation:

```kestrel
let point: Point
point.x             // Instance field
point.magnitude()   // Instance method

Point.zero()        // Static method
Vector.UNIT_X       // Static field
```

## Formal Semantics

### Struct Declaration

For `struct S { members... }`:

```
Effect:
    - Creates StructSymbol with name S
    - Creates type Ty::Struct(Arc<StructSymbol>)
    - Adds TypedBehavior with struct type
    - Creates scope for S
    - Processes all members in S's scope

Scope:
    scope(S) = {
        declarations: {field names, method names, nested type names},
        parent: enclosing scope
    }
```

### Type Creation

```rust
let struct_symbol = Arc::new(StructSymbol::new(name, visibility));
let struct_type = Ty::r#struct(struct_symbol.clone(), span);
struct_symbol.add_behavior(TypedBehavior::new(struct_type, span));
```

### Member Resolution

```
resolve_member(struct, name):
    for member in struct.members:
        if member.name == name:
            if is_visible(member, access_context):
                return member
    return NotFound
```

## Symbol Structure

```rust
StructSymbol {
    name: String,
    visibility_behavior: VisibilityBehavior,
    typed_behavior: TypedBehavior,  // Contains Ty::Struct(self)
    children: Vec<Symbol>,          // Fields, methods, nested types
}
```

## Common Patterns

### Data Transfer Object

```kestrel
struct UserDTO {
    let id: Int
    let name: String
    let email: String
}
```

### Immutable Value

```kestrel
struct ImmutablePoint {
    let x: Int
    let y: Int

    func moved(dx: Int, dy: Int) -> ImmutablePoint { }
}
```

### Configuration

```kestrel
struct AppConfig {
    let debug: Bool
    let maxRetries: Int
    let timeout: Int

    static func default() -> AppConfig { }
    static func fromFile(path: String) -> AppConfig { }
}
```

### Mathematical Types

```kestrel
struct Complex {
    let real: Float
    let imag: Float

    func add(other: Complex) -> Complex { }
    func multiply(other: Complex) -> Complex { }
    func magnitude() -> Float { }

    static func fromPolar(r: Float, theta: Float) -> Complex { }
}
```

## Source Location

- **Build/lowering:** `lib/kestrel-semantic-tree-builder/src/builders/struct.rs`
- **Bind:** `lib/kestrel-semantic-tree-binder/src/binders/struct.rs`
- **Symbol:** `lib/kestrel-semantic-tree/src/symbol/struct.rs`
- **Validate (duplicates):** `lib/kestrel-semantic-analyzers/src/analyzers/duplicate_symbol/mod.rs`
- **Validate (cycles):** `lib/kestrel-semantic-analyzers/src/analyzers/struct_cycles/mod.rs`
