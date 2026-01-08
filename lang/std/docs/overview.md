# Kestrel Standard Library

## Design Principles

The Kestrel standard library draws inspiration from both Rust and Swift, combining Rust's explicit control over memory and allocation with Swift's elegant, expressive APIs.

### Core Principles

1. **Minimal but Complete** - Include only what's essential, but ensure those essentials are thoroughly designed
2. **Explicit Over Implicit** - Make costs visible; allocation, copying, and side effects should be apparent
3. **Composable** - Small, focused protocols that combine to create powerful abstractions
4. **Zero-Cost Abstractions** - High-level APIs that compile to efficient code
5. **Allocator-Aware** - Types that allocate are parameterized by allocator, enabling custom memory strategies

### From Rust

- **Allocator Protocol** - All heap-allocating types are generic over their allocator
- **Result/Optional** - Explicit error handling without exceptions for recoverable errors
- **Ownership Semantics** - Clear ownership and borrowing rules
- **Iterator Protocol** - Lazy, composable iteration
- **Monomorphization** - Generics compile to specialized code

### From Swift

- **Clean Syntax** - Readable, expressive API design
- **Protocol Extensions** - Default implementations in protocols
- **Computed Properties** - Properties that compute their values
- **`-able` Naming** - Protocols named as adjectives: `Iterable`, `Hashable`, `Equatable`
- **Existentials** - `any Protocol` for type-erased values

---

## Module Structure

```
std/
  core/           # Primitives, fundamental protocols
  memory/         # Allocators, pointers, buffers
  collections/    # Array, Dictionary, Set, etc.
  text/           # String, Character, Unicode
  result/         # Optional, Result, Error
  iter/           # Iterator protocols and adapters
  ops/            # Operator protocols
  io/             # Basic I/O protocols
```

---

## Naming Conventions

### Types and Protocols

| Entity | Convention | Example |
|--------|------------|---------|
| Types | PascalCase | `Array`, `Optional`, `Buffer` |
| Protocols | PascalCase, `-able` suffix | `Iterable`, `Hashable`, `Equatable`, `Comparable` |
| Enum Cases | PascalCase | `Some`, `None`, `Ok`, `Err`, `Less`, `Equal`, `Greater` |
| Functions | camelCase | `unwrap(or:)`, `flatMap`, `starts(with:)` |
| Properties | camelCase | `isEmpty`, `count` |
| Constants | camelCase | `maxValue`, `minValue` |
| Modules | snake_case | `std/collections` |

### Protocol Naming Guidelines

- **Capability protocols** use `-able`: `Iterable`, `Hashable`, `Comparable`, `Equatable`, `Cloneable`, `Copyable`
- **Role protocols** use nouns: `Iterator`, `Allocator`, `Hasher`, `Error`
- **Conversion protocols** use `ExpressibleBy-`: `ExpressibleByIntLiteral`, `ExpressibleByStringLiteral`

### Enum Case Guidelines

Enum cases are PascalCase, matching Swift/Kotlin style:

```kestrel
public enum Optional[T] {
    case Some(T)
    case None
}

public enum Result[T, E] {
    case Ok(T)
    case Err(E)
}

public enum Ordering {
    case Less
    case Equal
    case Greater
}

// Usage with shorthand
let x: Optional[Int] = .Some(42)
let y: Optional[Int] = .None
```

---

## Lang Module (Compiler Intrinsics)

The `lang` module provides direct wrappers around LLVM operations. These are the lowest-level primitives that std builds upon.

### Primitive Types

```
lang.i8, lang.i16, lang.i32, lang.i64, lang.i128   // signed integers
lang.u8, lang.u16, lang.u32, lang.u64, lang.u128   // unsigned integers
lang.f32, lang.f64                                  // floats
lang.bool                                           // boolean
lang.ptr[T]                                         // raw pointer
```

### Primitive Operations

Operations follow the pattern: `lang.<type>_<operation>`

```kestrel
// Arithmetic (signed)
lang.i32_add(a, b)
lang.i32_sub(a, b)
lang.i32_mul(a, b)
lang.i32_div(a, b)
lang.i32_rem(a, b)
lang.i32_neg(a)

// Arithmetic (unsigned)
lang.u32_add(a, b)
lang.u32_sub(a, b)
lang.u32_mul(a, b)
lang.u32_div(a, b)
lang.u32_rem(a, b)

// Arithmetic (float)
lang.f64_add(a, b)
lang.f64_sub(a, b)
lang.f64_mul(a, b)
lang.f64_div(a, b)
lang.f64_neg(a)

// Comparison
lang.i32_eq(a, b)
lang.i32_ne(a, b)
lang.i32_lt(a, b)
lang.i32_le(a, b)
lang.i32_gt(a, b)
lang.i32_ge(a, b)

// Bitwise
lang.i32_and(a, b)
lang.i32_or(a, b)
lang.i32_xor(a, b)
lang.i32_not(a)
lang.i32_shl(a, b)
lang.i32_shr(a, b)      // arithmetic shift (signed)
lang.u32_shr(a, b)      // logical shift (unsigned)

// Conversions
lang.i32_to_i64(a)
lang.i64_to_i32(a)      // truncate
lang.i32_to_f64(a)
lang.f64_to_i32(a)

// Memory
lang.ptr_read[T](ptr)
lang.ptr_write[T](ptr, value)
lang.ptr_offset[T](ptr, offset)

// Allocation (raw)
lang.alloc(size, align)
lang.dealloc(ptr, size, align)
lang.realloc(ptr, old_size, new_size, align)
```

---

## Numeric Type Hierarchy

```
Numeric
├── Integer
│   ├── SignedInteger
│   │   ├── Int8
│   │   ├── Int16
│   │   ├── Int32
│   │   ├── Int64
│   │   └── Int (platform-sized)
│   └── UnsignedInteger
│       ├── UInt8
│       ├── UInt16
│       ├── UInt32
│       ├── UInt64
│       └── UInt (platform-sized)
└── FloatingPoint
    ├── Float32
    └── Float64 (aka Float)
```

### Numeric Protocols

```kestrel
public protocol Numeric: Equatable, ExpressibleByIntLiteral {
    static var zero: Self { get }
    static var one: Self { get }
}

public protocol Integer: Numeric, Comparable, Hashable {
    static var minValue: Self { get }
    static var maxValue: Self { get }
}

public protocol SignedInteger: Integer {
    func abs() -> Self
}

public protocol UnsignedInteger: Integer {}

public protocol FloatingPoint: Numeric, Comparable {
    static var infinity: Self { get }
    static var nan: Self { get }
    func isNaN() -> Bool
    func isInfinite() -> Bool
}
```

