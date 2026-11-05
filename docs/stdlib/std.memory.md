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

## struct `Buffer`

```kestrel
public struct Buffer[T, A]: not Copyable where A: Allocator { /* private fields */ }
```

Owning, allocator-parameterised contiguous storage.

`Buffer` is the building block underneath `Array`, `String`, and any
other COW/growable collection. It owns its allocation, deallocates on
drop, and is `not Copyable` to keep ownership unique. For a non-owning
view see `Sliceable`; for a refcounted owning wrapper see `RcBox`.

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

#### function `asUnsafeSlice`

```kestrel
public func asUnsafeSlice() -> UnsafeSlice[T]
```

Returns a `UnsafeSlice[T]` over the entire buffer. The slice does not
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
public func slice(from: Int64, to: Int64) -> UnsafeSlice[T]?
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
public struct CowBox[T] where T: Cloneable, T: not Sized, T: not Static { /* private fields */ }
```

_Defined in `lang/std/memory/cowbox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(consuming T) where T: Sized
```

Allocates fresh storage holding `value` with refcount 1.

_Defined in `lang/std/memory/cowbox.ks`._

#### initializer `Inner`

```kestrel
public init(inner: RcBox[T])
```

Adopts an existing `RcBox` without allocating.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `ensureUnique`

```kestrel
public mutating func ensureUnique()
```

COW barrier — makes the storage uniquely owned, cloning it if shared.

The fork clones the heap value in place (`RcBox.fork`), so a
shared box is deep-copied exactly once and an unsized payload works.
Every write (`value`'s `mutating ref`, `pointeeMutRef`) goes through
here.

_Defined in `lang/std/memory/cowbox.ks`._

#### field `isUnique`

```kestrel
public var isUnique: Bool { get }
```

`true` when no other copy shares this storage.

_Defined in `lang/std/memory/cowbox.ks`._

#### field `value`

```kestrel
public var value: T { ref mutating ref }
```

The payload, in place. Reading borrows the shared storage; writing
(`c.value.len = 0`, a `mutating` argument, a mutating method) forks a
shared box first, so other copies never see the change.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `valuePtr`

```kestrel
internal func valuePtr() -> Pointer[T]
```

Pointer to the payload on the heap. Internal plumbing for the
accessors below; writing through it skips the fork.

_Defined in `lang/std/memory/cowbox.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> CowBox[T]
```

Shares storage with the returned clone (refcount bump).

_Defined in `lang/std/memory/cowbox.ks`._

### Implements `Indirection`

#### typealias `Target`

```kestrel
type Target = T
```

_Defined in `lang/std/memory/cowbox.ks`._

#### function `pointeeRef`

```kestrel
public func pointeeRef() -> &T
```

_Defined in `lang/std/memory/cowbox.ks`._

### Implements `ExistentialBox`

### Implements `MutableIndirection`

_Conditional: `where T: Cloneable`._

#### function `pointeeMutRef`

```kestrel
public mutating func pointeeMutRef() -> &mutating T where T: Cloneable
```

_Defined in `lang/std/memory/cowbox.ks`._

## typealias `DefaultBox`

```kestrel
public type DefaultBox[T] = RcBox[T]
```

The box `any P` is written in: `any P` is `DefaultBox[<raw existential>]`,
so copies of an `any P` share one payload (reference semantics). Rebinding
this alias changes every default existential at once; write `any[B] P` to
pick a box at one site (docs/plans/existentials/existentials-design.md).

_Defined in `lang/std/memory/rcbox.ks`._

## protocol `ExistentialBox`

```kestrel
public protocol ExistentialBox
```

The contract a box type meets to hold an existential: `any[B] P` is
`B[<raw existential>]` (docs/plans/existentials/existentials-design.md §3).

Beyond conforming, `B` must have exactly one type parameter, declared
`not Sized`, and store it only behind a pointer, so the compiler can widen
`B[T]` to `B[<raw existential>]` (E402 checks this).

_Defined in `lang/std/memory/sharedbox.ks`._

### Members

#### initializer `init`

```kestrel
init(consuming Target)
```

Takes ownership of `value`, moves it into the box's storage and
returns the first handle onto it. Only ever called at a sized
`Target`; boxing an existential converts the result afterwards.

`value` is a single-name parameter, so call sites are positional:
`B(payload)`, never `B(value: payload)`.

_Defined in `lang/std/memory/sharedbox.ks`._

### Implements `Indirection`

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

Two `Int64`s — `size` and `alignment`. Explicitly constructed layouts
are validated by allocators; the factories in this type reject negative
sizes, invalid alignments, and arithmetic overflow.

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
public static func array[T](Int64) -> Layout where T: not Copyable
```

