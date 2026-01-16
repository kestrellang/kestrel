# std2 Implementation Issues

Tracking issues encountered during std2 migration and their workarounds.

---

## Phase 6: Integer Types

### Trailing comma before brace causes parse error

**Error:** `found 'LBrace' expected something else`

**Cause:** A trailing comma after the last protocol conformance before the opening brace:
```kestrel
public struct Int8:
    Convertible[Int16],
    Convertible[Int32],  // <- trailing comma here
{
```

**Fix:** Remove trailing comma before `{`. Fixed in `num/generate.py`.

---

## Phase 4: Core Protocols

### Name conflict between protocol and enum case

**Error:** `'Equal' is not a type`

**Cause:** In `ordering.ks`, the enum case `case Equal` conflicts with the protocol `Equal[Self]` when both are in scope.

**Fix:** Use import aliasing:
```kestrel
import std.core.(Equal as EqualOp, NotEqual as NotEqualOp)

public enum Ordering: EqualOp[Self], NotEqualOp[Self] {
    case Equal  // no conflict now
}
```

---

## Phase 9: Pointer Types

### Type aliases don't work for member access

**Error:** `cannot find type 'Int' in this scope`

**Cause:** Type aliases like `public type Int = Int64` cannot be used to access members or call constructors through the alias name.

**Fix:** Use the concrete type directly (`Int64`, `UInt64`) instead of aliases (`Int`, `UInt`) in implementation files.

---

### Type inference fails with untyped lang intrinsics

**Error:** `could not infer type for 1 placeholder(s)`

**Cause:** `lang.ptr_null()` returns an untyped pointer, and wrapping it in `lang.cast_ptr[T]()` doesn't help inference.

**Fix:** Use typed intrinsics directly:
```kestrel
// Instead of:
lang.cast_ptr[T](lang.ptr_null())

// Use:
lang.ptr_null[T]()
```

Same for `lang.ptr_offset`:
```kestrel
// Instead of:
lang.cast_ptr[T](lang.ptr_offset(ptr, bytes))

// Use:
lang.ptr_offset[T](ptr, bytes)
```

---

## Phase 10: Iterator Protocol

### Generic init with where clause not supported

**Error:** `cannot find type 'I' in this scope`

**Cause:** Generic initializers with where clauses in protocols may not be fully supported:
```kestrel
public protocol Collectable {
    type Item
    init[I](from iter: I) where I: Iterator, I.Item = Item
}
```

**Workaround:** Commented out `Collectable` protocol for now.

---

## Phase 11: Optional Type

### Computed properties in enums parsed as free functions

**Error:** `cannot use 'self' in free function`

**Cause:** Multiline computed properties in enums are being misinterpreted by the parser:
```kestrel
public enum Optional[T] {
    public var isSome: Bool {
        match self {  // <- error: 'self' in free function
            .Some(_) => true,
            .None => false
        }
    }
}
```

**Workaround:** Convert computed properties to functions:
```kestrel
public func isSome() -> Bool {
    match self {
        .Some(_) => true,
        .None => false
    }
}
```

---

### Circular import between Iterator and Optional

**Cause:** `Iterator.next()` returns `Optional[Item]`, but `OptionalIterator` needs to conform to `Iterator`.

**Workaround:**
- `iter/iterator.ks` imports `Optional`
- `result/optional.ks` does NOT import `Iterator`
- `OptionalIterator` is defined without `Iterator` conformance in `optional.ks`
- Iterator conformance can be added via extension in a separate file if needed

---

## Phase 14: Buffer and ArcBox

### Type alias doesn't expose methods

**Error:** `cannot access member on type 'GlobalAllocator'`

**Cause:** Type aliases like `public type GlobalAllocator = SystemAllocator` don't allow method calls through the alias:
```kestrel
var allocator: GlobalAllocator = GlobalAllocator();
allocator.allocate(layout)  // Error: member access not supported
```

**Workaround:** Use the concrete type directly:
```kestrel
var allocator: SystemAllocator = SystemAllocator();
allocator.allocate(layout)  // Works
```

---

### `get` is a reserved keyword

**Error:** `Parse error: found 'Get' expected something else`

**Cause:** `get` is a keyword (for computed property getters), so it cannot be used as a method name.