### Integer Implementation Example

```kestrel
public struct Int32: SignedInteger, Addable, Subtractable, Multipliable, Divisible {
    private var value: lang.i32

    public static var zero: Int32 { Int32(0) }
    public static var one: Int32 { Int32(1) }
    public static var minValue: Int32 { Int32(-2147483648) }
    public static var maxValue: Int32 { Int32(2147483647) }

    public func add(other: Int32) -> Int32 {
        Int32(value: lang.i32_add(self.value, other.value))
    }

    public func equals(other: Int32) -> Bool {
        lang.i32_eq(self.value, other.value)
    }

    public func abs() -> Int32 {
        if lang.i32_lt(self.value, 0) {
            Int32(value: lang.i32_neg(self.value))
        } else {
            self
        }
    }
}
```

---

## Pointer Types

### Pointer[T] - Single Element Pointer

```kestrel
public struct Pointer[T] {
    private var raw: lang.ptr[T]

    public init(to value: ref T)

    public var pointee: T { get set }

    public func read() -> T
    public func write(value: T)

    // Unsafe operations
    public func offset(by n: Int) -> Pointer[T]
    public func asRaw() -> RawPointer
}
```

### Buffer[T] - Contiguous Memory Region

```kestrel
public struct Buffer[T, A: Allocator = GlobalAllocator] {
    private var ptr: lang.ptr[T]
    private var cap: Int
    private var allocator: A

    // Allocate buffer with capacity
    public init(capacity: Int)
    public init(capacity: Int, allocator: A)

    // Create from existing pointer (non-owning view)
    public init(pointer: Pointer[T], count: Int)

    public var capacity: Int { get }
    public var pointer: Pointer[T] { get }

    // Element access via subscript
    public subscript(safe index: Int) -> Optional[T] { get set }
    public subscript(wrapping index: Int) -> T { get set }  // wraps around
    public subscript(unchecked index: Int) -> T { get set } // no bounds check

    // Bulk operations
    public func fill(with value: T)
    public func copy(from source: Buffer[T], count: Int)
    public func move(from source: Buffer[T], count: Int)

    // Resizing (only for owned buffers)
    public func resize(to newCapacity: Int)
}

// Usage
let buf = Buffer[Int](capacity: 10)
buf(safe: 0) = .Some(42)
buf(unchecked: 1) = 43
buf(wrapping: -1)  // wraps to last element
```

### RawPointer - Untyped Pointer

```kestrel
public struct RawPointer {
    private var raw: lang.ptr[lang.u8]

    public init(address: UInt)

    public func as[T]() -> Pointer[T]
    public func offset(by bytes: Int) -> RawPointer

    public var address: UInt { get }
}
```

---

## Allocator System

### The Allocator Protocol

```kestrel
public struct Layout {
    public var size: Int
    public var alignment: Int

    public init(size: Int, alignment: Int)
    public static func of[T]() -> Layout
    public static func array[T](count: Int) -> Layout
}

public protocol Allocator {
    func allocate(layout: Layout) -> RawPointer?
    func deallocate(ptr: RawPointer, layout: Layout)
    func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer?
}
```

### Global Allocator

```kestrel
// Set the global allocator (typically in main module)
public type GlobalAllocator = SystemAllocator

// Types default to GlobalAllocator
public struct String[A: Allocator = GlobalAllocator] { ... }
public struct Array[T, A: Allocator = GlobalAllocator] { ... }
public struct Buffer[T, A: Allocator = GlobalAllocator] { ... }

// Explicit allocator usage
let arena = ArenaAllocator(capacity: 1024)
let s = String[ArenaAllocator](allocator: arena)
```

### Built-in Allocators

| Allocator | Description |
|-----------|-------------|
| `SystemAllocator` | Default system malloc/free |
| `ArenaAllocator` | Bump allocation, bulk deallocation |
| `PoolAllocator[T]` | Fixed-size block allocation for type T |

---

## Literal Protocols

Types can be constructed from literals by implementing these protocols:

```kestrel
@literal
public protocol ExpressibleByBoolLiteral {
    init(boolLiteral: Bool)
}

@literal
public protocol ExpressibleByIntLiteral {
    init(intLiteral: Int)
}

@literal
public protocol ExpressibleByFloatLiteral {
    init(floatLiteral: Float)
}

@literal
public protocol ExpressibleByStringLiteral {
    init(stringLiteral: String)
}

@literal
public protocol ExpressibleByNilLiteral {
    init(nilLiteral: Nil)
}

@literal
public protocol ExpressibleByArrayLiteral {
    type Element
    init(arrayLiteral: [Element])
}

@literal
public protocol ExpressibleByDictionaryLiteral {
    type Key
    type Value
    init(dictionaryLiteral: [(Key, Value)])
}
```

---

## Operator Protocols

Operators are defined as protocols with the `// TODO: Add back 
//@operator` attribute. The operator protocols are **pure syntax** - they define what `+`, `==`, etc. mean for a type, with no constraints on return types.

Semantic protocols like `Equatable` and `Comparable` are separate, and provide default implementations of the operator protocols.

### Operator Protocol Design

```kestrel
// Raw operator protocol - no semantic meaning, flexible return type
// TODO: Add back 
//@operator(==)
public protocol Equal[Rhs = Self] {
    type Output
    func eq(other: Rhs) -> Output
}

// Semantic protocol - has meaning, returns Bool
public protocol Equatable {
    func equals(other: Self) -> Bool
}

// Equatable provides default implementation of Equal
extension Equatable: Equal[Self] {
    type Output = Bool
    func eq(other: Self) -> Bool {
        self.equals(other)
    }
}
```

This separation enables DSL use cases:

```kestrel
// SQL-like DSL where == returns an expression, not a Bool
struct Column[T] {
    var name: String
}

extension Column[T]: Equal[T] {
    type Output = SqlExpr
    func eq(other: T) -> SqlExpr {
        SqlExpr.Equal(column: self.name, value: other)
    }
}

// All comparison operators work for DSLs
extension Column[T]: Greater[T], GreaterOrEqual[T], Less[T], LessOrEqual[T] {
    type Output = SqlExpr
    func gt(other: T) -> SqlExpr { SqlExpr.GreaterThan(column: self.name, value: other) }
    func ge(other: T) -> SqlExpr { SqlExpr.GreaterOrEqual(column: self.name, value: other) }
    func lt(other: T) -> SqlExpr { SqlExpr.LessThan(column: self.name, value: other) }
    func le(other: T) -> SqlExpr { SqlExpr.LessOrEqual(column: self.name, value: other) }
}

// Usage
let query = users.where { $0.age >= 21 and $0.age < 65 }  // returns SqlExpr
```

