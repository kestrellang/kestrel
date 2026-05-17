# std.memory

## protocol `Allocator`

```kestrel
public protocol Allocator
```

Protocol for raw-memory allocators.

`Allocator` is the indirection collections use so they can be parameterised
over allocation strategy (e.g. `Array[T, A]`, `Buffer[T, A]`, custom
arenas). All three methods are `mutating` so stateful allocators (arenas,
pools) can update their bookkeeping; stateless wrappers around `malloc`
don't need to.

### Examples

```
var alloc = SystemAllocator();
if let .Some(p) = alloc.allocate(Layout.of[Int64]()) {
    // ... use p ...
    alloc.deallocate(p, Layout.of[Int64]())
}
```

_Defined in `lang/std/memory/allocator.ks`._

### Members

#### function `allocate`

```kestrel
mutating func allocate(Layout) -> RawPointer?
```

Returns a pointer to a fresh region matching `layout`, or `.None`
when allocation fails. Returned memory is uninitialised.

_Defined in `lang/std/memory/allocator.ks`._

#### function `deallocate`

```kestrel
mutating func deallocate(RawPointer, Layout)
```

Releases memory previously returned by `allocate` / `reallocate`.
`layout` must match the layout used to obtain the pointer.

##### Safety

`ptr` must have been produced by this allocator (or a clone of it)
for `layout`. Mismatching the layout, double-freeing, or freeing a
pointer from another allocator is undefined behavior.

_Defined in `lang/std/memory/allocator.ks`._

#### function `reallocate`

```kestrel
mutating func reallocate(RawPointer, Layout, Layout) -> RawPointer?
```

Resizes the allocation at `ptr` from `oldLayout` to `newLayout`.
On failure the original allocation is left intact and `.None` is
returned. On success the old pointer must not be reused — use the
returned pointer instead.

_Defined in `lang/std/memory/allocator.ks`._

## struct `ArraySlice`

```kestrel
public struct ArraySlice[T] { /* private fields */ }
```

Non-owning view over a contiguous run of `T` values.

`Slice` is the standard "borrow" type for arrays, buffers, and any
other contiguous storage: it stores a pointer + length and provides
safe and unchecked indexing, sub-slicing, iteration, and pattern
matching. The slice does **not** track or extend the lifetime of the
underlying storage — keeping a slice past the end of its source is a
use-after-free.

### Examples

```
let arr = [1, 2, 3, 4];
let s = arr.asSlice();
s[safe: 0]                    // .Some(1)
s[safe: 99]                   // .None
for x in s.iter() { print(x) }
```

### Memory Model

Non-owning. Drop the source (`Array`, `Buffer`, literal scope) and the
slice becomes dangling. Slices freely copy — they're just `(ptr, len)`
pairs.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(pointer: Pointer[T], count: Int64)
```

Builds a slice from an existing pointer and element count. The
caller is responsible for ensuring `count` elements live at `pointer`.

_Defined in `lang/std/memory/pointer.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Element count.

_Defined in `lang/std/memory/pointer.ks`._

#### function `first`

```kestrel
public func first() -> Optional[T]
```

First element, or `.None` for an empty slice.

_Defined in `lang/std/memory/pointer.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when `count == 0`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `last`

```kestrel
public func last() -> Optional[T]
```

Last element, or `.None` for an empty slice.

_Defined in `lang/std/memory/pointer.ks`._

#### field `pointer`

```kestrel
public var pointer: Pointer[T] { get }
```

Pointer to the first element. `pointer.offset(by: i)` reaches
element `i` (0-indexed).

_Defined in `lang/std/memory/pointer.ks`._

### Implements `ArrayMatchable`

#### typealias `Element`

```kestrel
type Element = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchGet`

```kestrel
public func matchGet(Int64) -> T
```

Compiler-driven element read; safe to skip the bounds check
because the matcher emits `index < matchLength()` first.

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchLength`

```kestrel
public func matchLength() -> Int64
```

Element count, exposed to the pattern matcher.

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchSlice`

```kestrel
public func matchSlice(Int64, Int64) -> ArraySlice[T]
```

Sub-slice for rest-pattern bindings (`..rest`). As above, the
matcher guarantees `0 <= from <= to <= matchLength()`.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Slice`

