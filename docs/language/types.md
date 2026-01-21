# Types

Kestrel has a static type system with primitive types, composite types, and user-defined nominal types.

> **Note**: Features marked as *(Future)* are planned but not yet fully implemented.

## Primitive Types

Kestrel provides built-in primitive types for common data representations.

### Integer Types

Integers are signed and come in multiple bit widths:

| Type | Internal Name | Bit Width | Range |
|------|---------------|-----------|-------|
| `lang.i8` | I8 | 8 bits | -128 to 127 |
| `lang.i16` | I16 | 16 bits | -32,768 to 32,767 |
| `lang.i32` | I32 | 32 bits | -2,147,483,648 to 2,147,483,647 |
| `lang.i64` | I64 | 64 bits | -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807 |

```kestrel
let small: lang.i8 = 127;
let medium: lang.i32 = 1000000;
let large: lang.i64 = 9223372036854775807;
```

**Default:** Integer literals without explicit type annotation default to `lang.i64`.

### Floating-Point Types

Floating-point types represent real numbers:

| Type | Internal Name | Bit Width | Precision |
|------|---------------|-----------|-----------|
| `lang.f16` | F16 | 16 bits | Half precision |
| `lang.f32` | F32 | 32 bits | Single precision |
| `lang.f64` | F64 | 64 bits | Double precision |

```kestrel
let half: lang.f16 = 3.14;
let single: lang.f32 = 3.14159;
let double: lang.f64 = 3.141592653589793;
```

**Default:** Float literals without explicit type annotation default to `lang.f64`.

### Boolean Type

The boolean type represents truth values:

```kestrel
let flag: lang.i1 = true;
let condition: lang.i1 = false;
```

**Internal Name:** `lang.i1` (1-bit integer internally)

### String Type

Strings represent UTF-8 encoded text:

```kestrel
let message: lang.str = "Hello, world!";
let empty: lang.str = "";
let multiline: lang.str = "Line 1\nLine 2";
```

### Unit Type

The unit type `()` represents the absence of a meaningful value:

```kestrel
func doSomething() -> () {
    // Returns unit
}

let unit_value: () = ();
```

The unit type is used for:
- Functions that don't return a value
- Empty tuples
- Placeholder values

### Never Type

The never type `!` represents computations that never return normally:

```kestrel
func panic() -> ! {
    lang.panic_unwind("error");
}

func loop_forever() -> ! {
    loop { }
}
```

The never type is the **bottom type** and is assignable to any other type. It's used for:
- Functions that panic
- Infinite loops
- Early returns (break, continue, return)

```kestrel
// Never is assignable to any type
func example() -> lang.i64 {
    if condition {
        return 42;
    } else {
        panic();  // panic() returns !, which is assignable to lang.i64
    }
}
```

## Tuple Types

Tuples are ordered, fixed-size collections of values with potentially different types.

### Syntax

```kestrel
// Two-element tuple
let point: (lang.i64, lang.i64) = (10, 20);

// Three-element tuple with mixed types
let record: (lang.str, lang.i64, lang.i1) = ("Alice", 30, true);

// Single-element tuple (requires trailing comma)
let single: (lang.i64,) = (42,);

// Nested tuples
let nested: ((lang.i64, lang.i64), lang.str) = ((1, 2), "pair");
```

### Properties

- **Structural typing:** Two tuple types are equal if they have the same number of elements with the same types in the same order
- **Immutable by default:** Elements are accessed by position but cannot be modified unless wrapped in a mutable container
- **Zero-indexed:** First element is at position 0

### Unit as Empty Tuple

The unit type `()` is equivalent to an empty tuple (a tuple with zero elements).

```kestrel
let unit: () = ();  // Empty tuple
```

## Array Types

Arrays are homogeneous, dynamically-sized collections of elements.

### Syntax

```kestrel
// Array of integers
let numbers: [lang.i64] = [1, 2, 3, 4, 5];

// Empty array (type must be specified)
let empty: [lang.str] = [];

// Nested arrays (2D array)
let matrix: [[lang.i64]] = [[1, 2], [3, 4]];

// Array of tuples
let points: [(lang.i64, lang.i64)] = [(0, 0), (1, 1), (2, 4)];
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

## Function Types

Function types represent callable functions with parameter and return types.

### Syntax

```kestrel
// Function taking no parameters, returning lang.i64
let producer: () -> lang.i64;

// Function taking one parameter
let increment: (lang.i64) -> lang.i64;

// Function taking multiple parameters
let add: (lang.i64, lang.i64) -> lang.i64;

// Function returning unit (void-like)
let action: (lang.str) -> ();