### Arithmetic Operators

```kestrel
// TODO: Add back 
//@operator(+)
public protocol Addable[Rhs = Self] {
    type Output
    func add(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(-)
public protocol Subtractable[Rhs = Self] {
    type Output
    func subtract(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(*)
public protocol Multipliable[Rhs = Self] {
    type Output
    func multiply(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(/)
public protocol Divisible[Rhs = Self] {
    type Output
    func divide(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(%)
public protocol Modulo[Rhs = Self] {
    type Output
    func mod(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(prefix -)
public protocol Negatable {
    type Output
    func negate() -> Output
}
```

### Comparison Operators

Raw operator protocols (flexible return type):

```kestrel
// TODO: Add back 
//@operator(==)
public protocol Equal[Rhs = Self] {
    type Output
    func eq(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(!=)
public protocol NotEqual[Rhs = Self] {
    type Output
    func ne(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(<)
public protocol Less[Rhs = Self] {
    type Output
    func lt(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(<=)
public protocol LessOrEqual[Rhs = Self] {
    type Output
    func le(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(>)
public protocol Greater[Rhs = Self] {
    type Output
    func gt(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(>=)
public protocol GreaterOrEqual[Rhs = Self] {
    type Output
    func ge(other: Rhs) -> Output
}
```

Semantic protocols (return Bool, have meaning):

```kestrel
public protocol Equatable {
    func equals(other: Self) -> Bool
}

// Default operator implementations
extension Equatable: Equal[Self], NotEqual[Self] {
    type Output = Bool

    func eq(other: Self) -> Bool {
        self.equals(other)
    }

    func ne(other: Self) -> Bool {
        not self.equals(other)
    }
}

public enum Ordering {
    case Less
    case Equal
    case Greater
}

public protocol Comparable: Equatable {
    func compare(other: Self) -> Ordering
}

// Default operator implementations
extension Comparable: Less[Self], LessOrEqual[Self], Greater[Self], GreaterOrEqual[Self] {
    type Output = Bool

    func lt(other: Self) -> Bool {
        self.compare(other) == .Less
    }

    func le(other: Self) -> Bool {
        self.compare(other) != .Greater
    }

    func gt(other: Self) -> Bool {
        self.compare(other) == .Greater
    }

    func ge(other: Self) -> Bool {
        self.compare(other) != .Less
    }
}
```

### Logical Operators

Kestrel uses keyword-style logical operators for clarity:

```kestrel
// TODO: Add back 
//@operator(and)
public protocol And[Rhs = Self] {
    type Output
    func and(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(or)
public protocol Or[Rhs = Self] {
    type Output
    func or(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(not)
public protocol Not {
    type Output
    func not() -> Output
}

// Bool implements these with Output = Bool
extension Bool: And[Bool], Or[Bool], Not {
    type Output = Bool
    // ...
}

// Usage
if x > 0 and x < 100 {
    // ...
}

if not isEmpty or forceRefresh {
    // ...
}
```

### Bitwise Operators

```kestrel
// TODO: Add back 
//@operator(&)
public protocol BitwiseAnd[Rhs = Self] {
    type Output
    func bitwiseAnd(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(|)
public protocol BitwiseOr[Rhs = Self] {
    type Output
    func bitwiseOr(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(^)
public protocol BitwiseXor[Rhs = Self] {
    type Output
    func bitwiseXor(other: Rhs) -> Output
}

// TODO: Add back 
//@operator(prefix ~)
public protocol BitwiseNot {
    type Output
    func bitwiseNot() -> Output
}

// TODO: Add back 
//@operator(<<)
public protocol LeftShift[Rhs = Int] {
    type Output
    func shiftLeft(by: Rhs) -> Output
}

// TODO: Add back 
//@operator(>>)
public protocol RightShift[Rhs = Int] {
    type Output
    func shiftRight(by: Rhs) -> Output
}
```

### Compound Assignment Operators

```kestrel
// TODO: Add back 
//@operator(+=)
public protocol AddAssign[Rhs = Self] {
    func addAssign(other: Rhs)
}

// TODO: Add back 
//@operator(-=)
public protocol SubtractAssign[Rhs = Self] {
    func subtractAssign(other: Rhs)
}

// ... etc for *=, /=, %=, &=, |=, ^=, <<=, >>=

// Default implementations from base operators
extension Addable[Rhs] where Output == Self: AddAssign[Rhs] {
    func addAssign(other: Rhs) {
        self = self.add(other)
    }
}
```

---

## Subscript Syntax

The `subscript` keyword enables `object(index)` syntax for element access. Collections provide multiple subscript variants for different safety/performance tradeoffs:

```kestrel
public struct Array[T, A: Allocator = GlobalAllocator] {
    // Safe access - returns Optional, bounds checked
    public subscript(safe index: Int) -> Optional[T] {
        get { /* return element or None */ }
        set { /* set element if in bounds */ }
    }

    // Wrapping access - indices wrap around (like Python negative indexing)
    public subscript(wrapping index: Int) -> T {
        get { /* index % count, negative wraps from end */ }
        set { /* same wrapping behavior */ }
    }

    // Unchecked access - no bounds check, undefined behavior if out of bounds
    public subscript(unchecked index: Int) -> T {
        get { /* direct access */ }
        set { /* direct access */ }
    }

    // Range subscripts
    public subscript(safe range: Range[Int]) -> Optional[Slice[T]] {
        get { /* return slice or None */ }
    }
}

let arr = [1, 2, 3, 4, 5]

// Safe access (default for most code)
let x = arr(safe: 0)      // Some(1)
let y = arr(safe: 100)    // None (out of bounds)

// Wrapping access (convenient for circular buffers, Python-style)
let last = arr(wrapping: -1)   // 5 (last element)
let wrap = arr(wrapping: 7)    // 3 (7 % 5 = 2, so third element)

// Unchecked access (performance-critical inner loops)
for i in 0..<arr.count {
    process(arr(unchecked: i))  // no bounds check overhead
}
```

### Range Subscripts