#### function `asSlice`

```kestrel
public func asSlice() -> ArraySlice[T]
```

Returns `self` — `ArraySlice` is already the borrowed view.

_Defined in `lang/std/memory/pointer.ks`._

#### function `ensureUnique`

```kestrel
public mutating func ensureUnique()
```

No-op — `ArraySlice` is a non-owning view with no COW barrier.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ArraySliceIterator[T]
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `iter`

```kestrel
public func iter() -> ArraySliceIterator[T]
```

Forward iterator over the elements.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
func isEqual(to: Self) -> Bool
```

Returns `true` iff `self` and `other` are considered equal. Should
be reflexive, symmetric, and transitive — `Hashable` requires equal
values to hash equal, so don't drift from those laws.

_Defined in `lang/std/core/protocols.ks`._

## struct `ArraySliceIterator`

```kestrel
public struct ArraySliceIterator[T] { /* private fields */ }
```

Forward iterator over an `ArraySlice[T]`. Holds a moving pointer and a
remaining count; advancing reads through the pointer.

### Representation

A `Pointer[T]` cursor and an `Int64` countdown.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Storage`

```kestrel
public init(ptr: Pointer[T], remaining: Int64)
```

Builds an iterator from a starting pointer and remaining count.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[T]
```

Yields the next element, or `.None` when the count reaches zero.

_Defined in `lang/std/memory/pointer.ks`._

## struct `Buffer`

```kestrel
public struct Buffer[T, A] where A: Allocator { /* private fields */ }
```

Owning, allocator-parameterised contiguous storage.

`Buffer` is the building block underneath `Array`, `String`, and any
other COW/growable collection. It owns its allocation, deallocates on
drop, and is `not Copyable` to keep ownership unique. For a non-owning
view see `Slice`; for a refcounted owning wrapper see `RcBox`.

### Examples

```
var buf = Buffer[Int64, SystemAllocator](capacity: 4, allocator: SystemAllocator());
buf.write(at: 0, value: 10);
buf.write(at: 1, value: 20);
buf.read(at: 0)              // .Some(10)
buf.resize(to: 8);           // grow in place if possible
```

### Representation

A `Pointer[T]` to the storage, an `Int64` capacity, and the allocator
instance. The buffer's contents are not initialised on construction —
reading an uninitialised slot is undefined behavior.

### Memory Model

Owning, unique. The deinit reclaims storage via the same allocator.
Marked `not Copyable` so an accidental `let b2 = b1` is rejected at
compile time; use a higher-level COW wrapper (e.g. via `RcBox`) for
shared semantics.

_Defined in `lang/std/memory/buffer.ks`._

### Members

#### initializer `With Capacity`

```kestrel
public init(Int64, A)
```

Allocates a buffer holding `capacity` elements. Storage is
uninitialised; the caller is responsible for writing valid `T`s
before reading them.

##### Errors

Panics with `"Buffer allocation failed"` if `allocator.allocate`
returns `.None`.

_Defined in `lang/std/memory/buffer.ks`._

#### function `asSlice`

```kestrel
public func asSlice() -> ArraySlice[T]
```

Returns a `ArraySlice[T]` over the entire buffer. The slice does not
extend the buffer's lifetime; callers must keep the buffer alive
for as long as they use the slice.

_Defined in `lang/std/memory/buffer.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

Number of element slots — not the count of *initialised* elements.

_Defined in `lang/std/memory/buffer.ks`._

#### field `pointer`

```kestrel
public var pointer: Pointer[T] { get }
```

Pointer to the first slot.

_Defined in `lang/std/memory/buffer.ks`._

#### function `read`

```kestrel
public func read(unchecked: Int64) -> T
```

Reads slot `index` without bounds checking.

##### Safety

`index` must satisfy `0 <= index < capacity`, and the slot must
already hold an initialised `T`. Out-of-range or uninitialised
reads are undefined behavior.

_Defined in `lang/std/memory/buffer.ks`._

#### function `read`

```kestrel
public func read(at: Int64) -> T?
```