**Fix:** Rename the method (e.g., `getValue()` instead of `get()`).

---

### Computed property `.raw` not accessible on local variables

**Error:** `member not found: 'raw' on type 'Int64'` or `undefined name 'raw'`

**Cause:** Single-line computed properties like `public var raw: lang.i64 { self.value }` work when accessed on struct fields (`layout.size.raw`) but fail when accessed on local variables or expressions:
```kestrel
let byteCount: Int64 = copyCount * elementSize;
memcpy(..., byteCount.raw);  // Error: member not found
```

**Workaround:** Comment out affected code or restructure to avoid accessing `.raw` on local variables.

---

### Match in init doesn't prove field initialization

**Error:** `initializer does not initialize all fields: 'ptr'`

**Cause:** When using match expressions where one branch panics, the compiler doesn't recognize that fields are always initialized:
```kestrel
public init(value: T) {
    match allocator.allocate(layout) {
        .Some(rawPtr) => {
            self.ptr = rawPtr.cast[T]();  // Compiler doesn't see this
        },
        .None => lang.panic("failed")
    }
}
```

**Workaround:** Use if/else with Optional methods instead:
```kestrel
public init(value: T) {
    let result = allocator.allocate(layout);
    if result.isSome() {
        self.ptr = result.unwrap().cast[T]();
    } else {
        lang.panic("failed")
    }
}
```

---

### Labels not used at method call sites

**Error:** `no method 'write' with 1 argument(s) and labels (value:)`

**Cause:** Method parameters have labels in the definition, but labels are not used at call sites:
```kestrel
// Definition
public func write(value: T) { ... }

// Call site - wrong:
ptr.write(value: x)

// Call site - correct:
ptr.write(x)
```

**Fix:** Remove labels from method call sites.

---

## Phase 15: Literal Slice

### Comparison of lang types returns lang.i1, not Bool

**Error:** `type mismatch: expected 'Bool', found 'lang.i1'`

**Cause:** Comparing `lang.i64` values with `==` returns `lang.i1`, not `Bool`:
```kestrel
public func isEmpty() -> Bool { self.len == 0 }  // Error
```

**Fix:** Wrap the comparison in `Bool(boolLiteral: ...)`:
```kestrel
public func isEmpty() -> Bool { Bool(boolLiteral: self.len == 0) }
```

---

### Iterator type must be defined before Iterable type

**Error:** `type 'LiteralSliceIterator' does not satisfy bound` / `does not conform to required protocol 'Iterator'`

**Cause:** When a struct declares `type Iter = SomeIterator[T]` for Iterable conformance, the iterator type must be defined before the struct that references it.

**Fix:** Define iterator structs before the types that use them in `type Iter = ...`.

---

## Phase 16: Literals Protocol

### Child protocol cannot redeclare parent's associated type

**Error:** `conflicting associated type 'Element' from inherited protocols`

**Cause:** When a protocol inherits from another protocol, it cannot redeclare the same associated type:
```kestrel
public protocol _ExpressibleByArrayLiteral {
    type Element  // defined here
}

public protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
    type Element  // Error: cannot redeclare
}
```

**Fix:** Remove the duplicate associated type declaration from the child protocol - it's inherited automatically.

---

## Phase 17: Range Types

### Steppable is in std.num, not std.core

**Error:** `symbol 'Steppable' not found in module 'std.core'`

**Cause:** `Steppable` protocol is defined in `std.num.numeric`, not `std.core`.

**Fix:** Import from correct module:
```kestrel
import std.num.(Steppable)
```

---

## Phase 18: Iterator Adapters

### `while true` with unreachable code after loop causes issues

**Error:** `undefined name 'None'` (on unreachable `.None` after infinite loop)

**Cause:** Code after `while true { ... return ... }` is unreachable, but the compiler still tries to parse/check it:
```kestrel
while true {
    if condition { return .Some(x) }
    if done { return .None }
}
.None  // unreachable - causes error
```

**Workaround:** Use a `done` flag pattern instead:
```kestrel
var done: Bool = false;
var result: Optional[T] = .None;
while done == false {
    // ... set result and done = true when finished
}
result
```

---

### `not` operator fails on Bool variables

**Error:** `member not found: 'logicalNot' on type 'lang.i1'`

