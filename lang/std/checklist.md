# Standard Library Compilation Checklist

This checklist tracks language features required for `lang/std/` to compile successfully.

## Summary

- **Total Features Needed**: 23
- **Implemented**: 6 (Associated Type Visibility, Static Methods, Protocol Inheritance, `lang.panic_unwind`, Type Cast Intrinsics, Unnamed Enum Case Parameters)
- **Low-Hanging Fruit**: 0
- **Blocked**: 17

## ✅ Recently Implemented

| Feature | Date | Notes |
|---------|------|-------|
| Associated type visibility | 2025-01-08 | Protocol associated types inherit visibility |
| `lang.panic_unwind()` | 2025-01-08 | Intrinsic that emits `Terminator::Panic` |
| Type cast intrinsics | 2025-01-08 | `lang.cast_<from>_<to>()` for all primitive conversions |
| Unnamed enum case params | 2025-01-08 | `case Some(T)` instead of `case Some(value: T)` |

## 🍎 Nearly Complete (Testing/Edge Cases)

| Feature | Status | Effort | Notes |
|---------|--------|--------|-------|
| Static methods | 95% done | Testing only | Recent fix in `3bc9ca9`, edge cases remain |
| Protocol inheritance | 85% done | 1-3 days | Parser done, semantics mostly done |

---

## Priority 1: Blocking Core Functionality

These features block the most fundamental parts of the standard library.

### 1.1 Computed Properties
**Status**: Not Implemented (Medium effort - 1-2 weeks)
**Blocking**: All numeric types, String, Array, Optional, Result, protocols

**What's needed:**
- Lexer: Add `get`, `set` keywords
- Parser: Computed property syntax `var name: Type { get { ... } set { ... } }`
- Semantic tree: Extend `FieldSymbol` or create new symbol type
- Resolution: Desugar property access to getter/setter calls
- MIR: Generate getter calls for property access

