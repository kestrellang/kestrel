// LiteralSlice - read-only view into compiler-generated array literal data

module std.memory

import std.core.(Bool)
import std.num.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

/// Iterator yielded by `LiteralSlice.iter()`. Walks the backing buffer
/// element-by-element, advancing a raw pointer.
///
/// # Representation
///
/// A raw pointer plus a primitive-typed remaining count. No `Slice`
/// indirection — the iterator is what `LiteralSlice` hands out instead of
/// exposing its raw pointer directly.
public struct LiteralSliceIterator[T]: Iterator {
    type Item = T

    private var ptr: lang.ptr[T]
    private var remaining: lang.i64

    /// @name From Storage
    /// Builds an iterator from the raw pointer and length the compiler
    /// hands to a literal init. Not normally called by user code.
    public init(ptr ptr: lang.ptr[T], remaining remaining: lang.i64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Yields the next element, or `.None` once the buffer is exhausted.
    public mutating func next() -> T? {
        if lang.i64_signed_gt(self.remaining, 0) {
            let value = lang.ptr_read(self.ptr);
            self.ptr = lang.ptr_offset[T](self.ptr, lang.sizeof[T]());
            self.remaining = lang.i64_sub(self.remaining, 1);
            .Some(value)
        } else {
            .None
        }
    }
}

/// Read-only view over the compiler-emitted backing buffer for an array
/// literal.
///
/// User code rarely names this type directly: it appears in
/// `ExpressibleByArrayLiteral.init(arrayLiteral:)` and friends so that
/// types accepting `[a, b, c]` literals can iterate the elements without
/// touching raw pointers. The slice does **not** own the storage — the
/// compiler keeps the literal alive for the duration of the call.
///
/// # Examples
///
/// ```
/// // Conforming to ExpressibleByArrayLiteral
/// public struct MyVec[T]: ExpressibleByArrayLiteral {
///     type Element = T
///     public init(arrayLiteral lit: LiteralSlice[T]) {
///         var v = MyVec();
///         for x in lit { v.push(x) }
///         self = v
///     }
/// }
/// ```
///
/// # Memory Model
///
/// Non-owning. The backing storage is compiler-managed and lives for the
/// scope of the literal expression. Capturing a `LiteralSlice` past that
/// scope is a use-after-free.
public struct LiteralSlice[T]: Iterable {
    type Item = T
    type Iter = LiteralSliceIterator[T]

    private var ptr: lang.ptr[T]
    private var len: lang.i64

    /// @name From Storage
    /// Builds the slice from the raw pointer and count the compiler emits.
    public init(pointer pointer: lang.ptr[T], count count: lang.i64) {
        self.ptr = pointer;
        self.len = count;
    }

    /// Number of elements in the literal.
    public func count() -> Int64 { Int64(intLiteral: self.len) }

    /// Returns `true` for `[]`.
    public func isEmpty() -> Bool { Bool(boolLiteral: lang.i64_eq(self.len, 0)) }

    /// Iterator over the elements in source order.
    public func iter() -> LiteralSliceIterator[T] {
        LiteralSliceIterator(ptr: self.ptr, remaining: self.len)
    }

    /// @name Unchecked Index
    /// Reads element `index` without bounds checking. The compiler-emitted
    /// init paths that use this guarantee the index is in range; do not
    /// expose this subscript to user input without checking `count` first.
    public subscript(unchecked index: Int64) -> T {
        get {
            let offset = lang.i64_mul(index.raw, lang.sizeof[T]());
            lang.ptr_read(lang.ptr_offset[T](self.ptr, offset))
        }
    }
}
