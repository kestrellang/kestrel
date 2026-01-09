# Computed Properties

Computed properties provide a way to define properties whose values are calculated rather than stored. They use getter and setter blocks to compute values on access and optionally respond to value changes.

## Syntax

### Getter-Only (Shorthand)

The most common form - a single expression that computes the value:

```kestrel
var isEmpty: Bool { self.count == 0 }
```

This is shorthand for the explicit getter form:

```kestrel
var isEmpty: Bool { get { self.count == 0 } }
```

### Getter and Setter

For read-write computed properties:

```kestrel
var celsius: Float64 {
    get { (self.fahrenheit - 32) * 5 / 9 }
    set { self.fahrenheit = newValue * 9 / 5 + 32 }
}
```

The setter receives an implicit `newValue` parameter of the property's type.

### Static Computed Properties

Type-level computed properties:

```kestrel
struct Int64 {
    public static var zero: Int64 { Int64(value: 0) }
    public static var maxValue: Int64 { Int64(value: 9223372036854775807) }
}
```

### Protocol Property Requirements

Protocols declare property requirements using `{ get }` or `{ get set }`:

```kestrel
protocol Numeric {
    static var zero: Self { get }
    static var one: Self { get }
}

protocol Container {
    var count: Int { get }
    var isEmpty: Bool { get }
}

protocol MutableContainer: Container {
    var count: Int { get set }
}
```

## Rules

### Only `var`

Computed properties must use `var`, not `let`:

```kestrel
// Valid
var isEmpty: Bool { self.count == 0 }

// Invalid - computed properties cannot use let
let isEmpty: Bool { self.count == 0 }  // ERROR
```

### Same Visibility for Getter and Setter

Unlike Swift, Kestrel does not support different visibility levels for getters and setters:

```kestrel
// Valid
public var value: Int {
    get { self._value }
    set { self._value = newValue }
}

// NOT supported - no split visibility
public private(set) var value: Int { ... }  // NOT VALID
```

### Implicit Return

Single-expression getter bodies return implicitly, consistent with function behavior:

```kestrel
// Implicit return
var doubled: Int { self.value * 2 }

// Equivalent explicit return
var doubled: Int { get { return self.value * 2 } }
```

### Setter Parameter

The setter receives an implicit `newValue` parameter. Explicit parameter naming is not supported:

```kestrel
// Valid - uses implicit newValue
set { self._value = newValue }

// NOT supported - explicit parameter name
set(value) { self._value = value }  // NOT VALID
```

## Where Allowed

Computed properties can appear in:

- **Structs** - instance and static
- **Enums** - instance and static
- **Protocols** - as requirements (`{ get }` or `{ get set }`)
- **Extensions** - adding computed properties to existing types

```kestrel
struct Point {
    var x: Int
    var y: Int

    var magnitude: Float64 { sqrt(self.x * self.x + self.y * self.y) }
}

enum Optional[T] {
    case Some(T)
    case None

    var isSome: Bool {
        match self {
            .Some(_) => true,
            .None => false
        }
    }
}

extension String {
    var isEmpty: Bool { self.count == 0 }
}
```

## Not Supported

The following features are intentionally not included:

- **Property observers** (`willSet`, `didSet`) - may be added in the future
- **Split visibility** (`public private(set)`) - getter and setter share visibility
- **Explicit setter parameter names** (`set(value)`) - always uses `newValue`
- **`let` computed properties** - use `var` with getter-only

## Examples

### Derived Values

```kestrel
struct Rectangle {
    var width: Float64
    var height: Float64

    var area: Float64 { self.width * self.height }
    var perimeter: Float64 { 2 * (self.width + self.height) }
}
```

### Type Constants

```kestrel
struct UInt8 {
    public static var zero: UInt8 { UInt8(value: 0) }
    public static var min: UInt8 { UInt8(value: 0) }
    public static var max: UInt8 { UInt8(value: 255) }
}
```

### Wrapper Properties

```kestrel
struct Temperature {
    private var kelvin: Float64

    var celsius: Float64 {
        get { self.kelvin - 273.15 }
        set { self.kelvin = newValue + 273.15 }
    }

    var fahrenheit: Float64 {
        get { self.celsius * 9 / 5 + 32 }
        set { self.celsius = (newValue - 32) * 5 / 9 }
    }
}
```

### Protocol Conformance

```kestrel
protocol Named {
    var name: String { get }
}

protocol MutableNamed: Named {
    var name: String { get set }
}

struct Person: MutableNamed {
    private var _name: String

    var name: String {
        get { self._name }
        set { self._name = newValue }
    }
}
```

### Enum Computed Properties

```kestrel
enum Result[T, E] {
    case Ok(T)
    case Err(E)

    var isOk: Bool {
        match self {
            .Ok(_) => true,
            .Err(_) => false
        }
    }

    var isErr: Bool { not self.isOk }
}
```

## Grammar

```
FieldDeclaration → Attributes? Visibility? STATIC? VAR Identifier COLON Type ComputedBody?

ComputedBody → LBRACE Expression RBRACE
             | LBRACE GetterClause SetterClause? RBRACE

GetterClause → GET CodeBlock

SetterClause → SET CodeBlock

ProtocolPropertyRequirement → Attributes? Visibility? STATIC? VAR Identifier COLON Type PropertyRequirementBody

PropertyRequirementBody → LBRACE GET RBRACE
                        | LBRACE GET SET RBRACE
```

## Implementation Notes

### Parser

The parser must distinguish between:
1. Stored property: `var x: Int` (no body, ends with semicolon or newline)
2. Computed property (shorthand): `var x: Int { expr }`
3. Computed property (explicit): `var x: Int { get { } set { } }`
4. Protocol requirement: `var x: Int { get }` or `var x: Int { get set }`

### Semantic Analysis

- Computed properties do not allocate storage
- Getter must return a value of the declared type
- Setter receives `newValue` of the declared type
- `self` is available in both getter and setter
- Setter implies the enclosing method must be `mutating` (for structs)

### Type Checking

- Getter body type must match property type
- `newValue` in setter has the property's type
- Protocol conformance: `{ get set }` requirement needs both getter and setter in implementation