```kestrel
// Static computed property
public static var zero: Int64 { Int64(value: 0) }

// Instance computed property
public var isEmpty: Bool { self.count == 0 }

// Protocol computed property requirement
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
**Status**: ✅ Implemented (2025-01-08)

Associated types in protocols now inherit their protocol's visibility. Fixed in `lib/kestrel-semantic-analyzers/src/analyzers/visibility_consistency/mod.rs` by making the `find_less_visible_type()` function check if an associated type's parent is a public protocol and use that visibility level.

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
**Status**: Not Implemented (Large effort - 2-3 weeks)
**Blocking**: Array, Dictionary, Set, Buffer, Slice, String views

**What's needed:**
- Lexer: `subscript` keyword
- Parser: Subscript declaration syntax
- Semantic tree: New `SubscriptSymbol` type
- Resolution: Desugar `obj[index]` → subscript method call
- MIR: Already has `Place::Index` for arrays, needs protocol dispatch

**Note:** MIR already supports array indexing via `Place::Index`, but custom subscripts need method protocol dispatch.

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

### 3.2 `lang.panic_unwind()` Intrinsic
**Status**: ✅ Implemented (2025-01-08)

Implemented as `lang.panic_unwind(message: String) -> Never` intrinsic.

**Implementation:**
- Added `LangIntrinsic::PanicUnwind` to expression kinds
- Path resolution detects `lang.panic_unwind` and returns intrinsic reference
- Call resolution validates arguments and creates intrinsic call
- MIR lowering emits `Terminator::Panic(message)`
- All analyzers properly recognize it as diverging (never returns)

```kestrel
public func unwrap() -> T {
    match self {
        .Some(let value) => value,
        .None => lang.panic_unwind("called unwrap() on None")
    }
}
```

**Files affected**:
- `result/optional.ks`
- `result/result.ks`
- `collections/array.ks`

---

### 3.3 Type Casting Intrinsics ✅
**Status**: Implemented (2025-01-08)

Explicit cast intrinsics for type conversions:
```kestrel
// Format: lang.cast_<from>_<to>(value)
lang.cast_i64_i32(value)    // Int to Int32
lang.cast_i32_f64(self.value)  // Int32 to Float64
lang.cast_f64_f32(value)    // Float64 to Float32
```

**Implementation:**
- `LangPrimitive` enum in `kestrel-semantic-tree/src/expr.rs` (i8, i16, i32, i64, u8, u16, u32, u64, f32, f64)
- `LangIntrinsic::Cast { from, to }` variant for cast operations
- Path resolution in `paths.rs` parses `lang.cast_<from>_<to>` patterns
- MIR lowering emits `Rvalue::Cast` with appropriate `CastKind`

**Cast kinds supported:**
- Integer widening: `lang.cast_i8_i64`, etc. → `CastKind::IntWiden`
- Integer narrowing: `lang.cast_i64_i8`, etc. → `CastKind::IntTruncate`
- Int to float: `lang.cast_i64_f64`, etc. → `CastKind::IntToFloat`
- Float to int: `lang.cast_f64_i64`, etc. → `CastKind::FloatToInt`
- Float widening: `lang.cast_f32_f64` → `CastKind::FloatWiden`
- Float narrowing: `lang.cast_f64_f32` → `CastKind::FloatTruncate`

**Files affected**:
- All `core/*.ks` files (numeric conversions) - ✅ Updated
- `memory/pointer.ks` - uses `lang.cast_ptr[T]` (needs separate implementation)

---

### 3.4 Unnamed Enum Case Parameters ✅
**Status**: Implemented (2025-01-08)

Enum cases can now use unnamed (positional) parameters instead of requiring labels:
```kestrel
// Before (still works)
case Some(value: T)
case Ok(value: T)

// After (now also works)
case Some(T)
case Ok(T)
```

**Implementation:**
- Parser (`type_decl.rs`): `enum_case_parameter_parser()` tries named form first, falls back to unnamed
- Data structure (`data.rs`): `EnumCaseParameterData.label` and `.colon` are now `Option<Span>`
- Binder (`enum_case.rs`): Generates synthetic names (`_0`, `_1`) for unnamed params, sets `label: None`
- Pattern matching already supported `EnumPatternArgData::Unlabeled` - no changes needed

**Pattern matching syntax:**
```kestrel
match opt {
    .Some(value) => value,  // Positional binding
    .None => default,
}
```

**Files updated**:
- `result/optional.ks` - `case Some(T)` ✅
- `result/result.ks` - `case Ok(T)`, `case Err(E)` ✅
- `result/error.ks` - `case Output(Output)`, `case Early(Early)` ✅

---

### 3.6 ArcBox / Reference-Counted Box Type
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

### 3.7 Tuple Types and Access
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
**Status**: ✅ 85% Implemented - Mostly Working

**What works:**
- Parser: `protocol A: B, C { }` syntax fully supported
- Semantic: `ProtocolSymbol` supports inheritance
- Conformance checking validates inherited methods
- Method lookup traverses protocol hierarchy
- Associated type inheritance works
- Tests pass: `test_protocol_inheritance()`, `test_protocol_multiple_inheritance()`

**Potential edge cases:**
- Associated type refinement in child protocols
- Conflicting method names across multiple inherited protocols

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
**Status**: ✅ 95% Implemented

Static members on structs/enums are fully supported:
- Parser: `static func` and `static let/var` parse correctly
- Semantic: `FunctionSymbol.is_static()` and `FieldSymbol.is_static()` work
- Resolution: Static vs instance method dispatch works
- MIR lowering generates correct dispatch
- Recent fix: `3bc9ca9 fix: static method protocols`

**Remaining edge cases**: Static methods on generic types, protocol static methods

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
    case Output(T)  // Uses unnamed enum case parameters
    case Early(E)
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

### Quick Wins (Do First)
1. **Test static methods** (6.1) - 1 day, 95% done
2. **Debug protocol inheritance** (5.3) - 1-3 days, 85% done

### Phase 1: Core Type System
- Computed properties (1.1) - Medium effort
- Type parameter defaults (1.3)
- `lang.*` primitives (1.4)
- Import resolution (1.5)

### Phase 2: Method Features
- Generic methods in protocols (2.3)
- `self.init()` delegation (3.1)
- `as` casting (3.3)

### Phase 3: Collection Features
- Subscript declarations (2.2) - Large effort
- ArcBox type (3.4)

### Phase 4: Protocol System
- Extension conformances (2.4)
- Where clause equality (2.5)
- Operator attributes (4.1)

### Phase 5: Refinements
- Remaining conditional extensions
- Tuple patterns
- Error handling system