Reads slot `index`, returning `.None` when out of range. As with
the unchecked form, the slot must already hold an initialised `T`.

_Defined in `lang/std/memory/buffer.ks`._

#### function `resize`

```kestrel
public mutating func resize(to: Int64)
```

Grows or shrinks the storage to hold `newCapacity` elements via
the allocator's `reallocate`. On success, existing initialised
elements are preserved up to the smaller of the two capacities;
the new pointer becomes the buffer's storage.

##### Errors

Panics with `"Buffer resize failed"` if `reallocate` returns
`.None` (the original allocation is left intact, but the panic
aborts).

_Defined in `lang/std/memory/buffer.ks`._

#### function `slice`

```kestrel
public func slice(from: Int64, to: Int64) -> ArraySlice[T]?
```

Returns a slice over `[start, end)`, or `.None` when the range
falls outside `[0, capacity]`. As with `asSlice`, the slice
borrows from the buffer.

_Defined in `lang/std/memory/buffer.ks`._

#### function `write`

```kestrel
public func write(unchecked: Int64, T)
```

Writes `value` into slot `index` without bounds checking.

##### Safety

Same precondition as `read(unchecked:)` — `0 <= index < capacity`.

_Defined in `lang/std/memory/buffer.ks`._

#### function `write`

```kestrel
public func write(at: Int64, T) -> Bool
```

Writes `value` to slot `index`. Returns `false` (and does
nothing) when out of range.

_Defined in `lang/std/memory/buffer.ks`._

## struct `CowBox`

```kestrel
public struct CowBox[T] where T: Cloneable { /* private fields */ }
```

Copy-on-write wrapper around `RcBox[T]`.

Mutable owners use `CowBox`; read-only shared owners (like
`StringSlice`) hold the inner `RcBox` directly via `shareBox()`.
The mutation protocol is `write()` → modify → `setValue()`.

### Examples

```
var box = CowBox(MyStorage());
var s = box.write();   // COW barrier — clones if shared
s.len = s.len + 1;
box.setValue(s);        // write back
```

### Representation

A single `RcBox[T]` field.

### Memory Model

Same as `RcBox`: non-atomic refcount. Cloning bumps the count;
`write` splits off a private copy when shared.

_Defined in `lang/std/memory/cowbox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(T)
```

Allocates fresh storage holding `value` with refcount 1.

_Defined in `lang/std/memory/cowbox.ks`._

#### initializer `Inner`

```kestrel
public init(inner: RcBox[T])
```

Adopts an existing `RcBox` without allocating.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `isUnique`

```kestrel
public func isUnique() -> Bool
```

Returns `true` when no other clone shares this storage.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `read`

```kestrel
public func read() -> T
```

Read access — no clone, no refcount check.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `setValue`

```kestrel
public func setValue(T)
```

Writes `value` into the storage in place. Only valid after
a preceding `write()` call (which ensures uniqueness).

_Defined in `lang/std/memory/cowbox.ks`._

#### function `shareBox`

```kestrel
public func shareBox() -> RcBox[T]
```

Returns a shared `RcBox` pointing at the same storage
(refcount bumped). Use this to hand read-only access to
types like `StringSlice`.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `write`

```kestrel
public mutating func write() -> T
```

Write access — clones storage if shared, then returns the
(now unique) value. Caller modifies and calls `setValue`.

_Defined in `lang/std/memory/cowbox.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> CowBox[T]
```

Shares storage with the returned clone (refcount bump).

_Defined in `lang/std/memory/cowbox.ks`._

## typealias `GlobalAllocator`

```kestrel
public type GlobalAllocator = SystemAllocator
```

Project-wide default allocator, aliased to `SystemAllocator`. Override
at the project level if a global custom allocator is needed.

_Defined in `lang/std/memory/allocator.ks`._

## struct `Layout`

```kestrel
public struct Layout { /* private fields */ }
```

Size + alignment pair describing the memory footprint of a type.

Allocators take a `Layout` rather than a raw byte count so they can
honour alignment requirements (SIMD types, page-aligned buffers, etc.).
The static `of[T]` and `array[T]` factories cover the common cases;
`merge` and `padToAlign` exist for hand-rolled struct layouts.

