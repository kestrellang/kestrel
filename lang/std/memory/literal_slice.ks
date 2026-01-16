// LiteralSlice - read-only view into compiler-generated array literal data

module std.memory

import std.core.(Int, Bool)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

// Read-only wrapper around compiler-generated array literal data
// This provides a safe, iterable view over the literal elements
public struct LiteralSlice[T]: Iterable {
    type Item = T
    type Iter = LiteralSliceIterator[T]

    private var ptr: lang.ptr[T]
    private var len: lang.i64

    public init(pointer: lang.ptr[T], count: lang.i64) {
        self.ptr = pointer
        self.len = count
    }

    public var count: Int { Int(self.len) }

    public var isEmpty: Bool { self.len == 0 }

    public func iter() -> LiteralSliceIterator[T] {
        LiteralSliceIterator(ptr: self.ptr, remaining: self.len)
    }
}

public struct LiteralSliceIterator[T]: Iterator {
    type Item = T

    private var ptr: lang.ptr[T]
    private var remaining: lang.i64

    public init(ptr: lang.ptr[T], remaining: lang.i64) {
        self.ptr = ptr
        self.remaining = remaining
    }

    public mutating func next() -> Optional[T] {
        if self.remaining > 0 {
            let value = lang.ptr_read(self.ptr)
            self.ptr = lang.ptr_offset(self.ptr, Int(lang.sizeof[T]()))
            self.remaining = self.remaining - 1
            .Some(value)
        } else {
            .None
        }
    }
}