Layout for `count` contiguous `T` values. Traps if `count` is negative
or the total size cannot be represented by `Int64`. Use
`arrayChecked` to handle those cases explicitly.

_Defined in `lang/std/memory/layout.ks`._

#### function `arrayChecked`

```kestrel
public static func arrayChecked[T](Int64) -> Layout? where T: not Copyable
```

Checked counterpart to `array`. Returns `None` for a negative count
or when `sizeof[T] * count` overflows.

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

#### function `mergeChecked`

```kestrel
public func mergeChecked(with: Layout) -> (Layout, Int64)?
```

Checked counterpart to `merge`. Returns `None` if either input is
invalid or if padding/size arithmetic overflows.

_Defined in `lang/std/memory/layout.ks`._

#### function `of`

```kestrel
public static func of[T]() -> Layout where T: not Copyable, T: not Static
```

Layout for a single value of `T` — uses the compiler-known
`sizeof` and `alignof` for the type.

_Defined in `lang/std/memory/layout.ks`._

#### function `ofValue`

```kestrel
public static func ofValue[T](at: Pointer[T]) -> Layout where T: not Copyable, T: not Sized, T: not Static
```

Layout of the value at `pointer` — the one way to measure a payload
that may be unsized (a raw existential, whose size and alignment
live in its function table). For a sized `T` this is `of[T]()`.

_Defined in `lang/std/memory/layout.ks`._

#### function `padToAlign`

```kestrel
public func padToAlign() -> Layout
```

Rounds `size` up to the next multiple of `alignment`. Traps for an
invalid layout or arithmetic overflow; use `padToAlignChecked` to
handle failure explicitly.

_Defined in `lang/std/memory/layout.ks`._

#### function `padToAlignChecked`

```kestrel
public func padToAlignChecked() -> Layout?
```

Checked counterpart to `padToAlign`.

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

A `Pointer[T]` plus a remaining count. No `Sliceable` indirection — the
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

## struct `ManuallyDrop`

```kestrel
public struct ManuallyDrop[T] { /* private fields */ }
```

_Defined in `lang/std/memory/manually_drop.ks`._

### Members

#### initializer `init`

```kestrel
public init(T)
```

_Defined in `lang/std/memory/manually_drop.ks`._

#### field `value`

```kestrel
public var value: T { get }
```

_Defined in `lang/std/memory/manually_drop.ks`._

## struct `MutRefSliceIterator`

```kestrel
public struct MutRefSliceIterator[T] { /* private fields */ }
```

Forward iterator yielding MUTABLE REFERENCES (`&mutating T`) to
contiguous elements — in-place mutation without writeback
(`for x in arr.mutatingRefs() { x += 1 }`). Surfaced as
`Array.mutatingRefs()`, which runs the COW barrier before handing out
the buffer.

### Invalidation

