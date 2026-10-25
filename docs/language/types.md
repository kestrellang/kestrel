# Types

Kestrel has a static type system with primitive types, composite types, and user-defined nominal types.

> **Note**: Features marked as *(Future)* are planned but not yet fully implemented.

## Primitive Types

The user-facing primitive types are defined by the standard library and are always in scope — no import needed. (Internally they wrap compiler primitives in the `lang` namespace, such as `lang.i64`; you should never need to name those directly.)

### Integer Types

Integers come in signed and unsigned variants at four bit widths:

| Type | Bit Width | Range |
|------|-----------|-------|
| `Int8` | 8 bits | -128 to 127 |
| `Int16` | 16 bits | -32,768 to 32,767 |
| `Int32` | 32 bits | -2,147,483,648 to 2,147,483,647 |
| `Int64` | 64 bits | -2⁶³ to 2⁶³-1 |
| `UInt8` | 8 bits | 0 to 255 |
| `UInt16` | 16 bits | 0 to 65,535 |
| `UInt32` | 32 bits | 0 to 4,294,967,295 |
| `UInt64` | 64 bits | 0 to 2⁶⁴-1 |

`Int` is an alias for `Int64` (the platform-sized signed integer).

```kestrel
let small: Int8 = 127;
let medium: Int32 = 1000000;
let large: Int64 = 9223372036854775807;
let unsigned: UInt64 = 18446744073709551615;
let n: Int = 42;  // Int == Int64
```

**Default:** Integer literals without explicit type annotation default to `Int64`.

**Range checking:** An integer literal that doesn't fit its type is a compile error (E121) — it is never silently truncated or wrapped:

```kestrel
let a: Int8 = 200;   // error[E121]: 200 is not in the range -128...127
let b: UInt8 = -1;   // error[E121]: out of range for UInt8
```

### Floating-Point Types

| Type | Bit Width | Precision |
|------|-----------|-----------|
| `Float32` | 32 bits | Single precision |
| `Float64` | 64 bits | Double precision |

```kestrel
let single: Float32 = 3.14159;
let double: Float64 = 3.141592653589793;
```

**Default:** Float literals without explicit type annotation default to `Float64`.

### Boolean Type

```kestrel
let flag: Bool = true;
let condition: Bool = false;
```

### String and Character Types

`String` represents UTF-8 encoded text; `Char` represents a single Unicode scalar value:

```kestrel
let message: String = "Hello, world!";
let empty: String = "";
let multiline: String = "Line 1\nLine 2";
let letter: Char = 'k';
```

Strings support interpolation with `\(expr)` — see [String Interpolation](string-interpolation.md).

### Unit Type

The unit type `()` represents the absence of a meaningful value:

```kestrel
func doSomething() -> () {
    // Returns unit
}

let unitValue: () = ();
```

The unit type is used for:
- Functions that don't return a value
- Empty tuples
- Placeholder values

### Never Type

The never type `!` represents computations that never return normally:

```kestrel
func crash() -> ! {
    fatalError("error");
}

func loopForever() -> ! {
    loop { }
}
```

The never type is the **bottom type** and is assignable to any other type. It's used for:
- Functions that panic
- Infinite loops
- Early exits (break, continue, return)

```kestrel
// Never is assignable to any type
func example(condition: Bool) -> Int64 {
    if condition {
        return 42;
    } else {
        crash();  // crash() returns !, which is assignable to Int64
    }
}
```

## Tuple Types

Tuples are ordered, fixed-size collections of values with potentially different types.

### Syntax

```kestrel
// Two-element tuple
let point: (Int64, Int64) = (10, 20);

// Three-element tuple with mixed types
let record: (String, Int64, Bool) = ("Alice", 30, true);

// Nested tuples
let nested: ((Int64, Int64), String) = ((1, 2), "pair");
```

### Properties

- **Structural typing:** Two tuple types are equal if they have the same number of elements with the same types in the same order
- **Positional access:** Elements are accessed by position (`pair.0`, `pair.1`), zero-indexed

### Unit as Empty Tuple

The unit type `()` is equivalent to an empty tuple (a tuple with zero elements).

```kestrel
let unit: () = ();  // Empty tuple
```

## Array Types