### Examples

```
let l = Layout.of[Int64]();           // size 8, alignment 8
let buf = Layout.array[UInt8](1024);  // size 1024, alignment 1
allocator.allocate(l)
```

### Representation

Two `Int64`s — `size` and `alignment`. No invariants enforced at
construction; misaligned layouts are caught (or undefined) at the
allocator level.

_Defined in `lang/std/memory/layout.ks`._

### Members

#### initializer `From Fields`

```kestrel
public init(size: Int64, alignment: Int64)
```

Builds a layout from explicit `size` and `alignment`. Caller is
responsible for keeping `alignment` a power of two.

_Defined in `lang/std/memory/layout.ks`._

#### field `alignment`

```kestrel
public var alignment: Int64
```

Required alignment in bytes — always a power of two for layouts
produced by `of`/`array`.

_Defined in `lang/std/memory/layout.ks`._

#### function `array`

```kestrel
public static func array[T](Int64) -> Layout
```

Layout for `count` contiguous `T` values. Inherits the element's
alignment; size is `sizeof[T] * count` with no inter-element padding
(T is assumed already padded to its own alignment).

_Defined in `lang/std/memory/layout.ks`._

#### function `merge`

```kestrel
public func merge(with: Layout) -> (Layout, Int64)
```

Concatenates `other` after `self`, mimicking how a C struct lays
out its second field. Returns the combined layout and the byte
offset where `other`'s storage starts (handy for building field
access tables by hand).

_Defined in `lang/std/memory/layout.ks`._

#### function `of`

```kestrel
public static func of[T]() -> Layout
```

Layout for a single value of `T` — uses the compiler-known
`sizeof` and `alignof` for the type.

_Defined in `lang/std/memory/layout.ks`._

#### function `padToAlign`

```kestrel
public func padToAlign() -> Layout
```

Rounds `size` up to the next multiple of `alignment`. Use when
emitting a value into a packed array — without padding, element
`i+1` would land at the wrong offset.

_Defined in `lang/std/memory/layout.ks`._

#### field `size`

```kestrel
public var size: Int64
```

Footprint in bytes.

_Defined in `lang/std/memory/layout.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Layout) -> Bool
```

Equal when both fields match.

_Defined in `lang/std/memory/layout.ks`._

## struct `LiteralSlice`

```kestrel
public struct LiteralSlice[T] { /* private fields */ }
```

Read-only view over the compiler-emitted backing buffer for an array
literal.

User code rarely names this type directly: it appears in
`ExpressibleByArrayLiteral.init(arrayLiteral:)` and friends so that
types accepting `[a, b, c]` literals can iterate the elements without
touching raw pointers. The slice does **not** own the storage — the
compiler keeps the literal alive for the duration of the call.

### Examples

```
// Conforming to ExpressibleByArrayLiteral
public struct MyVec[T]: ExpressibleByArrayLiteral {
    type Element = T
    public init(arrayLiteral lit: LiteralSlice[T]) {
        var v = MyVec();
        for x in lit { v.push(x) }
        self = v
    }
}
```

### Memory Model

Non-owning. The backing storage is compiler-managed and lives for the
scope of the literal expression. Capturing a `LiteralSlice` past that
scope is a use-after-free.

_Defined in `lang/std/memory/literal_slice.ks`._

### Members

#### subscript `Checked Index`

```kestrel
public subscript(checked: Int64) -> T? { get }
```

Reads element `index`, returning `.None` on out-of-bounds.

_Defined in `lang/std/memory/literal_slice.ks`._

#### initializer `From Storage`

```kestrel
public init(pointer: lang.ptr[T], count: lang.i64)
```

Builds the slice from the raw pointer and count the compiler emits.

_Defined in `lang/std/memory/literal_slice.ks`._

#### subscript `Indexed`

```kestrel
public subscript(Int64) -> T { get }
```

Reads element `index`, panicking on out-of-bounds.

The default subscript: trades a single comparison for a guaranteed
trap on bad input. Use `(unchecked:)` inside compiler-emitted init
paths where the index is statically known in range, or
`(checked:)` to handle out-of-range without a panic.

