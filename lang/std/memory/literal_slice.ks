// LiteralSlice - read-only view into compiler-generated array literal data

module std.memory

import std.core.(Bool, fatalError)
import std.num.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.memory.(Pointer)

/// Iterator yielded by `LiteralSlice.iter()`. Walks the backing buffer
/// element-by-element, advancing a typed pointer.
///
/// # Representation
///
/// A `Pointer[T]` plus a remaining count. No `Slice` indirection — the
/// iterator is what `LiteralSlice` hands out instead of exposing its
/// pointer directly.
public struct LiteralSliceIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int64

    /// @name From Storage
    /// Builds an iterator from a typed pointer and element count.
    /// Not normally called by user code.
    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Yields the next element, or `.None` once the buffer is exhausted.
    public mutating func next() -> T? {
        if self.remaining > 0 {
            let value = self.ptr.read();
            self.ptr = self.ptr.offset(by: 1);
            self.remaining = self.remaining - 1;
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

    private var ptr: Pointer[T]
    private var len: Int64

    /// @name From Storage
    /// Builds the slice from the raw pointer and count the compiler emits.
    public init(pointer pointer: lang.ptr[T], count count: lang.i64) {
        self.ptr = Pointer(raw: pointer);
        self.len = Int64(intLiteral: count);
    }

    /// Number of elements in the literal.
    public func count() -> Int64 { self.len }

    /// Returns `true` for `[]`.
    public func isEmpty() -> Bool { self.len == 0 }

    /// Iterator over the elements in source order.
    public func iter() -> LiteralSliceIterator[T] {
        LiteralSliceIterator(ptr: self.ptr, remaining: self.len)
    }

    /// @name Indexed
    /// Reads element `index`, panicking on out-of-bounds.
    ///
    /// The default subscript: trades a single comparison for a guaranteed
    /// trap on bad input. Use `(unchecked:)` inside compiler-emitted init
    /// paths where the index is statically known in range, or
    /// `(checked:)` to handle out-of-range without a panic.
    ///
    /// # Errors
    ///
    /// Panics with `"LiteralSlice index out of bounds"` if `index < 0`
    /// or `index >= count`.
    public subscript(index: Int64) -> T {
        get {
            if index < 0 or index >= self.len {
                fatalError("LiteralSlice index out of bounds")
            }
            self.ptr.offset(by: index).read()
        }
    }

    /// @name Checked Index
    /// Reads element `index`, returning `.None` on out-of-bounds.
    public subscript(checked index: Int64) -> T? {
        get {
            if index < 0 or index >= self.len {
                .None
            } else {
                .Some(self.ptr.offset(by: index).read())
            }
        }
    }

    /// @name Unchecked Index
    /// Reads element `index` without bounds checking.
    ///
    /// # Safety
    ///
    /// Undefined behavior if `index < 0` or `index >= count`. Compiler-
    /// emitted init paths that use this guarantee the index is in range;
    /// do not expose this subscript to user input without checking
    /// `count` first.
    public subscript(unchecked index: Int64) -> T {
        get {
            self.ptr.offset(by: index).read()
        }
    }
}
