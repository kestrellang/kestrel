# Standard Library Compilation Checklist

This checklist tracks language features required for `lang/std/` to compile successfully.

## Summary

- **Total Features Needed**: 23
- **Implemented**: 0
- **In Progress**: 0
- **Blocked**: 23

---

## Priority 1: Blocking Core Functionality

These features block the most fundamental parts of the standard library.

### 1.1 Computed Properties
**Status**: Not Implemented
**Blocking**: All numeric types, String, Array, Optional, Result, protocols

Static and instance computed properties with getter bodies:
```kestrel
// Static computed property
public static var zero: Int64 { Int64(value: 0) }

// Instance computed property
public var isEmpty: Bool { self.count == 0 }

// Protocol computed property
var description: String { get }
```

**Files affected**:
- `core/int*.ks`, `core/uint*.ks`, `core/float*.ks`
- `text/string.ks`
- `collections/array.ks`, `collections/dictionary.ks`, `collections/set.ks`
- `result/optional.ks`, `result/result.ks`
- `memory/pointer.ks`, `memory/allocator.ks`

---

### 1.2 Associated Type Visibility in Protocols
**Status**: Not Implemented
**Blocking**: All operator protocols, Iterator, Iterable, Functor

Associated types declared with `type` in protocols are not being treated as public, causing "return type less visible than function" errors:
```kestrel
public protocol Addable[Rhs = Self] {
    type Output  // This should be visible to callers
    func add(other: Rhs) -> Output  // Error: Output is "internal"
}
```

**Files affected**:
- `ops/arithmetic.ks`, `ops/comparison.ks`, `ops/bitwise.ks`
- `ops/logical.ks`, `ops/range.ks`
- `iter/iterator.ks`

---

### 1.3 Type Parameter Default Values
**Status**: Not Implemented
**Blocking**: All operator protocols, generic collections

Default values for type parameters:
```kestrel
public protocol Addable[Rhs = Self] { }
public struct Array[T, A = GlobalAllocator] { }
```

**Files affected**:
- All `ops/*.ks` files
- `collections/array.ks`, `collections/dictionary.ks`, `collections/set.ks`
- `text/string.ks`

---

### 1.4 `lang.*` Primitive Types and Intrinsics
**Status**: Not Implemented
**Blocking**: All numeric types, pointer types, memory operations

Access to compiler primitive types and intrinsic functions:
```kestrel
// Primitive types
private var value: lang.i32
private var raw: lang.ptr[T]

// Intrinsic operations
lang.i32_add(a, b)
lang.ptr_read(ptr)
lang.memcpy(dest, src, len)
lang.alloc(size, alignment)
lang.sizeof[T]()
```

**Files affected**:
- All `core/*.ks` files
- `memory/pointer.ks`, `memory/buffer.ks`, `memory/allocator.ks`

---

### 1.5 Import Path Resolution
**Status**: Not Implemented
**Blocking**: All files with cross-module imports

Resolving imports from the standard library modules:
```kestrel
import std.ffi.(FFISafe)
import std.core.ordering.(Ordering)
```

**Files affected**:
- `core/int*.ks` (imports FFISafe)
- `core/protocols.ks` (imports Ordering)
- Many others

---

## Priority 2: Blocking Type System Features

### 2.1 `ref` Parameter Mode
**Status**: Not Implemented
**Blocking**: Hasher protocol, any mutable reference passing

Mutable reference parameters:
```kestrel
public func hash[H: Hasher](into hasher: ref H)
```

**Files affected**:
- `core/protocols.ks` (Hasher.write, hash functions)
- All types implementing Hashable

---

### 2.2 Subscript Declarations
**Status**: Not Implemented
**Blocking**: Array, Dictionary, Set, Buffer, Slice, String views

Custom subscript with labels:
```kestrel
public subscript(safe index: Int) -> Optional[T] { get }
public subscript(unchecked index: Int) -> T { get set }
public subscript(wrapping index: Int) -> T { get set }
```

**Files affected**:
- `collections/array.ks`
- `collections/dictionary.ks`
- `memory/pointer.ks` (Slice)
- `memory/buffer.ks`

---

### 2.3 Generic Methods in Protocols
**Status**: Not Implemented
**Blocking**: Hashable, Collectable, map/flatMap methods

Methods with their own type parameters declared in protocols:
```kestrel
public protocol Hashable {
    func hash[H: Hasher](into hasher: ref H)
}

public protocol Collectable {
    init[I](from iter: I) where I: Iterator, I.Item = Item
}
```

**Files affected**:
- `core/protocols.ks`
- `iter/iterator.ks`

---

### 2.4 Extension Adding Protocol Conformance
**Status**: Partially Working
**Blocking**: Default operator implementations, conditional conformances

Extensions that add protocol conformance to existing types/protocols:
```kestrel
extension Equatable: Equal[Self], NotEqual[Self] {
    type Output = Bool
    func eq(other: Self) -> Bool { self.equals(other) }
}
```

**Files affected**:
- `core/protocols.ks`
- `ops/assign.ks`

---