Same contract as `RefSliceIterator`: mutating the source collection's
STRUCTURE while iterating invalidates the cursor and any yielded
reference. Element writes through the yielded references are the
intended use.

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
type Item = &mutating T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[&mutating T]
```

Yields a mutable reference to the next element in place, or
`.None` when the count reaches zero.

_Defined in `lang/std/memory/pointer.ks`._

## struct `OwnedBox`

```kestrel
public struct OwnedBox[T]: not Copyable where T: not Copyable, T: not Sized, T: not Static { /* private fields */ }
```

_Defined in `lang/std/memory/ownedbox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(consuming T) where T: Sized
```

Allocates a block holding `value`. Panics if the underlying
`SystemAllocator` returns `.None`.

##### Errors

Panics with `"OwnedBox allocation failed"` on allocation failure.

_Defined in `lang/std/memory/ownedbox.ks`._

#### typealias `Target`

```kestrel
type Target = T
```

_Defined in `lang/std/memory/ownedbox.ks`._

#### function `pointeeRef`

```kestrel
public func pointeeRef() -> &T
```

_Defined in `lang/std/memory/ownedbox.ks`._

#### function `takeValue`

```kestrel
public consuming func takeValue() -> T where T: Sized
```

_Defined in `lang/std/memory/ownedbox.ks`._

#### field `value`

```kestrel
public var value: T { ref mutating ref }
```

The payload, in place: `&b.value` borrows it, `b.value.x = 1` writes
it. The box is its only owner, so a write needs a mutable box.

_Defined in `lang/std/memory/ownedbox.ks`._

#### function `valuePtr`

```kestrel
internal func valuePtr() -> Pointer[T]
```

Pointer to the payload. Valid while the box is alive.

_Defined in `lang/std/memory/ownedbox.ks`._

### Implements `MutableIndirection`

#### function `pointeeMutRef`

```kestrel
public mutating func pointeeMutRef() -> &mutating T
```

_Defined in `lang/std/memory/ownedbox.ks`._

### Implements `ConsumingIndirection`

#### function `consumePointee`

```kestrel
public consuming func consumePointee[R]((Pointer[T]) -> R) -> R where R: not Copyable, R: not Static
```

_Defined in `lang/std/memory/ownedbox.ks`._

### Implements `ExistentialBox`

### Implements `Cloneable`

_Conditional: `where T: Cloneable, T: Sized`._

#### function `clone`

```kestrel
public func clone() -> OwnedBox[T] where T: Cloneable, T: Sized
```

_Defined in `lang/std/memory/ownedbox.ks`._

## struct `Pointer`

```kestrel
public struct Pointer[T] where T: not Copyable, T: not Static, T: not Sized { /* private fields */ }
```

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Raw`

```kestrel
public init(raw: lang.ptr[T])
```

Wraps an existing primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### typealias `Target`

```kestrel
type Target = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### initializer `To Value`

```kestrel
public init(to: T) where T: Sized
```

Takes the address of `value`. Equivalent to `&value` in C — the
borrowed place itself is captured; no copy is made.

##### Safety

The pointer does not keep `value` alive: the caller must ensure the
place outlives every use of the resulting pointer, or any read is
undefined behavior.

This is the sole capture init and it accepts any place — `var` or
`let` — yielding the same write-capable `Pointer[T]`. Writing
through a pointer captured from an immutable place is the C
const-cast footgun: it compiles, and it is on the caller to know the
storage is actually mutable.

_Defined in `lang/std/memory/pointer.ks`._

#### field `address`

```kestrel
public var address: UInt64 { get }
```

Live view of the value at the address. `get` reads through the
pointer; `set` writes. Both are unchecked — see `# Safety`.

##### Safety

The pointer must be non-null and the storage must hold a valid
initialised `T`. Reading past the end of an allocation, after
the pointee has been freed, or through a dangling pointer is
undefined behavior.
Numeric address — same value as `asRaw().address`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `asRaw`

```kestrel
public func asRaw() -> RawPointer
```

Drops the type tag, returning a `RawPointer` to the same address.

_Defined in `lang/std/memory/pointer.ks`._

#### function `assign`

```kestrel
public func assign(consuming T) where T: Sized
```

Replaces an INITIALIZED pointee: runs the old value's destructor in
place, then writes `value`. Use this — not `write` — to overwrite a
live slot; `write` is for uninitialized (or moved-out) storage and
drops nothing, so overwriting a live heap value with it leaks.

_Defined in `lang/std/memory/pointer.ks`._

#### function `cast`

```kestrel
public func cast[U]() -> Pointer[U] where U: not Copyable, U: not Sized, U: not Static
```

Reinterprets the address as a `Pointer[U]`.

##### Safety

Same caveats as `RawPointer.cast` — the storage must be valid for
`U` (size, alignment, contents) at the moment of the read/write.

_Defined in `lang/std/memory/pointer.ks`._

#### function `dropInPlace`

```kestrel
public func dropInPlace()
```

Runs T's destructor at this address without copying the value to stack.
The pointer remains valid but the pointee is left in a destroyed state.

_Defined in `lang/std/memory/pointer.ks`._

#### field `isNull`

```kestrel
public var isNull: Bool { get }
```

Convenience for `address == 0`.

_Defined in `lang/std/memory/pointer.ks`._

#### field `mutatingValue`