```kestrel
let arr = [1, 2, 3, 4, 5]
let slice = arr(safe: 1..4)     // Some([2, 3, 4]) (exclusive end)
let slice2 = arr(safe: 1..=3)   // Some([2, 3, 4]) (inclusive end)
let bad = arr(safe: 10..20)     // None (out of bounds)
```

---

## Range Types

Ranges are constructed via operator protocols, not magic syntax:

```kestrel
// Range operator protocols
// TODO: Add back 
//@operator(..)
public protocol RangeConstructible[Rhs = Self] {
    type Output
    func rangeExclusive(to: Rhs) -> Output
}

// TODO: Add back 
//@operator(..=)
public protocol ClosedRangeConstructible[Rhs = Self] {
    type Output
    func rangeInclusive(to: Rhs) -> Output
}

// Integers implement these
extension Int: RangeConstructible[Int], ClosedRangeConstructible[Int] {
    type Output = Range[Int]  // or ClosedRange[Int]

    func rangeExclusive(to end: Int) -> Range[Int] {
        Range(start: self, end: end)
    }

    func rangeInclusive(to end: Int) -> ClosedRange[Int] {
        ClosedRange(start: self, end: end)
    }
}
```

### Range Structs

```kestrel
public struct Range[T: Comparable]: Iterable where T: Steppable {
    public var start: T
    public var end: T

    public func contains(value: T) -> Bool
    public func iter() -> RangeIterator[T]
}

public struct ClosedRange[T: Comparable]: Iterable where T: Steppable {
    public var start: T
    public var end: T

    public func contains(value: T) -> Bool
    public func iter() -> ClosedRangeIterator[T]
}

// Steppable enables iteration (integers, etc.)
public protocol Steppable {
    func successor() -> Self
    func predecessor() -> Self
}
```

### Usage

```kestrel
let exclusive = 0..10      // Range[Int], 0 to 9
let inclusive = 0..=10     // ClosedRange[Int], 0 to 10

// Iteration works because Range: Iterable
for i in 0..10 {
    print(i)
}

// Can use iterator extensions on ranges
let evens = (0..100).filter { $0 % 2 == 0 }.collect[Array]()
```

---

## Error Handling

Error handling in Kestrel is entirely protocol-driven. The `throws` syntax, `try`, and `throw` keywords are not magic - they desugar to protocol method calls.

### Error Protocol

```kestrel
public protocol Error {
    var description: String { get }
}
```

### Residual Enum

The fundamental type for early return semantics:

```kestrel
public enum Residual[Output, Early] {
    case Output(Output)  // continue with value
    case Early(Early)    // break/return early with value
}
```

This is the abstraction layer between `try` and `Result`. Any type that can express "continue or break" implements `Tryable` by returning `Residual`.

### Throws Protocols

Three protocols enable the `throws` syntax:

```kestrel
// Enables: `try expr` - extracts success or breaks early
public protocol Tryable[Output, Early] {
    func tryExtract() -> Residual[Output, Early]
}

// Enables: `throw error` - constructs from early return value
public protocol Throwable[Early] {
    static func fromEarly(value: Early) -> Self
}

// Enables: `return value` - constructs from output value
public protocol Returnable[Output] {
    static func fromOutput(value: Output) -> Self
}
```

### Result Type

```kestrel
// The @throws attribute marks this as the canonical throws type
// and specifies the default error type for `X throws` (no specific error)
@throws(defaultError: any Error)
public enum Result[T, E: Error]:
    Tryable[T, E],
    Throwable[E],
    Returnable[T]
{
    case Ok(T)
    case Err(E)

    // Tryable - enables `try`
    public func tryExtract() -> Residual[T, E] {
        match self {
            case .Ok(let v) => .Output(v)
            case .Err(let e) => .Early(e)
        }
    }

    // Throwable - enables `throw`
    public static func fromEarly(value: E) -> Result[T, E] {
        .Err(value)
    }

    // Returnable - enables `return value`
    public static func fromOutput(value: T) -> Result[T, E] {
        .Ok(value)
    }

    // Standard Result methods
    public var isOk: Bool { get }
    public var isErr: Bool { get }

    public func map[U](transform: (T) -> U) -> Result[U, E]
    public func mapErr[F](transform: (E) -> F) -> Result[T, F]
    public func flatMap[U](transform: (T) -> Result[U, E]) -> Result[U, E]
    public func unwrap() -> T  // panics if Err
    public func unwrap(or default: T) -> T
    public func unwrapErr() -> E  // panics if Ok
}
```

### Throws Syntax Desugaring

The `throws` keyword constructs a `Result` type:

```kestrel
// These are equivalent:
func parse(json: String) -> Json throws JsonError { ... }
func parse(json: String) -> Result[Json, JsonError] { ... }

// Generic error uses the @throws defaultError:
func load(path: String) -> Data throws { ... }
func load(path: String) -> Result[Data, any Error] { ... }
```

### Keyword Desugaring

Inside a function returning `R` where `R: Throwable[E] + Returnable[T]`:

```kestrel
func process() -> Output throws ProcessError {
    // try: calls Tryable.tryExtract(), matches on Residual
    let data = try load(path)
    // Desugars to:
    // let data = match load(path).tryExtract() {
    //     case .Output(let v) => v
    //     case .Early(let e) => return R.fromEarly(e)
    // }

    // throw: calls Throwable.fromEarly()
    if data.isEmpty {
        throw ProcessError.EmptyData
    }
    // Desugars to:
    // if data.isEmpty {
    //     return R.fromEarly(ProcessError.EmptyData)
    // }

    // return: calls Returnable.fromOutput()
    return transform(data)
    // Desugars to:
    // return R.fromOutput(transform(data))
}
```

### Try with Optional and Custom Errors

The `try ... or ...` syntax allows converting Optional failures to specific errors:

```kestrel
// Optional is Tryable - but what error to use?
func getUser(id: Int) -> User throws UserError {
    let maybeUser: User? = database.find(id)

    // try ... or ... specifies the error for None case
    let user = try maybeUser or UserError.NotFound(id)
    return user
}

// Desugars to:
func getUser(id: Int) -> User throws UserError {
    let maybeUser: User? = database.find(id)
    let user = match maybeUser {
        case .Some(let v) => v
        case .None => return .Err(UserError.NotFound(id))
    }
    return user
}
```

This is cleaner than requiring every error type to implement `Convertible[NoneError]`.

### Residual is Also Tryable

`Residual` itself can be `Tryable`, enabling generic control flow abstractions:

```kestrel
extension Residual[Output, Early]: Tryable[Output, Early] {
    public func tryExtract() -> Residual[Output, Early] {
        self
    }
}

// Useful for iterator methods like tryFold, tryForEach
extension Iterator {
    public func tryForEach[E](action: (Item) -> Residual[(), E]) -> Residual[(), E] {
        while let item = self.next() {
            try action(item)
        }
        .Output(())
    }

    public func tryFold[Acc, E](
        initial: Acc,
        combine: (Acc, Item) -> Residual[Acc, E]
    ) -> Residual[Acc, E] {
        var acc = initial
        while let item = self.next() {
            acc = try combine(acc, item)
        }
        .Output(acc)
    }
}
```

### Error Type Conversions

When `try` is used and error types don't match, the compiler looks for conversion:

```kestrel
func outer() -> X throws OuterError {
    // inner() returns Result[Y, InnerError]
    // OuterError must be constructible from InnerError
    let y = try inner()
}

// Conversion via protocol
public protocol Convertible[From] {
    init(from: From)
}

// Or via extension
extension OuterError: Convertible[InnerError] {
    public init(from inner: InnerError) {
        self = .Inner(inner)
    }
}
```

### try? and try! Variants

```kestrel
// try? converts to Optional, discarding early return value
let maybeJson = try? parse(data)  // Optional[Json]
// Desugars to:
// let maybeJson = match parse(data).tryExtract() {
//     case .Output(let v) => .Some(v)
//     case .Early(_) => .None
// }

// try! unwraps or panics
let json = try! parse(data)  // Json (panics on early return)
// Desugars to:
// let json = match parse(data).tryExtract() {
//     case .Output(let v) => v
//     case .Early(_) => panic("try! failed")
// }
```

---

## Optional Type

```kestrel
public enum Optional[T]: ExpressibleByNilLiteral {
    case Some(T)
    case None

    public init(nilLiteral: Nil) {
        self = .None
    }

    public var isSome: Bool { get }
    public var isNone: Bool { get }

    public func map[U](transform: (T) -> U) -> Optional[U]
    public func flatMap[U](transform: (T) -> Optional[U]) -> Optional[U]
    public func unwrap() -> T  // panics if None
    public func unwrap(or default: T) -> T

    // Filter
    public func filter(predicate: (T) -> Bool) -> Optional[T]

    // Convert to Result
    public func ok[E](or error: E) -> Result[T, E]
}

// Sugar: T? is Optional[T]
var x: Int? = 42
var y: Int? = nil
```

---

## Functor Protocol and Chaining

Kestrel provides a general `Functor` protocol that enables the `->` mapping operator and `?.` chaining syntax across many types, not just `Optional`.

### The Functor Protocol

```kestrel
public protocol Functor {
    type Inner
    func map[U](transform: (Inner) -> U) -> Self[U]
}
```

### The `->` Mapping Operator

The `->` operator applies a function to the inner value of any `Functor`:

```kestrel
// TODO: Add back 
//@operator(->)
public protocol Mappable[Rhs] {
    type Output
    func apply(transform: Rhs) -> Output
}

// Functor types get Mappable for free
extension Functor: Mappable[(Inner) -> U] {
    type Output = Self[U]
    func apply(transform: (Inner) -> U) -> Self[U] {
        self.map(transform)
    }
}
```

Usage:

```kestrel
// Optional
let name: String? = getName()
let upper = name -> uppercase          // Optional[String]
let len = name -> { $0.chars.count }   // Optional[Int]

// Array (also a Functor!)
let numbers = [1, 2, 3]
let doubled = numbers -> { $0 * 2 }    // [2, 4, 6]
let strings = numbers -> toString      // ["1", "2", "3"]

// Result
let data: Result[Data, Error] = load(path)
let parsed = data -> parse             // Result[Json, Error]
```

### The `?.` Chaining Operator

The `?.` operator chains method calls through any `Functor`, short-circuiting on "empty" values:

```kestrel
// For Optional: short-circuits on None
let user: User? = getUser(id)
let city = user?.address?.city         // Optional[String]

// For Array: maps over all elements
let users: [User] = getUsers()
let cities = users?.address?.city      // [String] - all cities

// For Result: short-circuits on Err
let data: Result[Config, Error] = loadConfig()
let port = data?.server?.port          // Result[Int, Error]
```

The `?.` operator desugars to `flatMap` for nested Functors:

```kestrel
// user?.address?.city desugars to:
user.flatMap { $0.address }.flatMap { $0.city }

// users?.address?.city desugars to:
users.flatMap { $0.address }.map { $0.city }
// But since Array.flatMap flattens, this becomes:
users.map { $0.address }.map { $0.city }  // [[String]] flattened to [String]
```

### Functor Implementations

```kestrel
extension Optional[T]: Functor {
    type Inner = T

    public func map[U](transform: (T) -> U) -> Optional[U] {
        match self {
            case .Some(let v) => .Some(transform(v))
            case .None => .None
        }
    }
}

extension Array[T]: Functor {
    type Inner = T

    public func map[U](transform: (T) -> U) -> Array[U] {
        var result = Array[U](capacity: self.count)
        for item in self {
            result.append(transform(item))
        }
        return result
    }
}

extension Result[T, E]: Functor {
    type Inner = T

    public func map[U](transform: (T) -> U) -> Result[U, E] {
        match self {
            case .Ok(let v) => .Ok(transform(v))
            case .Err(let e) => .Err(e)
        }
    }
}
```

### Combining `->` and `?.`

```kestrel
let users: [User] = getUsers()

// Get all uppercase city names
let cities = users?.address?.city -> uppercase  // [String]

// Chain of operations
let result = users
    ?.profile                    // [Profile]
    ?.settings                   // [Settings]
    ?.theme -> uppercase         // [String]
```

---

## Iterator Protocols

```kestrel
// Core iterator - produces values
public protocol Iterator {
    type Item
    func next() -> Optional[Item]
}

// Type that can produce an iterator
public protocol Iterable {
    type Item
    type Iter: Iterator where Iter.Item == Item
    func iter() -> Iter
}

// Type that can be built from an iterator
public protocol Collectable {
    type Item
    init[I: Iterator](from: I) where I.Item == Item
}
```

### Iterable Gets Iterator Extensions

Unlike Rust's `IntoIterator`, Kestrel's `Iterable` protocol provides all `Iterator` extension methods directly on the iterable type. The key insight: methods that return iterators still return iterators (lazy), while methods that consume (like `collect`, `count`, `forEach`) eagerly evaluate.

