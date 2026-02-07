# Static Properties Design

## Overview

Static properties are type-level properties that belong to the type itself rather than instances. They provide shared state and computed values accessible via `TypeName.property` syntax.

This design covers:
- Static stored properties (`static let`, `static var`)
- Static computed properties (`static var x: T { get { } set { } }`)
- Protocol static property requirements
- Interaction with generics

## Syntax

### Static Stored Properties

```kestrel
struct Counter {
    static var count: Int64 = 0;
    static let maxCount: Int64 = 100;
}

// Access via type name
let c = Counter.count;
Counter.count = c + 1;
```

### Static Computed Properties

```kestrel
struct Temperature {
    private static var _celsius: Float64 = 0.0;

    static var fahrenheit: Float64 {
        get { _celsius * 9.0 / 5.0 + 32.0 }
        set { _celsius = (newValue - 32.0) * 5.0 / 9.0 }
    }
}

Temperature.fahrenheit = 98.6;
```

### Protocol Requirements

```kestrel
protocol Named {
    static var typeName: String { get }
    static var instanceCount: Int64 { get set }
}

struct User: Named {
    static var typeName: String { "User" }
    static var instanceCount: Int64 = 0;
}
```

### Enum Static Properties

```kestrel
enum Direction {
    case north, south, east, west

    // Static stored allowed
    static var defaultDirection: Direction = .north;

    // Instance stored NOT allowed (enums can't have stored instance fields)
    // var label: String  // ERROR

    // Instance computed allowed
    var opposite: Direction {
        match self {
            .north => .south,
            .south => .north,
            .east => .west,
            .west => .east,
        }
    }
}
```

## Semantic Behavior

### Storage Model

Static properties use **global storage namespaced to the type**:
- Each `static var`/`static let` gets a unique global memory location
- The location is identified by the mangled type name + property name
- Storage is allocated at compile time in the data segment

```
Foo.staticVar  →  global::Foo::staticVar (unique address)
Bar.staticVar  →  global::Bar::staticVar (different address)
```

### Initialization

Static stored properties use a **two-tier initialization model**:

**Tier 1: Constant initialization (compile-time)**
- For constant expressions (`static let x: Int64 = 42`)
- Value is embedded directly in `.data` section
- Zero runtime overhead

**Tier 2: Dynamic initialization (runtime)**
- For complex initializers (`static var x: Foo = Foo()`)
- Storage allocated in `.data` section (zeroed)
- Initialization code runs before `main()`
- Implemented via `__kestrel_init_statics()` function called from entry point

```
Program Start
    │
    ▼
__kestrel_init_statics()  ← initializes all static vars with complex initializers
    │
    ▼
main()
```

**Initialization order**: Declaration order within a module.

**Cross-module static references in initializers are banned**:
```kestrel
// module A
public static var a: Int64 = 1;

// module B
import A
public static var b: Int64 = A.a;  // ERROR: cannot reference static
                                    // from another module in initializer
```

This avoids the "static initialization order fiasco" where initialization order between modules is undefined.

### Access Semantics

| Access | Resolution |
|--------|------------|
| `Type.staticProp` | Direct global access |
| `instance.staticProp` | ERROR - must use type name |
| `Self.staticProp` (in method) | Resolves to concrete type's static |
| `T.staticProp` (generic bound) | Protocol witness lookup |

### Mutability

- `static let`: Immutable after initialization
- `static var`: Mutable (no thread safety guarantees in v1)

### Computed Properties

Computed properties have getter and optional setter:

```kestrel
static var prop: T {
    get { /* return T */ }
    set { /* newValue: T is in scope */ }
}
```

- **`newValue`**: Implicit parameter in setter body, type matches property type
- **Read**: Calls getter, returns result
- **Write**: Calls setter with assigned value as `newValue`
- **Read-modify-write** (`+=`, etc.): Calls getter, modifies, calls setter

### Name Resolution in Static Context

In static computed properties (and static methods), `self` is not available. Unqualified names resolve to other static members of the same type:

```kestrel
struct Foo {
    private static var _backing: Int64 = 0;

    public static var value: Int64 {
        get { _backing }           // _backing resolves to Foo._backing
        set { _backing = newValue }  // same here
    }
}
```

Resolution order in static context:
1. Local variables (including `newValue` in setter)
2. Static members of the enclosing type
3. Module-level declarations
4. Imported declarations

Note: Instance members are **not** in scope in static context.

### Generics Restriction

**Static stored properties are banned on generic types:**

```kestrel
struct Box[T] {
    static var count: Int64 = 0;  // ERROR: static stored properties not
                                   // supported in generic types

    static var typeName: String { "Box" }  // OK: computed is allowed
}
```

**Rationale**: With existentials, it becomes ambiguous which storage location to access when you only have `any Protocol`. Swift bans this for the same reason.

**Allowed on generic types**:
- Static computed properties (no storage)
- Static methods

## Protocol Conformance

### Property Requirements

Protocols can require static properties in two forms:

**Stored property requirement** (no accessors block):
```kestrel
protocol P {
    static let id: Int64      // Conformer must provide static let
    static var count: Int64   // Conformer must provide static var
}
```

**Computed property requirement** (with accessors block):
```kestrel
protocol P {
    static var name: String { get }       // Read-only requirement
    static var value: Int64 { get set }   // Read-write requirement
}
```

### Satisfaction Rules

| Protocol Requires | Can Be Satisfied By |
|-------------------|---------------------|
| `static let x: T` | `static let x: T = ...` |
| `static var x: T` | `static var x: T = ...` OR `static var x: T { get set }` |
| `static var x: T { get }` | `static let x: T = ...` OR `static var x: T = ...` OR `static var x: T { get }` OR `static var x: T { get set }` |
| `static var x: T { get set }` | `static var x: T = ...` OR `static var x: T { get set }` |

### Type Checking

Protocol conformance checks must verify:
1. Property exists with matching name and `static` modifier
2. Property has correct type (exact match required)
3. Property satisfies mutability requirement (see table above)
4. For computed requirements: property has required accessors

### Witness Table Entry

Static properties in protocols create witness table entries:
- Stored static: entry points to the global storage location
- Computed static: entry contains getter (and setter) function pointers

When accessing `T.staticProp` where `T` is a type parameter bounded by protocol:
```kestrel
func foo[T: Named]() {
    let name = T.typeName;  // Calls through witness table
}
```

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| `static` at module scope | "properties in global context are already static" |
| `let` with computed accessors | "computed properties must use 'var'" |
| Static stored in generic type | "static stored properties not supported in generic types" |
| Instance stored in enum | "enums cannot have stored fields" |
| Protocol property type mismatch | "property 'X' has wrong type for protocol 'P'" |
| Missing protocol property | "type 'T' does not implement required property 'X' from protocol 'P'" |
| Accessing static via instance | "static property 'X' must be accessed on type 'T', not an instance" |
| `self` in static context | "'self' is not available in static context" |
| Cross-module static in initializer | "cannot reference static property from another module in initializer" |

## Edge Cases

### Self in Static Context

`Self` in a static method or computed property refers to the concrete type:

```kestrel
struct Foo {
    static var instance: Self { Foo() }  // Self = Foo
}
```

### Protocol Extension Static Properties

Static computed properties can be provided via protocol extensions:

```kestrel
extension Named {
    static var description: String {
        Self.typeName + " (count: " + Self.instanceCount.toString() + ")"
    }
}
```

### Visibility

Static properties follow the same visibility rules as other members:
- `public static var` - accessible outside module
- `private static var` - only accessible within type
- Default visibility is internal (within module)

## Open Questions (Resolved)

1. **Q: Separate storage per generic instantiation?**
   A: No. Ban static stored properties on generic types entirely. Simpler and avoids existential issues.

2. **Q: How is `newValue` bound in setters?**
   A: Implicit magic parameter, automatically in scope within setter body.

3. **Q: When do static initializers run?**
   A: Eagerly at program start, before `main()`.

4. **Q: Can you access static via instance (`foo.staticProp`)?**
   A: No. Must use type name. Error for clarity (Swift allows it but it's confusing).
