// LiteralSlice - read-only view into compiler-generated array literal data

module std.memory

import std.core.(Bool)
import std.num.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

// Iterator for LiteralSlice - must be defined before LiteralSlice for Iterable conformance
public struct LiteralSliceIterator[T]: Iterator {
    type Item = T

    private var ptr: lang.ptr[T]
    private var remaining: lang.i64

    public init(ptr ptr: lang.ptr[T], remaining remaining: lang.i64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    public mutating func next() -> Optional[T] {
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

// Read-only wrapper around compiler-generated array literal data
// This provides a safe, iterable view over the literal elements
public struct LiteralSlice[T]: Iterable {
    type Item = T
    type Iter = LiteralSliceIterator[T]

    private var ptr: lang.ptr[T]
    private var len: lang.i64

    public init(pointer pointer: lang.ptr[T], count count: lang.i64) {
        self.ptr = pointer;
        self.len = count;
    }

    public func count() -> Int64 { Int64(intLiteral: self.len) }

    public func isEmpty() -> Bool { Bool(boolLiteral: lang.i64_eq(self.len, 0)) }

    public func iter() -> LiteralSliceIterator[T] {
        LiteralSliceIterator(ptr: self.ptr, remaining: self.len)
    }
}