```kestrel
public var mutatingValue: &mutating T { get }
```

Mutable borrowed view of the pointee. Mutating methods and
`mutating`-convention arguments through the result write the storage
in place — no clone, no write-back.

##### Safety

- Same validity preconditions as `value`.
- This is the const-cast escape hatch: `Pointer(to: x).mutatingValue`
  on a `let`/shared place compiles and writes through it — the
  compiler does not stop it (same class as `write()` after
  `Pointer(to:)` on a `let`).
- Writing through a pointer into storage shared by a COW container
  (`Array`, `String`, `Dictionary`) is visible through every copy,
  breaking value semantics. Make the storage unique first.

_Defined in `lang/std/memory/pointer.ks`._

#### function `nullPointer`

```kestrel
public static func nullPointer() -> Pointer[T]
```

Returns a typed null pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `offset`

```kestrel
public func offset(by: Int64) -> Pointer[T] where T: Sized
```

Strides the pointer by `n` *elements* (multiplied by `sizeof[T]`).
Compare with `RawPointer.offset`, which strides by raw bytes.

_Defined in `lang/std/memory/pointer.ks`._

#### function `pointeeRef`

```kestrel
public func pointeeRef() -> &T
```

_Defined in `lang/std/memory/pointer.ks`._

#### field `raw`

```kestrel
public var raw: lang.ptr[T] { get }
```

The wrapped primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `read`

```kestrel
public func read() -> T where T: Copyable
```

Bit-copies `T` out of the address — `T.deinit` does not run. This is
a copy, so it requires `T: Copyable`; for non-Copyable pointees use
`value` (borrow) or `take` (consuming read-out).

_Defined in `lang/std/memory/pointer.ks`._

#### function `take`

```kestrel
public func take() -> T where T: Sized
```

Moves `T` out of the address bitwise — a *consuming* read-out. Unlike
`read()` there is no `Copyable` requirement: ownership of the value
transfers to the caller, and the pointee is left logically
uninitialised. `T.deinit` does not run here.

This is the dual of `write(consuming:)` and the building block for
relocating non-Copyable values (e.g. `swap`).

##### Safety

Same validity preconditions as `read()`, and additionally: after the
take the pointee must not be read or dropped until the slot is
re-initialised (e.g. via `write`). Taking the same slot twice without
an intervening write double-owns the value and will double-free.

_Defined in `lang/std/memory/pointer.ks`._

#### field `value`

```kestrel
public var value: &T { get }
```

Borrowed view of the pointee — no copy, no clone, no `T: Copyable`
requirement. Member access, operators, and borrow-convention calls
go through it in place; binding it (`let x = p.value;`) stores an
owned copy instead (binding decay).

##### Safety

- The pointer must be non-null and the storage must hold a valid,
  initialized `T` for as long as the reference is used.
- The reference inherits this pointer's contract: the compiler does
  not verify the pointee's lifetime. Using it after the storage is
  freed is undefined behavior — the same trust point as `read()`,
  returning a view instead of a copy.

_Defined in `lang/std/memory/pointer.ks`._

#### function `withMetadata`

```kestrel
public func withMetadata[U](of: Pointer[U]) -> Pointer[T] where U: not Copyable, U: not Sized, U: not Static
```

This address, carrying `other`'s pointee metadata.

A pointer to an unsized pointee (a raw existential) is two words: the
address and the payload's function table. Address arithmetic through
`asRaw()` keeps only the first; this puts the table back. For a sized
`T` it returns `self` unchanged.

_Defined in `lang/std/memory/pointer.ks`._

#### function `write`

```kestrel
public func write(consuming T) where T: Sized
```

Writes `value` through the pointer, without dropping the old pointee.

##### Safety

The pointer must be non-null and the storage valid for writes of
`T`. The previous pointee is overwritten without running its
`deinit`. If the pointer was captured with `Pointer(to:)` from a
`let` place, writing is the documented const-cast footgun — the
compiler does not stop it.

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

### Implements `FFISafe`

_Conditional: `where T: FFISafe`._

### Implements `MutableIndirection`

#### function `pointeeMutRef`

```kestrel
public mutating func pointeeMutRef() -> &mutating T
```

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
public func cast[T]() -> Pointer[T] where T: not Copyable, T: not Sized, T: not Static
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