### 2.5 Where Clauses with Type Equality
**Status**: Partially Working
**Blocking**: Iterator extensions, conditional conformances

Where clauses using `=` or `==` for type equality:
```kestrel
extension Addable[Rhs] where Output = Self: AddAssign[Rhs] { }

public func flatMap[U, I: Iterable](transform: (Item) -> I) -> FlatMapIterator
    where I.Item == U
```

Note: Current stdlib uses both `=` and `==` inconsistently.

**Files affected**:
- `ops/assign.ks`
- `iter/iterator.ks`, `iter/extensions.ks`

---

### 2.6 Multiple Constraint Syntax in Declarations
**Status**: Not Implemented
**Blocking**: Range types, iterator adapters

Inline constraints with `+` or `and`:
```kestrel
// Current (not working)
public struct RangeIterator[T: Steppable + Comparable]: Iterator { }

// Should use where clause instead
public struct RangeIterator[T]: Iterator where T: Steppable, T: Comparable { }
```

**Files affected**:
- `ops/range.ks`
- `iter/adapters.ks`

---

## Priority 3: Blocking Standard Patterns

### 3.1 `self.init()` Delegation
**Status**: Not Implemented
**Blocking**: Convenience initializers

Calling one initializer from another:
```kestrel
public init(arrayLiteral elements: [T]) {
    self.init(capacity: elements.count)  // Delegation
    // ...
}
```

**Files affected**:
- `collections/array.ks`
- `text/string.ks`

---

### 3.2 `panic()` Builtin Function
**Status**: Not Implemented
**Blocking**: All unwrap/expect methods

Panic function for unrecoverable errors:
```kestrel
public func unwrap() -> T {
    match self {
        .Some(let value) => value,
        .None => panic("called unwrap() on None")
    }
}
```

**Files affected**:
- `result/optional.ks`
- `result/result.ks`
- `collections/array.ks`

---

### 3.3 `as` Type Casting
**Status**: Not Implemented
**Blocking**: Numeric conversions, pointer casts

Type casting expression:
```kestrel
self.value as lang.i64
lang.ptr_null() as lang.ptr[T]
```

**Files affected**:
- All `core/*.ks` files (numeric conversions)
- `memory/pointer.ks`

---

### 3.4 ArcBox / Reference-Counted Box Type
**Status**: Not Implemented
**Blocking**: All COW collections

Reference-counted box for COW semantics:
```kestrel
private var storage: ArcBox[ArrayStorage[T, A]]

// Methods needed
self.storage.isUnique()
self.storage.deepClone()
self.storage.value  // Access inner value
```

**Files affected**:
- `collections/array.ks`
- `collections/dictionary.ks`
- `collections/set.ks`
- `text/string.ks`

---

### 3.5 Tuple Types and Access
**Status**: Partially Working
**Blocking**: Hasher seed, dictionary literals

Tuple type parameters and `.0`, `.1` access:
```kestrel
public init(seed: (UInt64, UInt64)) {
    self.k0 = seed.0
    self.k1 = seed.1
}
```

**Files affected**:
- `core/protocols.ks` (DefaultHasher)
- `ops/literals.ks` (dictionary literals)
- `iter/iterator.ks` (enumerate, zip)

---

## Priority 4: Operator System

### 4.1 Operator Protocol Attributes
**Status**: Not Implemented (Design Needed)
**Blocking**: All operator desugaring

Mechanism to connect operators to protocol methods:
```kestrel
// Option 1: @operator attribute (current, commented out)
@operator(+)
public protocol Addable { }

// Option 2: @builtin attribute (planned)
@builtin(.AddOperatorProtocol)
public protocol Addable { }
```

**Files affected**:
- All `ops/*.ks` files

---

## Priority 5: Protocol/Extension Features

### 5.1 Extension on Protocols
**Status**: Unknown
**Blocking**: Iterator method forwarding

Extensions that add methods to protocols:
```kestrel
extension Iterator {
    public func map[U](transform: (Item) -> U) -> MapIterator[Self, U] { }
}
```

**Files affected**:
- `iter/extensions.ks`

---

### 5.2 Conditional Extensions with Multiple Where Clauses
**Status**: Unknown
**Blocking**: Conditional protocol conformances

Extensions with complex where clauses:
```kestrel
extension Result[T, E]: Equatable where T: Equatable, E: Equatable { }
```

**Files affected**:
- `result/result.ks`
- `result/optional.ks`
- `collections/array.ks`

---

### 5.3 Protocol Inheritance
**Status**: Partially Working
**Blocking**: Numeric hierarchy, protocol composition

Protocol extending another protocol:
```kestrel
public protocol Comparable: Equatable {
    func compare(other: Self) -> Ordering
}

public protocol SignedInteger: Integer { }
```

**Files affected**:
- `core/protocols.ks`
- `core/numeric.ks`

---

## Priority 6: Additional Features

### 6.1 Static Methods and Properties
**Status**: Partially Working
**Blocking**: Factory methods, type constants

Static members on structs/enums:
```kestrel
public static var zero: Int64 { Int64(value: 0) }
public static func some(value: T) -> Optional[T] { .Some(value) }
```

**Files affected**: Most files