// Higher-order function (function taking and returning functions)
let transform: ((lang.i64) -> lang.i64) -> (lang.i64) -> lang.i64;
```

### Properties

- **First-class values:** Functions can be passed as arguments and returned from other functions
- **Structural typing:** Function types are compared by their parameter types and return type
- **Parameter labels not part of type:** Labels are for call-site clarity, not type identity

```kestrel
// These two functions have the same type: (lang.i64, lang.i64) -> lang.i64
func add(a: lang.i64, b: lang.i64) -> lang.i64 { a + b }
func multiply(x: lang.i64, y: lang.i64) -> lang.i64 { x * y }
```

## Optional Types *(Future)*

Optional types represent values that may or may not be present.

```kestrel
let maybeNumber: lang.i64? = null;
let definiteNumber: lang.i64? = 42;
```

**Note:** Optional type syntax (`T?`) is planned but not yet implemented. Current null handling is limited.

## Pointer Types

Pointer types represent raw memory addresses. They are unsafe and should be used sparingly.

### Syntax

```kestrel
struct Wrapper {
    var ptr: lang.ptr[lang.i64];
}

func allocate() -> lang.ptr[lang.i64] {
    lang.ptr_null[lang.i64]();
}

// Nested pointers (pointer to pointer)
func double_indirection() -> lang.ptr[lang.ptr[lang.i64]] {
    lang.ptr_null[lang.ptr[lang.i64]]();
}

// Pointer to tuple
func tuple_ptr() -> lang.ptr[(lang.i64, lang.i1)] {
    lang.ptr_null[(lang.i64, lang.i1)]();
}
```

### Built-in Pointer Operations

Kestrel provides intrinsic functions for working with pointers:

| Function | Description |
|----------|-------------|
| `lang.ptr_null[T]()` | Create null pointer of type `T` |
| `lang.ptr_read(ptr)` | Read value from pointer |
| `lang.ptr_write(ptr, value)` | Write value to pointer |
| `lang.ptr_is_null(ptr)` | Check if pointer is null |
| `lang.ptr_cast[From, To](ptr)` | Cast pointer from one type to another |

### Safety

Pointers bypass Kestrel's memory safety guarantees. Use with caution:
- Dereferencing null or invalid pointers causes undefined behavior
- Type casting can lead to type confusion
- Manual memory management is required

## Type Aliases

Type aliases create alternative names for existing types.

### Syntax

```kestrel
// Simple alias
type ID = lang.str;

// Alias for complex type
type Point = (lang.i64, lang.i64);

// Alias for function type
type Handler = (lang.str) -> lang.i64;

// Using the alias
let user_id: ID = "user_123";
let origin: Point = (0, 0);
```

### Generic Type Aliases

Type aliases can be generic:

```kestrel
type Pair[T] = (T, T);
type Result[T, E] = (T, E);  // Simplified result type

// Using generic aliases
let numbers: Pair[lang.i64] = (1, 2);
let strings: Pair[lang.str] = ("hello", "world");
```

### Properties

- **Transparent:** Type aliases are resolved during compilation; they don't create new types
- **Not nominal:** Aliased types are structurally equivalent to their underlying types
- **Documentation:** Primarily used for code clarity and reducing verbosity

## Nominal Types

Nominal types are user-defined types identified by their declaration name.

### Struct Types

```kestrel
struct Point {
    var x: lang.i64;
    var y: lang.i64;
}

let p: Point = Point { x: 10, y: 20 };
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
    func draw() -> ();
}