### Implements `FFISafe`

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
public struct RcBox[T] where T: not Copyable, T: not Sized, T: not Static { /* private fields */ }
```

Heap allocation with a strong-reference count, used as the underlying
storage for the stdlib's copy-on-write types.

`String`, `Array`, and `Dictionary` all wrap an `RcBox` so that a
plain assignment shares storage and only the first mutating call pays
for a deep copy. Reach for `RcBox` directly when building a similar
COW type; for plain shared ownership without mutation prefer a more
purpose-built container.

`RcBox` is also the `@builtin(.SharedBox)` binding — the box the compiler
instantiates for implicit boxing (escaping closure environments today).
That binding is what promises deterministic last-release cleanup and no
strong-cycle collection; the `SharedBox` protocol itself promises neither.

### Examples

```
let a = RcBox(value: [1, 2, 3]);
let b = a.clone();          // shares storage; refCount == 2
b.mutatingValue.append(4);  // every handle sees it
&a.value;                   // read-only borrow: [1, 2, 3, 4]
```

### Representation

One `Pointer[RcBoxStorage[T]]`. The pointed-to block holds an `Int64`
refcount followed by the `T` value, allocated via `SystemAllocator`.

### Memory Model

Reference-counted, non-atomic (today — see TODOs). `clone()` increments
the count and shares storage; `deinit` decrements and frees on zero.
`deepClone()` allocates a fresh `RcBox` carrying a copied value.

### Guarantees

- `isUnique` being `true` means in-place mutation is safe; this is
  how COW types decide whether to copy.
- The refcount is currently **not** atomic, so `RcBox` is not safe to
  share across threads.
- The payload may be non-`Copyable` (`where T: not Copyable` is a
  relaxation, the same shape `Pointer[T]` uses): implicit boxing needs it,
  because a boxed closure environment owns its captures. The two
  `deepClone()`, which copies the payload out, is therefore only
  available when `T: Copyable`.

`T: not Sized`: an `RcBox` can hold a raw existential — `any P` is
`RcBox[<raw any P>]` by default. Members that move a `T` say
`where T: Sized`; everything else works on any payload. `T: not Static`
for the same reason: a raw existential is only `Static` when its
protocol is, so `RcBox[T]` is `Static` exactly when `T` is.

_Defined in `lang/std/memory/rcbox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(consuming T) where T: Sized
```

Allocates fresh storage holding `value` with refcount 1. Panics if
the underlying `SystemAllocator` returns `.None`.

##### Errors

Panics with `"RcBox allocation failed"` on allocation failure.

_Defined in `lang/std/memory/rcbox.ks`._

#### typealias `Target`

```kestrel
type Target = T
```

_Defined in `lang/std/memory/rcbox.ks`._

#### function `deepClone`

```kestrel
public func deepClone() -> RcBox[T] where T: Copyable
```

Allocates fresh storage with a copy of the value. Used by COW
types when `isUnique` is `false` — splits off a private
copy so the caller can mutate without affecting other clones.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `fork`

```kestrel
public func fork() -> RcBox[T] where T: Cloneable
```

_Defined in `lang/std/memory/rcbox.ks`._

#### field `mutatingValue`

```kestrel
public var mutatingValue: &mutating T { get }
```

_Defined in `lang/std/memory/rcbox.ks`._

#### function `pointeeRef`

```kestrel
public func pointeeRef() -> &T
```

_Defined in `lang/std/memory/rcbox.ks`._

#### field `refCount`

```kestrel
public var refCount: Int64 { get }
```

Current strong reference count. Mostly useful for tests and
diagnostics; production COW logic should branch on `isUnique`.

_Defined in `lang/std/memory/rcbox.ks`._

#### field `value`

```kestrel
public var value: &T { get }
```

The payload, read-only. Writes go through `mutatingValue`, because
every handle onto this storage sees them.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `valuePtr`

```kestrel
internal func valuePtr() -> Pointer[T]
```

Returns a pointer to the wrapped value on the heap. The pointer
is valid as long as the RcBox (and its storage) is alive. Use
this to read individual fields without creating a full `T` clone
whose deinit would free owned resources prematurely.

_Defined in `lang/std/memory/rcbox.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> RcBox[T]
```

Bumps the refcount and returns a second `RcBox` pointing at the
same storage. The receiver and the returned box now both reference
the value; the next mutation should test `isUnique`.

_Defined in `lang/std/memory/rcbox.ks`._

### Implements `MutableIndirection`

#### function `pointeeMutRef`

```kestrel
public mutating func pointeeMutRef() -> &mutating T
```

_Defined in `lang/std/memory/rcbox.ks`._

### Implements `SharedBox`

#### function `isIdentical`

```kestrel
public func isIdentical(to: RcBox[T]) -> Bool
```

Do the two handles point at the same storage block? Compares raw
storage addresses, the same way `Pointer.isEqual(to:)` does — payload
equality is irrelevant, two independently allocated boxes holding equal
values are never identical.

_Defined in `lang/std/memory/rcbox.ks`._

#### field `isUnique`

```kestrel
public var isUnique: Bool { get }
```

`true` when no other clone is sharing storage. The litmus test for
"safe to mutate in place" in COW collections.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `sharedMutRef`

```kestrel
public func sharedMutRef() -> &mutating T
```

_Defined in `lang/std/memory/rcbox.ks`._

## struct `RefSliceIterator`

```kestrel
public struct RefSliceIterator[T] { /* private fields */ }
```

Forward iterator yielding SHARED REFERENCES (`&T`) to contiguous
elements in place — no copies, no clones. The by-reference sibling of
`UnsafeSliceIterator`; surfaced as `Array.refs()`.

### Invalidation

Holds a raw pointer into the underlying buffer. Mutating the source
collection's STRUCTURE while iterating (append/realloc, removal)
invalidates the cursor and any yielded reference — the same contract
as every pointer-backed iterator, met through references here.

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
type Item = &T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[&T]
```