---

### 6.2 Enum with Associated Values Pattern Matching
**Status**: Partially Working
**Blocking**: Optional, Result unpacking

Pattern matching on enum associated values:
```kestrel
match self {
    .Some(let value) => value,
    .None => default
}
```

**Files affected**:
- `result/optional.ks`
- `result/result.ks`
- `ops/range.ks`

---

### 6.3 Tuple Pattern Matching
**Status**: Unknown
**Blocking**: Some Result/Optional methods

Matching on tuple of enums:
```kestrel
match (self, other) {
    (.Some(let a), .Some(let b)) => a == b,
    (.None, .None) => true,
    _ => false
}
```

**Files affected**:
- `result/optional.ks`
- `result/result.ks`

---

### 6.4 Residual/Tryable Protocol System
**Status**: Not Implemented
**Blocking**: try/throw expressions

The Residual, Tryable, Throwable, Returnable protocol system for error handling:
```kestrel
public enum Residual[T, E] {
    case Output(value: T)
    case Early(error: E)
}

public protocol Tryable[T, E] {
    func tryExtract() -> Residual[T, E]
}
```

**Files affected**:
- `result/result.ks`
- `result/optional.ks`

---

### 6.5 Error Protocol and String Concatenation
**Status**: Unknown
**Blocking**: Error messages

Error protocol with description and string `+` operator:
```kestrel
panic("called unwrap() on Err: " + error.description())
```

**Files affected**:
- `result/result.ks`
- `result/error.ks`

---

## Syntax Issues in Stdlib (To Fix)

These are incorrect syntax usages in the stdlib that need to be corrected:

### Already Documented in `docs/stdlib-issues.md`:
1. Type parameter constraints (use `where` instead of `:`)
2. `~` operator (use `.bitwiseNot()`)
3. `$0` shorthand (use explicit parameters)
4. `null` as function name (use `nilPointer`)

### Fixed (2025-01-08):
1. **`Output == Self` → `Output = Self`** in `ops/assign.ks` (9 occurrences)
2. **`T: Steppable + Comparable`** → `where T: Steppable, T: Comparable` in `ops/range.ks` (2 occurrences)
3. **`ref` → `mutating`** parameters in hash functions across:
   - `core/protocols.ks`
   - `core/int8.ks`, `core/int16.ks`, `core/int32.ks`, `core/int64.ks`
   - `core/uint8.ks`, `core/uint16.ks`, `core/uint32.ks`, `core/uint64.ks`
   - `text/char.ks`, `text/string.ks`
   - `collections/array.ks`, `collections/set.ks`
4. **`hash[H: Hasher]` → `hash[H] where H: Hasher`** in all hash function declarations
5. **`I: Iterator + Cloneable`** → `I: Iterator, I: Cloneable` in `iter/adapters.ks`
6. **`T: Comparable + Cloneable`** → `T: Comparable, T: Cloneable` in `collections/array.ks`
7. **Inline constraints with `+`** in `serde/serde.ks` and `json/json.ks` (partial)

### Additional Fixes (2025-01-08):
8. **All inline constraints** in `serde/serde.ks` converted to where clauses (~50 occurrences)
9. **All `ref` → `mutating`** in function parameters in `serde/serde.ks` (~60 occurrences)
10. **All inline constraints** in `json/json.ks` converted to where clauses (~15 occurrences)
11. **All `ref` → `mutating`** in function parameters in `json/json.ks`

### Additional Fixes (2025-01-08 continued):
12. **Added `where A: Allocator`** to main collection types:
    - `collections/array.ks` - `Array[T, A]`
    - `collections/dictionary.ks` - `Dictionary[K, V, A]`
    - `collections/set.ks` - `Set[T, A]`
    - `text/string.ks` - `String[A]`

### Remaining Issues:
1. `ref` fields in struct types (e.g., `private var serializer: ref JsonSerializer`) - these are reference-typed fields, not parameters

---

## Testing Checklist

Once features are implemented, test with:

```bash
# Check all stdlib files
find lang/std -name "*.ks" -print0 | xargs -0 cargo run -- check

# Check specific module
cargo run -- check lang/std/core/int32.ks

# Run with verbose output
cargo run -- check --verbose lang/std/ops/arithmetic.ks
```

---

## Implementation Order Recommendation

1. **Phase 1**: Core type system
   - Computed properties (1.1)
   - Associated type visibility (1.2)
   - Type parameter defaults (1.3)
   - `lang.*` primitives (1.4)
   - Import resolution (1.5)

2. **Phase 2**: Method features
   - `ref` parameters (2.1)
   - Generic methods in protocols (2.3)
   - `self.init()` delegation (3.1)
   - `as` casting (3.3)

3. **Phase 3**: Collection features
   - Subscript declarations (2.2)
   - ArcBox type (3.4)
   - `panic()` builtin (3.2)

4. **Phase 4**: Protocol system
   - Extension conformances (2.4)
   - Where clause equality (2.5)
   - Operator attributes (4.1)

5. **Phase 5**: Refinements
   - Remaining conditional extensions
   - Tuple patterns
   - Error handling system