```kestrel
// Iterator extensions are available on Iterator
extension Iterator {
    public func map[U](transform: (Item) -> U) -> MapIterator[Self, U]
    public func filter(predicate: (Item) -> Bool) -> FilterIterator[Self]
    // ... etc
}

// Iterable also gets these extensions by forwarding to iter()
extension Iterable {
    public func map[U](transform: (Item) -> U) -> MapIterator[Iter, U] {
        self.iter().map(transform)
    }

    public func filter(predicate: (Item) -> Bool) -> FilterIterator[Iter] {
        self.iter().filter(predicate)
    }

    // ... all Iterator extensions forwarded
}
```

This means you can chain directly on collections:

```kestrel
let numbers = [1, 2, 3, 4, 5]

// No need to call .iter() first - just like JavaScript!
let doubled = numbers.map { $0 * 2 }.collect[Array]()
let evens = numbers.filter { $0 % 2 == 0 }.collect[Array]()

// Chaining works naturally
let result = numbers
    .filter { $0 > 2 }
    .map { $0 * 10 }
    .take(2)
    .collect[Array]()  // [30, 40]
```

**Note:** Unlike JavaScript where `array.map()` returns an array, Kestrel's `map` returns a lazy iterator. Use `.collect[Array]()` to materialize. This is explicit but intentional - it makes the allocation visible.

### Iterator Extensions

```kestrel
extension Iterator {
    public func map[U](transform: (Item) -> U) -> MapIterator[Self, U]
    public func filter(predicate: (Item) -> Bool) -> FilterIterator[Self]
    public func take(count: Int) -> TakeIterator[Self]
    public func skip(count: Int) -> SkipIterator[Self]
    public func enumerate() -> EnumerateIterator[Self]
    public func zip[Other: Iterator](with other: Other) -> ZipIterator[Self, Other]
    public func chain[Other: Iterator](other: Other) -> ChainIterator[Self, Other] where Other.Item == Item
    public func peekable() -> PeekableIterator[Self]

    public func fold[Acc](initial: Acc, combine: (Acc, Item) -> Acc) -> Acc
    public func reduce(combine: (Item, Item) -> Item) -> Optional[Item]
    public func collect[C: Collectable]() -> C where C.Item == Item
    public func count() -> Int
    public func forEach(action: (Item) -> Void)
    public func any(predicate: (Item) -> Bool) -> Bool
    public func all(predicate: (Item) -> Bool) -> Bool
    public func find(predicate: (Item) -> Bool) -> Optional[Item]
    public func position(predicate: (Item) -> Bool) -> Optional[Int]
}
```

### PeekableIterator

```kestrel
public struct PeekableIterator[I: Iterator]: Iterator {
    type Item = I.Item

    public func peek() -> Optional[Item]
    public func next() -> Optional[Item]
}
```

---

## Collections

### Array

```kestrel
public struct Array[T, A: Allocator = GlobalAllocator]:
    Iterable,
    Collectable,
    Functor,
    ExpressibleByArrayLiteral,
    Equatable where T: Equatable
{
    type Item = T
    type Inner = T

    public var count: Int { get }
    public var capacity: Int { get }
    public var isEmpty: Bool { get }

    public init()
    public init(allocator: A)
    public init(capacity: Int)
    public init(capacity: Int, allocator: A)

    // Subscript variants
    public subscript(safe index: Int) -> Optional[T] { get set }
    public subscript(wrapping index: Int) -> T { get set }
    public subscript(unchecked index: Int) -> T { get set }
    public subscript(safe range: Range[Int]) -> Optional[Slice[T]] { get }

    // Mutation
    public func append(element: T)
    public func insert(element: T, at index: Int)
    public func remove(at index: Int) -> T
    public func pop() -> Optional[T]
    public func clear()

    // Access
    public func first() -> Optional[T]
    public func last() -> Optional[T]

    // Iteration
    public func iter() -> ArrayIterator[T]

    // Functor
    public func map[U](transform: (T) -> U) -> Array[U, A]
}
```

### Dictionary

```kestrel
public struct Dictionary[K: Hashable, V, A: Allocator = GlobalAllocator]:
    Iterable,
    ExpressibleByDictionaryLiteral
{
    type Item = (K, V)

    public var count: Int { get }
    public var isEmpty: Bool { get }
    public var keys: KeysView[K, V, A] { get }
    public var values: ValuesView[K, V, A] { get }

    public init()
    public init(allocator: A)

    // Subscript access (returns Optional)
    public subscript(key: K) -> Optional[V] { get set }

    // Mutation
    public func insert(value: V, for key: K) -> Optional[V]
    public func remove(for key: K) -> Optional[V]
    public func contains(key: K) -> Bool
    public func clear()

    // Iteration
    public func iter() -> DictionaryIterator[K, V]
}
```

### Set

```kestrel
public struct Set[T: Hashable, A: Allocator = GlobalAllocator]:
    Iterable,
    Collectable
{
    type Item = T

    public var count: Int { get }
    public var isEmpty: Bool { get }

    public init()
    public init(allocator: A)

    // Mutation
    public func insert(element: T) -> Bool
    public func remove(element: T) -> Bool
    public func contains(element: T) -> Bool
    public func clear()

    // Set operations
    public func union(with other: Set[T]) -> Set[T]
    public func intersection(with other: Set[T]) -> Set[T]
    public func difference(from other: Set[T]) -> Set[T]
    public func symmetricDifference(with other: Set[T]) -> Set[T]
    public func isSubset(of other: Set[T]) -> Bool
    public func isSuperset(of other: Set[T]) -> Bool

    // Iteration
    public func iter() -> SetIterator[T]
}
```

---

## Strings and Characters

### Character Types

```kestrel
// Unicode code point (single scalar value, 1-4 bytes in UTF-8)
public struct CodePoint: Equatable, Comparable, Hashable {
    // Wraps lang.u32 (Unicode code point, 0x0000-0x10FFFF)
    public var value: UInt32 { get }

    public func isAscii() -> Bool
    public func isAlphabetic() -> Bool
    public func isNumeric() -> Bool
    public func isWhitespace() -> Bool
}

// Extended grapheme cluster (user-perceived character, may be multiple code points)
public struct Char: Equatable, Hashable {
    // e.g., "é" (1 code point) or "👨‍👩‍👧" (7 code points)
    public var codePoints: CodePointsView { get }
}

// Byte (for raw UTF-8 access)
public type Byte = UInt8
```

### String (No Direct Iteration or Indexing)