Arrays are homogeneous, dynamically-sized collections. `[T]` is syntactic sugar for `Array[T]`.

### Syntax

```kestrel
// Array of integers
let numbers: [Int64] = [1, 2, 3, 4, 5];

// Empty array (type must be specified)
let empty: [String] = [];

// Nested arrays (2D array)
let matrix: [[Int64]] = [[1, 2], [3, 4]];

// Array of tuples
let points: [(Int64, Int64)] = [(0, 0), (1, 1), (2, 4)];
```

### Properties

- **Homogeneous:** All elements must have the same type
- **Dynamically sized:** Size is not part of the type
- **Type notation:** `[T]` where `T` is the element type

### Type Errors

```kestrel
// ERROR: Mixed types in array
let invalid = [1, "hello", true];  // Type error
```

## Dictionary Types

`[K: V]` is syntactic sugar for `Dictionary[K, V]`:

```kestrel
let ages: [String: Int64] = [:];        // empty dictionary
let scores: [String: Int64] = ["a": 1, "b": 2];
```

## Function Types

Function types represent callable functions with parameter and return types.

### Syntax

```kestrel
// Function taking no parameters, returning Int64
let producer: () -> Int64 = { 42 };

// Function taking one parameter
let increment: (Int64) -> Int64 = { it + 1 };

// Function taking multiple parameters
let add: (Int64, Int64) -> Int64 = { (a, b) in a + b };

// Function returning unit (void-like)
let action: (String) -> () = { (s) in println(s); };
```

### Properties

- **First-class values:** Functions can be passed as arguments and returned from other functions
- **Structural typing:** Function types are compared by their parameter types, return type, and closure kind
- **Parameter labels not part of type:** Labels are for call-site clarity, not type identity

```kestrel
// These two functions have the same type: (Int64, Int64) -> Int64
func add(a: Int64, b: Int64) -> Int64 { a + b }
func multiply(x: Int64, y: Int64) -> Int64 { x * y }
```

### Closure Kinds

A function type may carry an optional **kind** prefix — `mutating`, `consuming`, or `escaping` — which says how a closure of that type holds its captured environment:

```kestrel
let read:  (Int64) -> Int64 = { it * 2 };            // normal: read-only frame views
var bump:  mutating () -> () = { total = total + 1; }; // writes back to captures
let once:  consuming () -> Int64 = { seed };           // owns captures, called once
let saved: escaping () -> Int64 = { 7 };               // owns captures, may outlive the frame
```

The kind is part of type identity, so `Array[escaping () -> Int64]` and `Array[() -> Int64]` are different types. See [Closures](closures.md) for the full model.

## Optional Types

Optional types represent values that may or may not be present. `T?` is syntactic sugar for `Optional[T]`, an enum with cases `.Some(T)` and `.None`.

```kestrel
let maybeNumber: Int64? = .None;
let definiteNumber: Int64? = .Some(42);
let coalesced = maybeNumber ?? 0;   // null-coalescing operator
```

Doubled optionals are supported directly in type position — `T??` parses as `Optional[Optional[T]]`:

```kestrel
let nested: Int64?? = .Some(.None);  // outer Some, inner None

match nested {
    .Some(.Some(v)) => v,
    .Some(.None) => -1,
    .None => -2
}
```

## Reference Types

`&T` is a borrowed reference to a value of type `T`. References let functions and subscripts hand out views of stored data without copying it. See [References](references.md) for the full model, including `&mutating` references and escape rules.

## Pointer Types

Raw pointers are provided by the standard library's `Pointer[T]` type (`std.memory`). They bypass Kestrel's safety guarantees and are intended for FFI and low-level data structures — prefer references (`&T`) for ordinary borrowing.

```kestrel
var value: Int64 = 42;
let p = Pointer(to: value);   // pointer to a stored value
```

### Safety

Pointers bypass Kestrel's memory safety guarantees. Use with caution:
- Dereferencing invalid pointers causes undefined behavior
- Reads/writes through pointers are not lifetime-checked
- Manual memory management is required when allocating

## Type Aliases

Type aliases create alternative names for existing types.

### Syntax

```kestrel
// Simple alias
type ID = String;

// Alias for complex type
type Point = (Int64, Int64);

// Alias for function type
type Handler = (String) -> Int64;

// Using the alias
let userId: ID = "user_123";
let origin: Point = (0, 0);
```

