# std2 Migration Order

Move files in the order below. Build after each file/group to catch errors early.

---

## Phase 1: Operator Protocols (Zero Dependencies)

These define operator protocols that everything else implements.

```
ops/arithmetic.ks    # Addable, Subtractable, Multipliable, Divisible, Modulo, Negatable
ops/comparison.ks    # Equal, NotEqual, Less, LessOrEqual, Greater, GreaterOrEqual
ops/bitwise.ks       # BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, LeftShift, RightShift
ops/logical.ks       # And, Or, Not, BooleanConditional
ops/assign.ks        # Assignment operators
ops/copy.ks          # NonCopyable
```

## Phase 2: FFI Layer

External function interface (depends only on lang intrinsics).

```
ffi/ffi.ks           # FFISafe protocol marker
ffi/libc.ks          # malloc, free, realloc, memcpy, memmove, memset
```

## Phase 3: Error/Control Flow Protocols

Defines error handling protocols (before Optional/Result types).

```
result/error.ks      # Error protocol, ControlFlow, Tryable, FromResidual
```

## Phase 4: Core Protocols

Semantic protocols that types conform to.

```
core/protocols.ks    # Equatable, Comparable, Hashable, Hasher, DefaultHasher, Cloneable, Copyable, Defaultable
core/ordering.ks     # Ordering enum (Less, Equal, Greater)
core/numeric.ks      # Numeric, Integer, SignedInteger, UnsignedInteger, FloatingPoint, Steppable
```

## Phase 5: Bool

Boolean type (depends on ops, ffi, core protocols).

```
core/bool.ks         # Bool struct
```

## Phase 6: Integer Types

Signed and unsigned integers. Can be done in parallel within group.

```
core/int8.ks         # Int8
core/int16.ks        # Int16
core/int32.ks        # Int32
core/int64.ks        # Int64 (also defines Int alias)
core/uint8.ks        # UInt8
core/uint16.ks       # UInt16
core/uint32.ks       # UInt32
core/uint64.ks       # UInt64 (also defines UInt alias)
```

## Phase 7: Float Types

Floating point types.

```
core/float32.ks      # Float32
core/float64.ks      # Float64 (also defines Float alias)
```

## Phase 8: Memory Layout

Memory layout information (depends on core types).

```
memory/layout.ks     # Layout struct
```

## Phase 9: Pointer Types

Raw and typed pointer abstractions.

```
memory/pointer.ks    # RawPointer, Pointer[T], Slice[T], SliceIterator[T]
```

## Phase 10: Iterator Protocol

Core iteration protocol (depends on Optional which isn't moved yet - may need stub).

```
iter/iterator.ks     # Iterator, Iterable, Collectable protocols
```

## Phase 11: Optional Type

Maybe-present value (depends on iterator).

```
result/optional.ks   # Optional[T] enum, OptionalIterator[T]
```

## Phase 12: Result Type

Success/error value (depends on Optional).

```
result/result.ks     # Result[T, E] enum, ResultIterator[T, E]
```

## Phase 13: Allocator

Memory allocation protocol and implementations.

```
memory/allocator.ks  # Allocator protocol, SystemAllocator, GlobalAllocator
```

## Phase 14: Buffer

Owning memory buffer (depends on allocator, pointer).

```
memory/buffer.ks     # Buffer[T, A], ArcBox[T]
```

## Phase 15: Literal Slice

Compiler-generated immutable slice (for array literals).

```
memory/literal_slice.ks  # LiteralSlice[T], LiteralSliceIterator[T]
```

## Phase 16: Literals Protocol

Literal expressibility protocols (depends on LiteralSlice).

```
ops/literals.ks      # ExpressibleByIntLiteral, ExpressibleByArrayLiteral, etc.
```

## Phase 17: Range Types

Range and closed range with iteration.

```
ops/range.ks         # Range[T], ClosedRange[T], RangeIterator, ClosedRangeIterator
```

## Phase 18: Iterator Adapters

Iterator combinators (map, filter, take, etc.).

```
iter/adapters.ks     # MapIterator, FilterIterator, TakeIterator, etc.
iter/extensions.ks   # Extension methods on Iterator
```

## Phase 19: Array

Dynamic growable array (depends on buffer, iterator, literal slice).

```
collections/array.ks # Array[T, A], ArrayIterator
```

## Phase 20: Text - Char

Unicode code point and character.

```
text/char.ks         # Byte, CodePoint, Char, decodeUtf8()
```

## Phase 21: Text - String

UTF-8 string with COW semantics.

```
text/string.ks       # String[A], SplitIterator
```

## Phase 22: Text - Views

String views and iterators.

```
text/views.ks        # BytesView, CodePointsView, CharsView, LinesView
```

## Phase 23: Dictionary

Hash map (depends on buffer, hashable).

```
collections/dictionary.ks  # Dictionary[K, V, A], DictionaryIterator
```

## Phase 24: Set

Hash set (implemented on top of Dictionary).

```
collections/set.ks   # Set[T, A], SetIterator
```

---

## Dependency Graph Summary

```
         [ops protocols]
               |
         [ffi/libc]
               |
       [result/error]
               |
    [core protocols + ordering + numeric]
               |
           [bool]
               |
    [int types] [uint types] [float types]
               |
        [memory/layout]
               |
       [memory/pointer]
               |
      [iter/iterator]
               |
    [result/optional]
               |
     [result/result]
               |
     [memory/allocator]
               |
      [memory/buffer]
               |
   [memory/literal_slice]
               |
       [ops/literals]
               |
        [ops/range]
               |
     [iter/adapters]
               |
   [collections/array]
               |
        [text/char]
               |
       [text/string]
               |
       [text/views]
               |
  [collections/dictionary]
               |
    [collections/set]
```

## Notes

- Build after each phase to catch issues early
- Some phases may reveal circular dependencies that need resolution
- The errors.md file shows existing issues - many relate to:
  - Missing `BooleanConditional` conformance
  - Type mismatches between integer types
  - Methods not found on generic type parameters
  - Mutable field assignments
- Consider fixing foundational issues in early phases before proceeding