Strings require explicit views for iteration and indexing. This makes the cost and semantics explicit:

```kestrel
public struct String[A: Allocator = GlobalAllocator]:
    ExpressibleByStringLiteral,
    Addable,
    Equatable,
    Comparable,
    Hashable
{
    // Note: String is NOT Iterable - must use a view

    public var isEmpty: Bool { get }
    public var byteCount: Int { get }       // O(1) - UTF-8 byte count

    public init()
    public init(allocator: A)

    // Views for different representations
    public var bytes: BytesView { get }           // raw UTF-8 bytes
    public var codePoints: CodePointsView { get } // Unicode code points
    public var chars: CharsView { get }           // extended grapheme clusters (user-perceived characters)
    public var lines: LinesView { get }           // line iterator

    // Mutation (appending is always valid)
    public func append(string: String)

    // Search (works on bytes internally)
    public func contains(substring: String) -> Bool
    public func starts(with prefix: String) -> Bool
    public func ends(with suffix: String) -> Bool

    // Transformation (returns new string)
    public func trim() -> String
    public func trimStart() -> String
    public func trimEnd() -> String
    public func lowercase() -> String
    public func uppercase() -> String
    public func replace(pattern: String, with replacement: String) -> String

    // Splitting
    public func split(on separator: String) -> SplitIterator
}
```

### String Views

Each view provides iteration and indexing for its representation:

```kestrel
// Raw UTF-8 bytes - O(1) indexing
public struct BytesView: Iterable, Collectable {
    type Item = Byte

    public var count: Int { get }
    public subscript(safe index: Int) -> Optional[Byte] { get }
    public subscript(safe range: Range[Int]) -> Optional[BytesView] { get }

    public func iter() -> BytesIterator
}

// Unicode code points - O(1) iteration, O(n) indexing
public struct CodePointsView: Iterable {
    type Item = CodePoint

    public func iter() -> CodePointsIterator
    public subscript(safe index: CodePointIndex) -> Optional[CodePoint] { get }
}

// Extended grapheme clusters (user-perceived characters) - O(1) iteration, O(n) indexing
public struct CharsView: Iterable {
    type Item = Char

    public var count: Int { get }  // Note: O(n) to compute!

    public func iter() -> CharsIterator
    public subscript(safe index: CharIndex) -> Optional[Char] { get }
}

// Line iterator
public struct LinesView: Iterable {
    type Item = String

    public func iter() -> LinesIterator
}
```

### String Usage Examples

```kestrel
let s = "Hello, 世界! 👨‍👩‍👧"

// Byte access - fast, raw
s.byteCount                              // 26 (UTF-8 bytes)
s.bytes.iter().count()                   // 26
s.bytes(safe: 0)                         // Some(72) - 'H'

// Code point access - Unicode scalars
s.codePoints.iter().count()              // 17 code points
for cp in s.codePoints {
    print(cp.value)                      // prints code point values
}

// Char access - user-perceived characters (grapheme clusters)
s.chars.count                            // 13 chars (O(n)!)
for char in s.chars {
    print(char)                          // prints: H e l l o ,   世 界 !   👨‍👩‍👧
}

// This does NOT compile - String is not directly iterable:
// for c in s { }  // Error: String does not implement Iterable

// Must be explicit:
for c in s.chars { }       // OK - user-perceived characters
for b in s.bytes { }       // OK - raw bytes
for cp in s.codePoints { } // OK - Unicode code points
```

---

## Tuple Types

Tuples are built-in types with parenthesis syntax:

```kestrel
// Tuple type syntax
type Point = (Int, Int)
type Triple = (Int, String, Bool)

// Tuple literals
let point: (Int, Int) = (10, 20)
let triple = (1, "hello", true)

// Access by index
let x = point.0
let y = point.1
let s = triple.1  // "hello"

// Destructuring
let (a, b) = point
let (n, str, flag) = triple

// Tuples are Equatable and Comparable if all elements are
let eq = (1, 2) == (1, 2)  // true
let lt = (1, 2) < (1, 3)   // true (lexicographic)
```

---

## Hashing

```kestrel
public protocol Hasher {
    func write(bytes: Slice[UInt8])
    func finish() -> UInt64
}

public protocol Hashable: Equatable {
    func hash[H: Hasher](into: ref H)
}

// Default hasher (SipHash-1-3)
public struct DefaultHasher: Hasher {
    public init()
    public init(seed: (UInt64, UInt64))
    public func write(bytes: Slice[UInt8])
    public func finish() -> UInt64
}
```

---

## Memory Management

### Copy-on-Write (COW) + ARC

Heap-allocated collection types use copy-on-write semantics with automatic reference counting:

- **String**
- **Array**
- **Dictionary**
- **Set**

```kestrel
// Assignment is cheap - just increments reference count
var a = [1, 2, 3]
var b = a  // no copy, both reference same storage

// Copy happens lazily on mutation
b.append(element: 4)  // now b gets its own copy, a unchanged

// a is still [1, 2, 3]
// b is now [1, 2, 3, 4]
```

### How COW + ARC Works

1. **ARC (Automatic Reference Counting)**: Each heap allocation has a reference count. When a value is assigned or passed, the count increments. When it goes out of scope, the count decrements. When it reaches zero, memory is freed.

2. **COW (Copy-on-Write)**: Before any mutation, the collection checks if it's uniquely referenced (refcount == 1). If not, it copies the storage first, then mutates.

```kestrel
// Internal structure (conceptual)
public struct Array[T, A: Allocator = GlobalAllocator] {
    private var storage: ArcBox[ArrayStorage[T, A]]

    public func append(element: T) {
        // Ensure unique reference before mutation
        if not storage.isUnique() {
            storage = storage.deepClone()
        }
        storage.value.append(element)
    }
}
```

### Value vs Reference Semantics

| Type | Semantics | Copy Behavior |
|------|-----------|---------------|
| Primitives (Int, Bool, etc.) | Value | Bitwise copy |
| Structs (small) | Value | Bitwise copy |
| String, Array, Set, Dictionary | Value (COW) | Shallow copy, deep on mutation |
| Buffer, Pointer | Value | Copies the pointer, not the data |

### ArcBox[T]

Low-level reference-counted box for building COW types:

```kestrel
public struct ArcBox[T] {
    private var ptr: Pointer[ArcBoxStorage[T]]

    public init(value: T)

    public var value: ref T { get }
    public func isUnique() -> Bool
    public func deepClone() -> ArcBox[T]  // deep copy
}

struct ArcBoxStorage[T] {
    var refCount: Int  // atomic
    var value: T
}
```