##### Errors

Panics with `"LiteralSlice index out of bounds"` if `index < 0`
or `index >= count`.

_Defined in `lang/std/memory/literal_slice.ks`._

#### subscript `Unchecked Index`

```kestrel
public subscript(unchecked: Int64) -> T { get }
```

Reads element `index` without bounds checking.

##### Safety

Undefined behavior if `index < 0` or `index >= count`. Compiler-
emitted init paths that use this guarantee the index is in range;
do not expose this subscript to user input without checking
`count` first.

_Defined in `lang/std/memory/literal_slice.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of elements in the literal.

_Defined in `lang/std/memory/literal_slice.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` for `[]`.

_Defined in `lang/std/memory/literal_slice.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/literal_slice.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = LiteralSliceIterator[T]
```

_Defined in `lang/std/memory/literal_slice.ks`._

#### function `iter`

```kestrel
public func iter() -> LiteralSliceIterator[T]
```

Iterator over the elements in source order.

_Defined in `lang/std/memory/literal_slice.ks`._

## struct `LiteralSliceIterator`

```kestrel
public struct LiteralSliceIterator[T] { /* private fields */ }
```

Iterator yielded by `LiteralSlice.iter()`. Walks the backing buffer
element-by-element, advancing a typed pointer.

### Representation

A `Pointer[T]` plus a remaining count. No `Slice` indirection — the
iterator is what `LiteralSlice` hands out instead of exposing its
pointer directly.

_Defined in `lang/std/memory/literal_slice.ks`._

### Members

#### initializer `From Storage`

```kestrel
public init(ptr: Pointer[T], remaining: Int64)
```

Builds an iterator from a typed pointer and element count.
Not normally called by user code.

_Defined in `lang/std/memory/literal_slice.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/literal_slice.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Yields the next element, or `.None` once the buffer is exhausted.

_Defined in `lang/std/memory/literal_slice.ks`._

## struct `Pointer`

```kestrel
public struct Pointer[T] { /* private fields */ }
```

Typed pointer to a single value of `T`.

Element-typed counterpart to `RawPointer`: `offset(by:)` strides in
units of `sizeof[T]`, and `pointee` reads/writes through the address.
`Pointer[T]` is FFI-safe when `T` is.

### Examples

```
var x = 42;
let p = Pointer(to: x);
p.read()                       // 42
p.write(100)                   // x is now 100
p.pointee = 7                  // x is now 7
```

### Representation

One `lang.ptr[T]`. The wrapping struct is purely a typing convenience —
it lowers to a bare machine pointer.

### Memory Model

Non-owning. The pointee's lifetime is the caller's responsibility; the
pointer does not increment any refcount, register with any GC, or
trigger a deinit.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Raw`

```kestrel
public init(raw: lang.ptr[T])
```

Wraps an existing primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### initializer `To Value`

```kestrel
public init(to: T)
```

Takes the address of `value`. Equivalent to `&value` in C — the
caller must ensure `value` outlives any use of the resulting
pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### field `address`

```kestrel
public var address: UInt64 { get }
```

Numeric address — same value as `asRaw().address`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `asRaw`

```kestrel
public func asRaw() -> RawPointer
```

Drops the type tag, returning a `RawPointer` to the same address.

_Defined in `lang/std/memory/pointer.ks`._

#### function `cast`

```kestrel
public func cast[U]() -> Pointer[U]
```

Reinterprets the address as a `Pointer[U]`.

##### Safety

Same caveats as `RawPointer.cast` — the storage must be valid for
`U` (size, alignment, contents) at the moment of the read/write.

_Defined in `lang/std/memory/pointer.ks`._

#### field `isNull`

```kestrel
public var isNull: Bool { get }
```

Convenience for `address == 0`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `nullPointer`

```kestrel
public static func nullPointer() -> Pointer[T]
```

Returns a typed null pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `offset`

```kestrel
public func offset(by: Int64) -> Pointer[T]
```

Strides the pointer by `n` *elements* (multiplied by `sizeof[T]`).
Compare with `RawPointer.offset`, which strides by raw bytes.

_Defined in `lang/std/memory/pointer.ks`._

#### field `pointee`

```kestrel
public var pointee: T { get set }
```

Live view of the value at the address. `get` reads through the
pointer; `set` writes. Both are unchecked — see `# Safety`.