**Cause:** Using `not` on a Bool variable doesn't work properly:
```kestrel
var done: Bool = false;
while not done { ... }  // Error
```

**Workaround:** Use explicit comparison instead:
```kestrel
while done == false { ... }
```

---

### Inline tuple in .Some() fails type inference

**Error:** `undefined name 'Some'`

**Cause:** Creating a tuple inline inside `.Some()` fails type inference:
```kestrel
.Some((a.unwrap(), b.unwrap()))  // Error
```

**Workaround:** Assign the tuple to a variable first:
```kestrel
let pair = (a.unwrap(), b.unwrap());
.Some(pair)  // Works
```

---

## General Notes

### Blanket protocol extensions for operators

To avoid having every type manually implement `Less`, `Greater`, `LessOrEqual`, `GreaterOrEqual`, `NotEqual` when they already implement `Comparable`, blanket extensions were added to `core/protocols.ks`:

```kestrel
extend Equatable: Equal[Self] {
    type Equal.Output = Bool
}

extend Comparable: Less[Self], LessOrEqual[Self], Greater[Self], GreaterOrEqual[Self], NotEqual[Self] {
    type Less.Output = Bool
    // ... etc

    public func lessThan(other: Self) -> Bool {
        self.compare(other) == Ordering.Less
    }
    // ... etc
}
```

This means types only need to implement `Equatable` (with `equals()`) or `Comparable` (with `compare()`) to get all the operator protocols automatically.

---

## Phase 19: Array

### Static generic methods on types cause type parameter issues

**Error:** `type mismatch: expected 'T', found 'T'` / `expected 'Pointer[T]', found 'Pointer[T]'`

**Cause:** Calling static methods with explicit type parameters like `Pointer.nilPointer[T]()` confuses the compiler when `T` is already a type parameter in the current scope.

**Fix:** Use the underlying `lang` intrinsic directly:
```kestrel
// Instead of:
self.ptr = Pointer.nilPointer[T]();

// Use:
self.ptr = Pointer(raw: lang.ptr_null[T]());
```

---

### Extension blocks cannot access private members

**Error:** `member 'len' is private and not accessible from this scope`

**Cause:** `extend` blocks adding protocol conformance cannot access private fields of the type they extend.

**Fix:** Use public getter methods instead of private fields:
```kestrel
// Instead of:
if self.len != other.len { ... }

// Use:
let selfCount = self.count();
let otherCount = other.count();
if selfCount != otherCount { ... }
```

---

### COW (Copy-on-Write) too complex for current compiler state

**Workaround:** Simplified Array implementation to not use ArcBox for COW semantics. Array directly owns its memory and uses SystemAllocator. Generic allocator parameter removed for simplicity.

---

### Generic allocator parameter conflicts with ExpressibleByArrayLiteral

**Error:** `type mismatch: expected 'A', found 'SystemAllocator'`

**Cause:** When `Array[T, A]` is generic over allocator `A`, the `ExpressibleByArrayLiteral` init must use a concrete allocator (`SystemAllocator`), but this doesn't match the generic `A`.

**Workaround:** Removed generic allocator parameter. Array is now `Array[T]` and always uses `SystemAllocator`.

---

### Subscript syntax not used

**Cause:** Subscript syntax like `array(unchecked: i)` requires subscript definitions which add complexity.

**Workaround:** Use explicit methods instead:
```kestrel
// Instead of:
array(unchecked: i)
array(unchecked: i) = value

// Use:
array.getUnchecked(i)
array.setUnchecked(i, value)
```

---

### `for-in` loops not fully working

**Cause:** `for element in collection` syntax may not be fully implemented.

**Workaround:** Use manual iteration with while loop and flag:
```kestrel
// Instead of:
for element in elements {
    self.append(element)
}

// Use:
var iter = elements.iter();
var done: Bool = false;
while done == false {
    let item = iter.next();
    if item.isSome() {
        self.append(item.unwrap())
    } else {
        done = true
    }
}
```

---

### Collectable protocol not supported

**Cause:** Generic initializer with where clause not supported:
```kestrel
public init[I](from iter: I) where I: Iterator, I.Item = T
```

**Workaround:** Removed `Collectable` conformance from Array.

---

### Functor/map method removed