---

## Copying, Cloning, and Moving

Kestrel uses a Swift-inspired ownership model with three distinct behaviors:

### Copyable (Default)

By default, all types are **trivially copyable** via bitwise copy. This is opt-out, not opt-in.

```kestrel
// Copyable is the default - no annotation needed
struct Point {
    var x: Int
    var y: Int
}

let a = Point(x: 1, y: 2)
let b = a  // implicit bitwise copy - both a and b are valid
```

### Cloneable (Explicit Deep Copy)

Types that need custom copy logic implement `Cloneable`. Unlike the original design, **`clone()` must be called explicitly** - it does not override assignment:

```kestrel
public protocol Cloneable {
    func clone() -> Self
}

// Array implements Cloneable for explicit deep copies
let a = [1, 2, 3]
let b = a          // shallow copy (COW) - cheap, implicit
let c = a.clone()  // deep copy - explicit, you see the cost
```

This makes the distinction clear:
- `=` is always cheap (bitwise copy or ARC increment)
- `.clone()` may be expensive (deep copy)

### NonCopyable (Move-Only)

Use `NonCopyable` to make a type that cannot be copied, only moved:

```kestrel
struct FileHandle: NonCopyable {
    private var fd: Int

    public init(path: String) throws FileError
    public func read(into buffer: Buffer[UInt8]) -> Int
    public func write(from buffer: Buffer[UInt8]) -> Int

    deinit {
        lang.close(fd)
    }
}

let file = try FileHandle(path: "foo.txt")
let file2 = file       // ERROR: FileHandle is NonCopyable
let file2 = consume file  // OK: ownership transferred, file is now invalid
```

### Consuming and Borrowing

```kestrel
// consume: transfers ownership, original becomes invalid
func processFile(file: consume FileHandle) {
    // file is owned here, will be dropped at end of function
}

let handle = try FileHandle(path: "data.txt")
processFile(file: consume handle)
// handle is now invalid - cannot be used

// borrow: temporary read access, original remains valid
func inspectFile(file: borrow FileHandle) {
    // can read file but not consume it
}

let handle2 = try FileHandle(path: "data.txt")
inspectFile(file: handle2)  // implicit borrow
// handle2 is still valid
```

### Destructors (deinit)

Types can define a `deinit` block that runs when the value is dropped:

```kestrel
struct Connection: NonCopyable {
    private var socket: Socket

    public init(host: String, port: Int) throws ConnectionError {
        self.socket = try Socket.connect(host, port)
    }

    deinit {
        socket.close()
    }
}

func doWork() {
    let conn = try Connection(host: "example.com", port: 80)
    // use conn...
}  // conn.deinit() called here automatically
```

### Summary

| Type | Copy Behavior | Use Case |
|------|---------------|----------|
| Default (no annotation) | Bitwise copy on `=` | Simple value types |
| COW types (Array, String, etc.) | ARC increment on `=`, deep copy on mutation | Collections |
| Implements `Cloneable` | Explicit `.clone()` for deep copy | When you need explicit deep copies |
| `NonCopyable` | Cannot copy, must `consume` to transfer | Resources, unique ownership |

---

## Default Values

```kestrel
public protocol Defaultable {
    init()
}

// Usage
func getOrDefault[T: Defaultable](opt: Optional[T]) -> T {
    match opt {
        case .Some(let v) => v
        case .None => T()
    }
}
```

---

## Visibility

The standard library uses Swift-style visibility:

| Modifier | Scope |
|----------|-------|
| `public` | Accessible everywhere |
| `internal` | Accessible within the module (default) |
| `fileprivate` | Accessible within the file |
| `private` | Accessible within the declaration |

---

## Generics and Existentials

Kestrel uses monomorphization for generics with existential types for dynamic dispatch.

### Monomorphization

Generic functions are compiled to specialized versions for each type they're used with:

```kestrel
func double[T: Numeric](value: T) -> T {
    value + value
}

// When called:
let x = double(42)        // Compiles specialized double_Int
let y = double(3.14)      // Compiles specialized double_Float64
```

### Existential Types (`any`)

Use `any Protocol` for type-erased values when you need dynamic dispatch:

```kestrel
// Existential type - can hold any Drawable
var shapes: Array[any Drawable] = []
shapes.append(Circle(radius: 5))
shapes.append(Rectangle(width: 10, height: 20))

for shape in shapes {
    shape.draw()  // dynamic dispatch
}

// vs. Generic (monomorphized)
func drawAll[T: Drawable](shapes: Array[T]) {
    for shape in shapes {
        shape.draw()  // static dispatch, inlined
    }
}
```

### When to Use Each

| Approach | Use When |
|----------|----------|
| Generics (`T: Protocol`) | Homogeneous collections, performance-critical code |
| Existentials (`any Protocol`) | Heterogeneous collections, plugin systems, dynamic dispatch |

---

## Closures

Closures in Kestrel are currently non-capturing. Capture semantics will be added in a future version.

```kestrel
// Non-capturing closure
let double = { (x: Int) -> Int in x * 2 }
let result = double(21)  // 42

// Shorthand with $0, $1, etc.
let triple = { $0 * 3 }
let nums = [1, 2, 3].map { $0 * 2 }  // [2, 4, 6]

// Trailing closure syntax
let evens = numbers.filter { $0 % 2 == 0 }
```

---

## Scope Summary

### In Scope for std

| Category | Types/Protocols |
|----------|-----------------|
| **Core** | Bool, Int/UInt variants, Float variants, Character, Byte, Nil |
| **Memory** | Allocator, Layout, Pointer, Buffer, RawPointer |
| **Wrappers** | Optional, Result, Error |
| **Collections** | Array, Dictionary, Set, String |
| **Iteration** | Iterator, Iterable, Collectable |
| **Operators** | Addable, Subtractable, Equatable, Comparable, etc. |
| **Literals** | ExpressibleBy{Bool,Int,Float,String,Nil,Array,Dictionary}Literal |
| **Hashing** | Hasher, Hashable |
| **Utilities** | Cloneable, Defaultable, NonCopyable |
| **Ranges** | Range, ClosedRange |
| **Functor** | Functor, Mappable |

### Out of Scope for std (separate packages)

- Concurrency (async/await, channels, threads)
- Networking (HTTP, sockets)
- File system operations
- Serialization (JSON, etc. - see `lang/json`)
- Regular expressions
- Date/time
- Random numbers
- Cryptography
