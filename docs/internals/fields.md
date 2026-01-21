# Fields

Fields are typed storage locations within structs. They can be immutable (`let`) or mutable (`var`).

## Syntax

```
FieldDeclaration → Visibility? STATIC? Mutability Identifier COLON Type

Mutability → LET | VAR
```

### Tokens
- `LET` - The `let` keyword (immutable)
- `VAR` - The `var` keyword (mutable)
- `STATIC` - The `static` keyword (type-level, not instance-level)
- `COLON` - The `:` character
- `Visibility` - Optional visibility modifier

## Field Forms

### Immutable Field (`let`)

```kestrel
let name: String
```

- Value set once, cannot be changed
- Must be initialized before use

### Mutable Field (`var`)

```kestrel
var count: Int
```

- Value can be changed after initialization
- Can be reassigned any number of times

### Static Field

```kestrel
static let MAX_SIZE: Int
static var instanceCount: Int
```

- Belongs to the type, not instances
- Accessed via type name: `MyClass.MAX_SIZE`

## Examples

### Basic Fields

```kestrel
struct Person {
    let name: String      // Immutable
    var age: Int          // Mutable
}
```

### Fields with Visibility

```kestrel
struct Account {
    public let id: Int
    internal var balance: Int
    private var pin: String
}
```

### Static Fields

```kestrel
struct Counter {
    static let MAX: Int
    static var count: Int

    let id: Int
    var value: Int
}
```

### Complex Field Types

```kestrel
struct Container {
    let items: (Int, Int, Int)           // Tuple type
    let transform: (Int) -> String       // Function type
    let nested: Inner                    // Struct type

    struct Inner {
        let value: Int
    }
}
```

### Mixed Members

```kestrel
struct Entity {
    // Instance fields
    let id: Int
    var name: String
    private var cached: Bool

    // Static fields
    static let DEFAULT_NAME: String
    static var nextId: Int

    // Methods
    func getName() -> String { }
}
```

## Semantic Rules

### Rule 1: Type Annotation Required

All fields must have an explicit type annotation.

```kestrel
// Valid
let x: Int

// Invalid (not supported)
let x = 42    // ERROR: type inference not supported
```

### Rule 2: No Duplicate Field Names

Within a struct, field names must be unique.

```
ERROR: DuplicateSymbolPass error
WHEN: Two fields have the same name
WHY: Ambiguous field reference
```

**Example (invalid):**
```kestrel
struct Bad {
    let x: Int
    let x: String    // ERROR: duplicate member 'x'
}
```

### Rule 3: Field Cannot Share Name with Function

A field cannot have the same name as a function in the same type.

```
ERROR: DuplicateSymbolPass error
WHEN: Field and function have the same name
WHY: Ambiguous member reference
```

**Example (invalid):**
```kestrel
struct Bad {
    let process: Int
    func process() { }    // ERROR: duplicate member 'process'
}
```

### Rule 4: Visibility Consistency

Public fields cannot expose less-visible types.

```
ERROR: VisibilityConsistencyPass error
WHEN: Public field has private/internal/fileprivate type
WHY: External code couldn't use the field's type
```

**Example (invalid):**
```kestrel
private struct Secret { }

public struct Container {
    public let data: Secret    // ERROR: public field exposes private type
}
```

### Rule 5: Static Fields Only in Types

The `static` modifier is only valid inside struct, or protocol.

```
ERROR: StaticContextPass error
WHEN: static field at module level
WHY: static only makes sense relative to an enclosing type
```

**Example (invalid):**
```kestrel
module MyApp

static let GLOBAL: Int    // ERROR: static only allowed inside struct or protocol
```

### Rule 6: Fields Not Allowed in Protocols

Protocols can only contain method declarations, not fields.

```kestrel
// Invalid
protocol HasValue {
    let value: Int    // ERROR: protocols cannot have fields
}
```

## Field Access

### Instance Fields

```kestrel
struct Point {
    let x: Int
    let y: Int
}

let p: Point
let xValue = p.x    // Access instance field
```

### Static Fields

```kestrel
struct Config {
    static let VERSION: String
    static var debug: Bool
}

let v = Config.VERSION    // Access static field
Config.debug = true       // Modify static mutable field
```

## Mutability Rules

### Immutable (`let`)

```kestrel
let x: Int = 10
x = 20    // ERROR: cannot assign to immutable field
```

### Mutable (`var`)

```kestrel
var x: Int = 10
x = 20    // OK: mutable field can be reassigned
```

### Mutability and Containers

```kestrel
struct Container {
    var items: (Int, Int)
}

var c: Container
c.items = (1, 2)    // OK: c is mutable, items is mutable
```

## Typed Behavior

Fields have a `TypedBehavior` that stores the field's type:

```rust
FieldSymbol {
    name: String,
    visibility_behavior: VisibilityBehavior,
    typed_behavior: TypedBehavior,  // Contains the field type
    // Mutability stored elsewhere (let vs var)
}
```

## Formal Semantics

### Field Declaration

For `[visibility] [static] let|var name: T`:

```
Preconditions:
    - T must be a valid, resolvable type
    - name must be unique among fields in enclosing type
    - name must not conflict with function names
    - visibility(field) ≤ visibility(T) if field is public
    - static only valid inside struct

Effect:
    - Creates FieldSymbol with name
    - Adds TypedBehavior with type T
    - Adds VisibilityBehavior
    - Records mutability (let vs var)
    - Records static-ness
```

### Type Resolution

During binding, field types are resolved:

```
bind_field(field):
    resolved_type = resolve_type(field.syntactic_type, field.context)
    field.typed_behavior.set_resolved(resolved_type)
```

### Member Resolution

```
resolve_field(type, name):
    for field in type.fields:
        if field.name == name:
            if is_visible(field, access_context):
                return field
    return NotFound
```

## Symbol Structure

```rust
FieldSymbol {
    name: String,
    visibility_behavior: VisibilityBehavior,
    typed_behavior: TypedBehavior,
    is_static: bool,
    is_mutable: bool,  // var = true, let = false
}
```

## Common Patterns

### Immutable Data

```kestrel
struct ImmutableRecord {
    let id: Int
    let name: String
    let timestamp: Int
}
```

### Mutable State

```kestrel
struct StatefulService {
    private var state: String
    private var lastUpdate: Int

    func getState() -> String { }
    func setState(new: String) { }
}
```

### Static Configuration

```kestrel
struct AppConfig {
    static let APP_NAME: String
    static let VERSION: String
    static var debugMode: Bool
    static var logLevel: Int
}
```

### Mixed Visibility

```kestrel
struct Entity {
    public let id: Int           // Readable by all
    internal var name: String    // Readable/writable within module
    private var cache: Data      // Internal use only
}
```

## Source Location

- **Build/lowering:** `lib/kestrel-semantic-tree-builder/src/builders/field.rs`
- **Bind:** `lib/kestrel-semantic-tree-binder/src/binders/field.rs`
- **Symbol:** `lib/kestrel-semantic-tree/src/symbol/field.rs`
- **Validate (duplicates):** `lib/kestrel-semantic-analyzers/src/analyzers/duplicate_symbol/mod.rs`
- **Validate (visibility):** `lib/kestrel-semantic-analyzers/src/analyzers/visibility_consistency/mod.rs`