**Cause:** Would require creating `Array[U, A]` with different element type, complex with current limitations.

**Workaround:** Removed `Functor` conformance and `map` method. Can be added later or implemented externally.

---

### Type alias `Int` cannot be used for member access

**Error:** `cannot find type 'Int' in this scope`

**Cause:** Type aliases like `public type Int = Int64` cannot be used to access members or construct instances.

**Workaround:** Use `Int64` explicitly throughout std2 instead of `Int` alias.

---

## Phase 21: String

### Subscript parameters not bound in getter/setter body

**Error:** `undefined name 'index'`

**Cause:** Subscript parameter names are not accessible inside the getter/setter body:
```kestrel
public subscript(safe index: Int64) -> Optional[T] {
    get {
        if index >= Int64(intLiteral: 0) { ... }  // Error: undefined name 'index'
    }
}
```

**Workaround:** Commented out subscripts. Use explicit methods like `getValue(at:)` and `setUnchecked(index:, value:)` instead.

---

### Protocol extension default implementations not inherited

**Error:** `'Array' conforms to 'ExpressibleByArrayLiteral' but not its parent protocol '_ExpressibleByArrayLiteral'`

**Cause:** When a protocol extends another protocol and provides a default implementation via `extend`, types conforming to the child protocol don't automatically get the default implementation:
```kestrel
public protocol _ExpressibleByArrayLiteral {
    init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64)
}

public protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
    init(arrayLiteral: LiteralSlice[Element])
}

extend ExpressibleByArrayLiteral {
    // This default implementation is NOT applied to conforming types
    public init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64) { ... }
}
```

**Workaround:** Explicitly conform to both protocols and implement both initializers:
```kestrel
public struct Array[T]: ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral {
    public init(_arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
    }
    public init(arrayLiteral elements: LiteralSlice[T]) { ... }
}
```

---

### `UInt8.raw` returns `lang.i8`, not `lang.i32`

**Error:** `type mismatch: expected 'I32', found 'I8'`

**Cause:** The `.raw` property on `UInt8` returns `lang.i8`, but many bit operations expect `lang.i32`:
```kestrel
let byte: UInt8 = ...;
let v: lang.i32 = byte.raw;  // Error: expected I32, found I8
```

**Fix:** Use explicit cast:
```kestrel
let v: lang.i32 = lang.cast_i8_i32(byte.raw);
```

---

### CLI `build` command only accepts single file

**Error:** `module 'X' not found` when passing multiple files to `build`

**Cause:** The `build` command accepts `file: String` not `files: Vec<String>`. When multiple files are passed, only the first is compiled.

**Workaround:** Use `check` command for multi-file compilation, or ensure all imports are within the single file being built.

---

### Enum case name conflicts with protocol name

**Error:** `undefined name 'Equal'` when using `Ordering.Equal`

**Cause:** The `Equal` protocol from `std.core.comparison` conflicts with `Ordering.Equal` enum case when both are imported:
```kestrel
import std.core.(Ordering)  // Has case Equal
// std.core also has protocol Equal[Self]

Ordering.Equal  // Error: confused with protocol Equal
```

**Workaround:** Use enum shorthand syntax where the type is known:
```kestrel
public func compare(other: Self) -> Ordering {
    .Equal  // Works because return type is Ordering
}

// Or use intermediate variable:
let eql: Ordering = .Equal;
if cmp.equals(eql) { ... }
```

---

## Phase 23: Dictionary

### `Self` doesn't work for calling static methods

**Error:** `undefined name 'Self'`

**Cause:** Using `Self` to call a static method from within the same struct doesn't work:
```kestrel
public struct Dictionary[K, V] {
    private static func nextPowerOfTwo(n: Int64) -> Int64 { ... }

    public init(capacity: Int64) {
        let actualCap = Self.nextPowerOfTwo(capacity);  // Error
    }
}
```

**Workaround:** Move the function to module level:
```kestrel
func nextPowerOfTwo(n: Int64) -> Int64 { ... }

public struct Dictionary[K, V] {
    public init(capacity: Int64) {
        let actualCap = nextPowerOfTwo(capacity);  // Works
    }
}
```

---

### Struct fields must all be initialized - no uninitialized entries

**Error:** `initializer does not initialize all fields: 'key', 'value'`