The standard library uses this itself: `Int` is declared as `public type Int = Int64`.

### Generic Type Aliases *(Future)*

Generic type alias declarations parse, and can be referenced from other aliases:

```kestrel
type Pair[T] = (T, T);
type IntPair = Pair[Int64];
```

However, using an instantiated generic alias to annotate a value (e.g. `let p: Pair[Int64] = (1, 2);`) is not yet supported by the type checker.

### Properties

- **Transparent:** Type aliases are resolved during compilation; they don't create new types
- **Not nominal:** Aliased types are structurally equivalent to their underlying types
- **Documentation:** Primarily used for code clarity and reducing verbosity

## Nominal Types

Nominal types are user-defined types identified by their declaration name.

### Struct Types

```kestrel
struct Point {
    var x: Int64;
    var y: Int64;
}

let p: Point = Point(x: 10, y: 20);
```

### Enum Types

```kestrel
enum Color {
    case Red
    case Green
    case Blue
}

let color: Color = .Red;
```

### Protocol Types

```kestrel
protocol Drawable {
    func draw()
}

// Protocol types are used as constraints, not values
func render[T](item: T) where T: Drawable {
    item.draw();
}
```

See [Enums](enums.md) for detailed information on enumerated types.

## Generic Type Parameters

Generic type parameters allow types and functions to be parameterized over other types.

### Syntax

```kestrel
// Generic struct
struct Box[T] {
    var value: T;
}

// Generic function
func identity[T](value: T) -> T {
    value
}

// Multiple type parameters
struct Pair[A, B] {
    var first: A;
    var second: B;
}
```

### Type Arguments

Instantiate generic types by providing type arguments (or let inference fill them in):

```kestrel
let intBox: Box[Int64] = Box(value: 42);
let strBox = Box(value: "hello");        // inferred as Box[String]

let pair = Pair(first: 1, second: "one"); // inferred as Pair[Int64, String]
```

### Constraints

Generic parameters can be constrained with protocol bounds:

```kestrel
// T must conform to Comparable
struct SortedList[T] where T: Comparable {
    var items: [T];
}

// Non-copyable containers
struct Holder[T] where T: not Copyable {
    var value: T;
}
```

See [Generics](generics.md) for detailed information on generic types.

## The Self Type

`Self` is a special type that refers to the enclosing type within methods and protocol definitions.

### In Structs

```kestrel
struct Counter {
    var count: Int64;

    func incremented() -> Self {
        Self(count: self.count + 1)
    }

    static func zero() -> Self {
        Self(count: 0)
    }
}
```

### In Protocols

```kestrel
protocol Defaultable {
    static func default() -> Self
}

extend Counter: Defaultable {
    static func default() -> Self {
        Self(count: 0)
    }
}
```

### Properties

- **Type alias:** `Self` is an alias for the containing type
- **Useful for return types:** Ensures return type matches the actual type (not a parent type)
- **Cannot be used outside type context:** Only valid within structs, enums, and protocols

## Type Inference

Kestrel supports local type inference within function bodies.

### Inference from Literals

```kestrel
let x = 42;           // Inferred as Int64
let y = 3.14;         // Inferred as Float64
let s = "hello";      // Inferred as String
let b = true;         // Inferred as Bool
```

### Inference from Context

```kestrel
func process(x: Int32) { }

process(42);  // Literal 42 inferred as Int32

// Array element type inference
let numbers = [1, 2, 3];  // Inferred as [Int64]
```

### Explicit Type Annotations

Type annotations are required when inference is ambiguous or for documentation:

```kestrel
let empty: [String] = [];      // Cannot infer element type from empty array
let nothing: Int64? = .None;   // Cannot infer wrapped type from bare .None
```

### Limitations

- **No global inference:** Type signatures must be explicit for functions and struct fields
- **No bidirectional inference:** Return types must be specified for functions
- **Closures:** May require explicit parameter types depending on context

## Type Conversion

Kestrel does not perform implicit type conversions between numeric types. Conversions are explicit, via the `init(from:)` initializers each numeric type provides:

```kestrel
let x: Int64 = 42;
let y = Float64(from: x);        // Int64 → Float64
let narrowed = Int32(from: x);   // Int64 → Int32 (narrowing truncates high bits)
```

### Explicit Casting *(Future)*

```kestrel
let x: Int64 = 42;
let y: Float64 = x as Float64;  // Explicit cast (Future)
```

## Grammar

```
Type → UnitType
     | NeverType
     | TupleType
     | ArrayType
     | DictionaryType
     | FunctionType
     | ReferenceType
     | OptionalType
     | PathType

UnitType → LPAREN RPAREN

NeverType → BANG

TupleType → LPAREN Type (COMMA Type)* COMMA? RPAREN

ArrayType → LBRACKET Type RBRACKET

DictionaryType → LBRACKET Type COLON Type RBRACKET

FunctionType → LPAREN TypeList RPAREN ARROW Type

ReferenceType → AMP Type

OptionalType → Type QUESTION          // T?? parses as Optional[Optional[T]]

PathType → Identifier (DOT Identifier)* TypeArgumentList?

TypeArgumentList → LBRACKET Type (COMMA Type)* RBRACKET

TypeList → (Type (COMMA Type)* COMMA?)?
```

### Tokens

- `LPAREN` / `RPAREN` - Parentheses `(` `)`
- `LBRACKET` / `RBRACKET` - Square brackets `[` `]`
- `BANG` - Exclamation mark `!`
- `DOT` - Period `.`
- `COMMA` - Comma `,`
- `COLON` - Colon `:`
- `ARROW` - Arrow `->`
- `QUESTION` - Question mark `?`
- `AMP` - Ampersand `&`

## Examples

### Basic Types

```kestrel
// Primitives
let integer: Int64 = 42;
let unsigned: UInt32 = 7;
let floating: Float64 = 3.14;
let boolean: Bool = true;
let text: String = "hello";
let letter: Char = 'k';

// Unit and Never
let unit: () = ();
func diverges() -> ! {
    loop { }
}
```

### Composite Types

```kestrel
// Tuples
let point: (Int64, Int64) = (10, 20);
let triple: (String, Int64, Bool) = ("Alice", 30, true);

// Arrays
let numbers: [Int64] = [1, 2, 3, 4, 5];
let strings: [String] = ["hello", "world"];
let matrix: [[Int64]] = [[1, 2], [3, 4]];

// Functions
let add: (Int64, Int64) -> Int64 = { (a, b) in a + b };

// Optionals
let maybe: String? = .Some("present");
```

### User-Defined Types

```kestrel
// Struct
struct Person {
    var name: String;
    var age: Int64;
}

let person = Person(name: "Bob", age: 25);

// Enum
enum Status {
    case Active
    case Inactive
    case Pending
}

let status: Status = .Active;

// Generic types
struct Box[T] {
    var value: T;
}

let intBox = Box(value: 42);       // Box[Int64]
let strBox = Box(value: "hello");  // Box[String]
```

### Type Aliases

```kestrel
// Simple aliases
type UserID = String;
type Coordinate = (Int64, Int64);
type Callback = () -> ();

// Usage
let id: UserID = "user_123";
let pos: Coordinate = (10, 20);
```

## Type Categories Summary

| Category | Examples | Properties |
|----------|----------|------------|
| **Primitives** | `Int64`, `UInt8`, `Float64`, `Bool`, `String`, `Char`, `()`, `!` | Stdlib-defined, always in scope |
| **Composites** | `(A, B)`, `[T]`, `[K: V]`, `(A, B) -> R`, `T?`, `&T` | Constructed from other types, structural |
| **Nominal** | `struct`, `enum`, `protocol` | User-defined, identified by name |
| **Generic** | `Box[T]`, `Pair[A, B]` | Parameterized over types |
| **Special** | `Self` | Context-dependent |

## Best Practices

1. **Use type aliases** for complex or frequently-used types to improve readability
2. **Prefer explicit types** in function signatures for documentation and clarity
3. **Let inference work** for local variables within function bodies
4. **Choose appropriate bit widths** for integers and floats based on requirements
5. **Avoid raw pointers** unless interfacing with unsafe code or foreign functions — prefer references (`&T`)
6. **Use generic types** to write reusable, type-safe code
7. **Document type invariants** in comments when types alone cannot express constraints