##### Safety

The pointer must be non-null and the storage must hold a valid
initialised `T`. Reading past the end of an allocation, after
the pointee has been freed, or through a dangling pointer is
undefined behavior.

_Defined in `lang/std/memory/pointer.ks`._

#### field `raw`

```kestrel
public var raw: lang.ptr[T] { get }
```

The wrapped primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `read`

```kestrel
public func read() -> T
```

Reads `T` from the address. Same safety preconditions as `pointee.get`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `write`

```kestrel
public func write(T)
```

Writes `value` through the pointer. Same safety preconditions as
`pointee.set`.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Pointer[T]) -> Bool
```

Address-based equality.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the underlying address.

Heap allocations cluster on alignment boundaries, so the raw
address has predictable low bits. We run the address through
Murmur3's `fmix64` finalizer (two rounds of `xor-shift /
multiply`) before hashing so every input bit avalanches across
the 64-bit output. Without this, pointer-keyed maps see
collision clustering driven by the allocator's stride.

_Defined in `lang/std/memory/pointer.ks`._

## struct `RawPointer`

```kestrel
public struct RawPointer { /* private fields */ }
```

Untyped pointer to raw memory — `void*` in C terms.

Used at FFI boundaries and as an intermediate when casting between
typed pointers. `RawPointer` deliberately exposes no read/write methods
of its own; cast to `Pointer[T]` first via `cast[T]()`. Equality and
hashing are address-based.

### Examples

```
let p = RawPointer.nullPointer();
p.isNull                                // true
let typed: Pointer[Int64] = p.cast[Int64]()
```

### Representation

One `lang.ptr[lang.i8]`. FFI-safe — passes as a single machine pointer.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Address`

```kestrel
public init(address: UInt64)
```

Reconstructs a pointer from a numeric address. Useful for
platform-specific encodings (handles, MMIO addresses); incorrect
addresses produce a pointer that dereferences to undefined memory.

_Defined in `lang/std/memory/pointer.ks`._

#### initializer `From Raw`

```kestrel
public init(raw: lang.ptr[lang.i8])
```

Wraps an existing primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### field `address`

```kestrel
public var address: UInt64 { get }
```

Numeric address of the pointee. Round-trips through
`RawPointer(address:)`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `cast`

```kestrel
public func cast[T]() -> Pointer[T]
```

Reinterprets the address as a `Pointer[T]`.

##### Safety

The caller must ensure the address holds a valid `T` (correct size,
alignment, and initialised contents) before reading through the
returned pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### field `isNull`

```kestrel
public var isNull: Bool { get }
```

Convenience for `address == 0`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `nullPointer`

```kestrel
public static func nullPointer() -> RawPointer
```

Returns the canonical null pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `offset`

```kestrel
public func offset(by: Int64) -> RawPointer
```

Adds `bytes` to the address (no element-size scaling — this is
raw byte arithmetic). Use `Pointer[T].offset` for element-typed
strides.

_Defined in `lang/std/memory/pointer.ks`._

#### field `raw`

```kestrel
public var raw: lang.ptr[lang.i8]
```