**Cause:** All struct fields must be initialized in every initializer, even for "empty" or "placeholder" entries:
```kestrel
public struct Entry[K, V] {
    var key: K
    var value: V
    var occupied: Bool

    // Error - key and value not initialized
    public init() {
        self.occupied = false;
    }
}
```

**Workaround:** Require placeholder values to be passed in:
```kestrel
public init(placeholderKey: K, placeholderValue: V) {
    self.key = placeholderKey;
    self.value = placeholderValue;
    self.occupied = false;
}
```

This makes Dictionary initialization awkward since you need a key/value to create an empty dictionary with capacity.

---

### Hashable protocol not complete

**Cause:** The `Hasher` protocol exists but doesn't have `write` methods implemented, so types can't properly implement `Hashable`.

**Impact:** Dictionary can't use proper hashing and falls back to linear search (O(n) lookups instead of O(1)).

**Workaround:** Use linear search through entries instead of hash-based lookup. Correct but slow.

---

## Phase 24: Set

### `lang.panic` return type `!` doesn't unify with other branch types

**Error:** `type '!' does not conform to protocol 'ExpressibleByBoolLiteral'`

**Cause:** When an if-else expression has a concrete type in one branch and `lang.panic` (which returns `!` never type) in the other, the compiler can't unify the types:
```kestrel
public mutating func insert(element: T) -> Bool {
    if maybeSlot.isSome() {
        // ... do work
        true  // Bool
    } else {
        lang.panic("...")  // Returns `!`
    }
}
```

**Workaround:** Move the return value outside the if-else:
```kestrel
public mutating func insert(element: T) -> Bool {
    if maybeSlot.isSome() {
        // ... do work
    } else {
        lang.panic("...")
    }
    true  // Return after the if-else
}
```

---

## I/O Module (io)

### CLI `build` command only accepts single file (FIXED)

**Error:** `module 'X' not found` when passing multiple files to `build`

**Cause:** The `build` command accepted `file: String` not `files: Vec<String>`. When multiple files were passed, only the first was compiled.

**Fix:** Updated `src/main.rs` to accept multiple files in the `Build` command, matching `Check` behavior.

---

### Cross-module enum shorthand resolution fails

**Error:** `undefined name 'Ok'`

**Cause:** Using `.Ok(value)` or `.Err(error)` doesn't work when the `Result` enum is imported from another module, even when the return type clearly specifies `Result[T, E]`:
```kestrel
import std.result.(Result)

public func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
    .Ok(buf.count)  // Error: undefined name 'Ok'
}
```

**Workaround:** Use static constructor methods instead of shorthand:
```kestrel
Result.ok(value: buf.count)
Result.err(error: Error.last())
```

---

### Integer literal type inference in match

**Error:** `type mismatch: expected 'I64', found 'Int32'`

**Cause:** Match patterns with integer literals default to `I64`, but matching against `Int32` fails:
```kestrel
public func description() -> String {
    match self.code {  // self.code is Int32
        1 => "operation not permitted",  // Error: 1 is I64
        ...
    }
}
```

**Workaround:** Convert `Int32` to `Int64` before matching:
```kestrel
let code64 = Int64(from: self.code);
match code64 {
    1 => "operation not permitted",
    ...
}
```

---

### `public import` not supported

**Cause:** There's no `public import` or `pub use` equivalent to re-export symbols from submodules.

**Impact:** Users must import directly from submodules (e.g., `import io.error.(Error)`) rather than from the parent module (`import io.(Error)`).

**Workaround:** Document that users need to import from specific submodules.

---

### Module-level `public let` not supported

**Error:** Parse error when using `public let` at module level

**Cause:** Cannot declare constants at module level:
```kestrel
public let STDIN: Fd = 0  // Error
```

**Workaround:** Use functions returning constants:
```kestrel
public func STDIN() -> Fd { 0 }
```

---

## General Notes

### Blanket protocol extensions for operators

**Cause:** The `Hasher` protocol exists but doesn't have `write` methods implemented, so types can't properly implement `Hashable`.

**Impact:** Dictionary can't use proper hashing and falls back to linear search (O(1) lookups instead of O(1)).

**Workaround:** Use linear search through entries instead of hash-based lookup. Correct but slow.