Yields a reference to the next element in place, or `.None` when
the count reaches zero.

_Defined in `lang/std/memory/pointer.ks`._

## protocol `SharedBox`

```kestrel
public protocol SharedBox
```

The contract for a shared-ownership container the compiler may use for
implicit boxing: escaping closure environments, class storage, `any`
payloads, `indirect enum` payloads.

Conformers are *handles*: small, fixed-layout values pointing at managed
storage that owns one `Target`. Duplicating a handle (`clone()`) shares the
storage; dropping the last handle destroys the payload exactly once.
Most of the contract is inherited rather than invented — sharing is
`Cloneable.clone()`, releasing is the conformer's own `deinit`, and reading
the payload is `MutableIndirection.pointeeRef()` (which also supplies
transparent member access, so `box.field` reaches the payload's field).
Only creation, shared mutation, identity and uniqueness are new.

The mechanism-neutral name belongs to this protocol; each conformer says
what it actually does. `RcBox` is the stdlib default (non-atomic
refcounting, deterministic last-release cleanup, no strong-cycle
collection); a future `ArcBox` or `GcBox` would make different promises.
This protocol deliberately promises neither determinism nor cycle
collection — those are properties of the binding, documented there.

### Examples

```
// Generic over any conforming box; uses only SharedBox requirements.
func roundTrip[B](value: Int64) -> Int64 where B: SharedBox, B.Target = Int64 {
    let box: B = B(value);          // init(consuming value: Target)
    let alias = box.clone();        // shares the managed storage
    box.isIdentical(to: alias);     // true — one storage, two handles
    alias.pointeeRef()              // read the payload through the peel
}
```

### Memory Model

A conformer is a handle onto storage that is owned collectively by every
live handle. Lifecycle is ordinary value semantics — no compiler hooks are
involved: the aggregate copy fold routes a struct/enum copy through
`clone()`, so a struct holding a handle becomes `Cloneable` and shares on
copy for free, and the drop machinery releases handles like any other
value.

### Guarantees

- Handles are never bitwise-`Copyable` (the `Cloneable` refinement says
  so): a bit-copied handle would skip the share operation and
  over-release the payload.
- The handle's size and alignment do not depend on `Target`, so a handle
  can be type-erased and can appear in a recursive layout.
- The payload is destroyed exactly once, when the last handle goes away.
- `sharedMutRef()` is the marked exception to value semantics; it is sound
  while Kestrel is single-threaded.

_Defined in `lang/std/memory/sharedbox.ks`._

### Members

#### function `isIdentical`

```kestrel
func isIdentical(to: Self) -> Bool
```

Do the two handles refer to the same managed storage? Backs class
identity (`===`).

Required instead of exposing an address so that a moving collector can
still conform. Handles derived from one `init` — by `clone()` or by the
copy fold — are identical; handles from independent `init`s are not,
however equal their payloads.

_Defined in `lang/std/memory/sharedbox.ks`._

#### field `isUnique`

```kestrel
var isUnique: Bool { get }
```

`true` only when this handle is provably the sole owner. Backs
copy-on-write forking.

Implementations without cheap uniqueness information (a tracing
collector) may conservatively return `false`; callers must therefore
treat `false` as "fork before mutating" rather than as proof of
sharing. A box that always answers `false` degrades to fork-always —
slower, still correct.

_Defined in `lang/std/memory/sharedbox.ks`._

#### function `sharedMutRef`

```kestrel
func sharedMutRef() -> &mutating Target
```

Mutable access to the payload through a *shared*, non-`mutating`
handle — the interior-mutability primitive. Every handle onto the same
storage observes the write; nothing forks and uniqueness is unchanged.

This is the marked exception to value semantics, and the operation
escaping closures and (later) classes are built on. Value-semantic
clients — `any` payloads, `indirect enum`s — must not use it; they
check `isUnique` and fork instead.

##### Safety

Sound only while Kestrel is single-threaded: the returned reference
aliases storage that other handles can read at the same time. Writing
through it while a copy-on-write client assumes value semantics breaks
those semantics for every sharer.

_Defined in `lang/std/memory/sharedbox.ks`._

### Implements `ExistentialBox`

### Implements `Cloneable`

### Implements `MutableIndirection`

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

Allocates storage with the requested alignment. Naturally aligned
layouts use `malloc`; over-aligned layouts use `posix_memalign`.

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

Resizes while preserving the requested alignment. When either layout
is over-aligned, this allocates a new block, copies the common prefix,
and frees the old block only after allocation succeeds.

_Defined in `lang/std/memory/allocator.ks`._

## struct `UniqueBox`

```kestrel
public struct UniqueBox[T]: not Copyable where T: not Copyable { /* private fields */ }
```

Heap allocation with exactly one owner, used for the environment of a
`consuming` (one-shot) closure.

Unlike `RcBox` there is no reference count and no sharing: the single
owner either *takes* the payload out (`takeValue()`, the one call a
`consuming` closure gets) or *destroys* it (`destroy()`, the drop of a
closure that was never called). Exactly one of the two runs, and both
free the block.

This is deliberately NOT a `SharedBox` conformer. A shared box releases by
dropping its whole payload, which would double-free the capture slots a
one-shot body already moved out; the point of the unique box is that the
payload leaves the allocation intact and its per-slot teardown then happens
in the caller's frame, where the ordinary partial-move machinery applies.

### Examples

```
let box = UniqueBox(value: makeResource());
let r = box.takeValue();   // moves the payload out and frees the block
```

### Representation

One `Pointer[UniqueBoxStorage[T]]`. The pointed-to block holds an `Int64`
liveness word followed by the `T` value, allocated via `SystemAllocator`.

### Memory Model

Single owner, no counting. `UniqueBox` has no `deinit` on purpose: it is a
compiler-internal handle whose lifetime is driven by the closure value that
carries it, and dropping the handle value itself must never free the block
(the raw pointer word is forgotten and reconstituted across the closure
representation). Every allocation is released by exactly one `takeValue()`
or `destroy()`.

### Guarantees

- `takeValue()` transfers ownership of the payload to the caller and never
  runs `T.deinit`.
- `destroy()` runs `T.deinit` exactly once when the payload is still
  present, and never when it has already been taken.

_Defined in `lang/std/memory/uniquebox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(consuming T)
```

Allocates fresh storage holding `value`. Panics if the underlying
`SystemAllocator` returns `.None`.

##### Errors

Panics with `"UniqueBox allocation failed"` on allocation failure.

_Defined in `lang/std/memory/uniquebox.ks`._

#### function `destroy`

```kestrel
public func destroy()
```

Drops the payload (if it is still present) and frees the block. This is
the box's single reclamation point — it runs exactly once, whether or
not `takeValue()` emptied the block first.

##### Safety

Must be called exactly once per box, after any `takeValue()`.

_Defined in `lang/std/memory/uniquebox.ks`._

#### function `takeValue`

```kestrel
public func takeValue() -> T
```

Moves the payload out of the block, leaving the block itself alive and
marked empty. The caller becomes the sole owner of the returned value;
`T.deinit` does not run here.

The block is NOT freed: a `consuming` closure's call function takes the
environment out this way, and the closure value's release shim
(`destroy()`) reclaims the now-empty block afterwards. That keeps
exactly one free per allocation whether or not the closure was ever
called.

##### Safety

Must be called at most once per box, and never after `destroy()`.

_Defined in `lang/std/memory/uniquebox.ks`._

## struct `UnsafeSlice`

```kestrel
public struct UnsafeSlice[T] { /* private fields */ }
```

Unchecked view over a contiguous run of `T` values: a pointer and a
count.

The slice does **not** track or extend the lifetime of the underlying
storage, and the compiler doesn't check it: keeping one past its source,
or past the source's next change, is a use-after-free. Use it for FFI,
`Buffer`s and other memory that isn't an `Array`, and inside one call.
To view part of an `Array`, use `xs(1..<3)`, a `&Slice[T]` the
compiler keeps from outliving the array (it is still invalidated when
the array reallocates).

### Examples

```
let arr = [1, 2, 3, 4];
let s = arr.asUnsafeSlice();   // valid until `arr` next changes
s(checked: 0)                  // .Some(1)
s(checked: 99)                 // .None
```

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

#### subscript `subscript`

```kestrel
public subscript[I](I) -> I.SeqOutput where I: SeqIndex[T] { get set }
```

_Defined in `lang/std/collections/sliceable.ks`._

#### subscript `subscript`

```kestrel
public subscript[I](unchecked: I) -> I.SeqOutput where I: SeqIndex[T] { get set }
```

_Defined in `lang/std/collections/sliceable.ks`._

#### subscript `subscript`

```kestrel
public subscript[I](clamped: I) -> I.SeqClampedOutput where I: SeqClampable[T] { get set }
```

_Defined in `lang/std/collections/sliceable.ks`._

#### subscript `subscript`

```kestrel
public subscript[I](wrapped: I) -> I.SeqWrappedOutput where I: SeqWrappable[T] { get set }
```

_Defined in `lang/std/collections/sliceable.ks`._

### Implements `Sliceable`

#### function `asUnsafeSlice`

```kestrel
public func asUnsafeSlice() -> UnsafeSlice[T]
```

Returns `self` — `UnsafeSlice` is already the borrowed view.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = UnsafeSliceIterator[T]
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `iter`

```kestrel
public func iter() -> UnsafeSliceIterator[T]
```

Forward iterator over the elements.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Equatable`

_Conditional: `where T: Equatable`._

### Implements `ArrayMatchable`

#### typealias `Element`

```kestrel
type Element = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### typealias `Rest`

```kestrel
type Rest = UnsafeSlice[T]
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchGet`

```kestrel
public func matchGet(Int64) -> T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchLength`

```kestrel
public func matchLength() -> Int64
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchSlice`

```kestrel
public func matchSlice(Int64, Int64) -> UnsafeSlice[T]
```

_Defined in `lang/std/memory/pointer.ks`._

## struct `UnsafeSliceIterator`

```kestrel
public struct UnsafeSliceIterator[T] { /* private fields */ }
```

Forward iterator over an `UnsafeSlice[T]`. Holds a moving pointer and a
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

## function `swap`

```kestrel
public func swap[T](mutating T, mutating T) where T: not Copyable, T: not Static
```

Exchanges the contents of two mutable locations without cloning or
dropping either — three bitwise moves, like Rust's `mem::swap`. For a
COW value (`Array`, `String`, `Dictionary`) this swaps the *handles*, so
it never touches the heap buffers: an `O(1)` pointer exchange regardless
of contents.

This is the idiomatic way to rotate double buffers (`swap(a, b)` instead
of the `let tmp = a; a = b; b = tmp` dance, which clones for non-`Copyable`
types).

### Examples

```
var a = [1, 2, 3];
var b = [9, 8];
swap(a, b);          // a == [9, 8], b == [1, 2, 3] — no element copies
```

_Defined in `lang/std/memory/pointer.ks`._