The wrapped primitive `i8*`.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: RawPointer) -> Bool
```

Address-based equality. Two `RawPointer`s pointing into different
allocations are equal iff their addresses coincide.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the underlying address.

Heap allocations cluster on alignment boundaries, so the raw
address has predictable low bits. We run the address through
Murmur3's `fmix64` finalizer (two rounds of `xor-shift /
multiply`) before hashing so every input bit avalanches across
the 64-bit output. Without this, pointer-keyed maps see
collision clustering driven by the allocator's stride.

_Defined in `lang/std/memory/pointer.ks`._

## struct `RcBox`

```kestrel
public struct RcBox[T] { /* private fields */ }
```

Heap allocation with a strong-reference count, used as the underlying
storage for the stdlib's copy-on-write types.

`String`, `Array`, and `Dictionary` all wrap an `RcBox` so that a
plain assignment shares storage and only the first mutating call pays
for a deep copy. Reach for `RcBox` directly when building a similar
COW type; for plain shared ownership without mutation prefer a more
purpose-built container.

### Examples

```
let a = RcBox(value: [1, 2, 3]);
let b = a.clone();          // shares storage; refCount == 2
if b.isUnique() { ... } else { let c = b.deepClone(); /* ... */ }
```

### Representation

One `Pointer[RcBoxStorage[T]]`. The pointed-to block holds an `Int64`
refcount followed by the `T` value, allocated via `SystemAllocator`.

### Memory Model

Reference-counted, non-atomic (today — see TODOs). `clone()` increments
the count and shares storage; `deinit` decrements and frees on zero.
`deepClone()` allocates a fresh `RcBox` carrying a copied value.

### Guarantees

- `isUnique()` returning `true` means in-place mutation is safe; this is
  how COW types decide whether to copy.
- The refcount is currently **not** atomic, so `RcBox` is not safe to
  share across threads.

_Defined in `lang/std/memory/rcbox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(T)
```

Allocates fresh storage holding `value` with refcount 1. Panics if
the underlying `SystemAllocator` returns `.None`.

##### Errors

Panics with `"RcBox allocation failed"` on allocation failure.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `clone`

```kestrel
public func clone() -> RcBox[T]
```

Bumps the refcount and returns a second `RcBox` pointing at the
same storage. The receiver and the returned box now both reference
the value; the next mutation should test `isUnique`.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `deepClone`

```kestrel
public func deepClone() -> RcBox[T]
```

Allocates fresh storage with a copy of the value. Used by COW
types when `isUnique()` returns `false` — splits off a private
copy so the caller can mutate without affecting other clones.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `getValue`

```kestrel
public func getValue() -> T
```

Reads the wrapped value out of storage. Returns a copy — the
underlying `T` is read through a pointer, so callers see a
snapshot, not a live reference.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `isUnique`

```kestrel
public func isUnique() -> Bool
```

Returns `true` when no other clone is sharing storage. The litmus
test for "safe to mutate in place" in COW collections.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `refCount`

```kestrel
public func refCount() -> Int64
```

Current strong reference count. Mostly useful for tests and
diagnostics; production COW logic should branch on `isUnique`.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `setValue`

```kestrel
public func setValue(T)
```

Overwrites the wrapped value in place. Safe only when this is the
unique owner (`isUnique() == true`); otherwise other clones see the
new value, defeating COW. The COW types check `isUnique` before
calling this and `deepClone` otherwise.

_Defined in `lang/std/memory/rcbox.ks`._

## struct `SystemAllocator`

```kestrel
public struct SystemAllocator { /* private fields */ }
```

`Allocator` backed by libc `malloc`/`free`/`realloc`. Used as the
default `GlobalAllocator` and by every collection that doesn't pick a
custom allocator.

### Memory Model

Stateless: the struct holds no fields. All bookkeeping lives in libc's
heap. Cloning or copying the allocator has no effect on the heap state.

_Defined in `lang/std/memory/allocator.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds a stateless system allocator. No heap interaction occurs here.

_Defined in `lang/std/memory/allocator.ks`._

### Implements `Allocator`

#### function `allocate`

```kestrel
public mutating func allocate(Layout) -> RawPointer?
```

Calls `malloc(layout.size)`. Alignment beyond `malloc`'s natural
alignment (typically 16) is **not** honoured — types that need
larger alignment should use a different allocator.

_Defined in `lang/std/memory/allocator.ks`._

#### function `deallocate`

```kestrel
public mutating func deallocate(RawPointer, Layout)
```

Calls `free(ptr)`. The `layout` argument is ignored — kept for
protocol conformance; allocators that need it (arenas) use it.

_Defined in `lang/std/memory/allocator.ks`._

#### function `reallocate`

```kestrel
public mutating func reallocate(RawPointer, Layout, Layout) -> RawPointer?
```

Calls `realloc(ptr, newLayout.size)`. As with `allocate`, only
`malloc`-natural alignment is guaranteed.

_Defined in `lang/std/memory/allocator.ks`._