// Protocol types are used as constraints, not values
func render[T: Drawable](item: T) {
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

Instantiate generic types by providing type arguments:

```kestrel
let int_box: Box[lang.i64] = Box { value: 42 };
let str_box: Box[lang.str] = Box { value: "hello" };

let pair: Pair[lang.i64, lang.str] = Pair { first: 1, second: "one" };
```

### Constraints

Generic parameters can be constrained with protocol bounds:

```kestrel
// T must conform to Comparable
struct SortedList[T] where T: Comparable[T] {
    var items: [T];
}

// Multiple bounds
struct Container[T] where T: Copyable and Hashable {
    var value: T;
}
```

See [Generics](generics.md) for detailed information on generic types.

## The Self Type

`Self` is a special type that refers to the enclosing type within methods and protocol definitions.

### In Structs

```kestrel
struct Counter {
    var count: lang.i64;

    func increment() -> Self {
        Self { count: self.count + 1 }
    }

    static func zero() -> Self {
        Self { count: 0 }
    }
}
```

### In Protocols

```kestrel
protocol Cloneable {
    func clone() -> Self;
}

extension Counter: Cloneable {
    func clone() -> Self {
        Self { count: self.count }
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
let x = 42;           // Inferred as lang.i64
let y = 3.14;         // Inferred as lang.f64
let s = "hello";      // Inferred as lang.str
let b = true;         // Inferred as lang.i1
```

### Inference from Context

```kestrel
func process(x: lang.i64) { }

process(42);  // Literal 42 inferred as lang.i64

// Array element type inference
let numbers = [1, 2, 3];  // Inferred as [lang.i64]
```

### Explicit Type Annotations

Type annotations are required when inference is ambiguous or for documentation:

```kestrel
let empty: [lang.str] = [];  // Cannot infer element type from empty array
let nullable: lang.i64? = null;  // Cannot infer wrapped type from null
```

### Limitations

- **No global inference:** Type signatures must be explicit for functions and struct fields
- **No bidirectional inference:** Return types must be specified for functions
- **Closures:** May require explicit parameter types depending on context

## Type Conversion

Kestrel does not perform implicit type conversions. All conversions must be explicit.

### Explicit Casting *(Future)*

```kestrel
let x: lang.i64 = 42;
let y: lang.f64 = x as lang.f64;  // Explicit cast (Future)
```

### Integer/Float Conversions

Currently, use intrinsic functions for conversions:

```kestrel
let i: lang.i64 = 42;
// Use lang intrinsics for conversion (implementation-specific)
```

## Grammar

```
Type → UnitType
     | NeverType
     | TupleType
     | ArrayType
     | FunctionType
     | PointerType
     | OptionalType
     | PathType

UnitType → LPAREN RPAREN

NeverType → BANG

TupleType → LPAREN Type (COMMA Type)* COMMA? RPAREN

ArrayType → LBRACKET Type RBRACKET

FunctionType → LPAREN TypeList RPAREN ARROW Type

PointerType → PathType LBRACKET Type RBRACKET
            | PATH "lang.ptr" LBRACKET Type RBRACKET

OptionalType → Type QUESTION

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
- `ARROW` - Arrow `->`
- `QUESTION` - Question mark `?` *(Future)*

## Examples

### Basic Types

```kestrel
// Primitives
let integer: lang.i64 = 42;
let floating: lang.f64 = 3.14;
let boolean: lang.i1 = true;
let text: lang.str = "hello";

// Unit and Never
let unit: () = ();
func diverges() -> ! {
    loop { }
}
```

### Composite Types

```kestrel
// Tuples
let point: (lang.i64, lang.i64) = (10, 20);
let triple: (lang.str, lang.i64, lang.i1) = ("Alice", 30, true);

// Arrays
let numbers: [lang.i64] = [1, 2, 3, 4, 5];
let strings: [lang.str] = ["hello", "world"];
let matrix: [[lang.i64]] = [[1, 2], [3, 4]];

// Functions
let add: (lang.i64, lang.i64) -> lang.i64 = { (a, b) in a + b };
let predicate: (lang.str) -> lang.i1;
```

### User-Defined Types

```kestrel
// Struct
struct Person {
    var name: lang.str;
    var age: lang.i64;
}

let person: Person = Person { name: "Bob", age: 25 };

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

let int_box: Box[lang.i64] = Box { value: 42 };
let str_box: Box[lang.str] = Box { value: "hello" };
```

### Type Aliases

```kestrel
// Simple aliases
type UserID = lang.str;
type Coordinate = (lang.i64, lang.i64);
type Callback = () -> ();

// Generic aliases
type Pair[T] = (T, T);
type Transform[In, Out] = (In) -> Out;

// Usage
let id: UserID = "user_123";
let pos: Coordinate = (10, 20);
let nums: Pair[lang.i64] = (1, 2);
```

### Complex Nested Types

```kestrel
// Array of function types
let handlers: [(lang.str) -> ()] = [
    { (msg) in lang.print(msg) },
    { (msg) in lang.log(msg) }
];

// Tuple of arrays
let data: ([lang.i64], [lang.str]) = ([1, 2, 3], ["a", "b"]);

// Function returning function
let makeAdder: (lang.i64) -> (lang.i64) -> lang.i64 = { (x) in
    { (y) in x + y }
};
```

## Type Categories Summary

| Category | Examples | Properties |
|----------|----------|------------|
| **Primitives** | `lang.i64`, `lang.f64`, `lang.i1`, `lang.str`, `()`, `!` | Built-in, fixed representation |
| **Composites** | `(A, B)`, `[T]`, `(A, B) -> R` | Constructed from other types, structural |
| **Nominal** | `struct`, `enum`, `protocol` | User-defined, identified by name |
| **Generic** | `Box[T]`, `Pair[A, B]` | Parameterized over types |
| **Special** | `Self`, `_` (infer) | Context-dependent |

## Best Practices

1. **Use type aliases** for complex or frequently-used types to improve readability
2. **Prefer explicit types** in function signatures for documentation and clarity
3. **Let inference work** for local variables within function bodies
4. **Choose appropriate bit widths** for integers and floats based on requirements
5. **Avoid raw pointers** unless interfacing with unsafe code or foreign functions
6. **Use generic types** to write reusable, type-safe code
7. **Document type invariants** in comments when types alone cannot express constraints
