// test: diagnostics
// stdlib: true

// Without `and not Static`, an opaque param keeps the implicit `Static`
// bound, so a borrowing view is rejected.

module Test

import std.collections.(Array, Slice, SliceIndex, SliceClampable, Sliceable)
import std.memory.(UnsafeSlice, UnsafeSliceIterator)

// A borrowing view: it holds a reference, so it is not `Static`.
struct Borrowed { let base: &Array[Int64] }

extend Borrowed: Sliceable[Int64], Iterable {
    type Item = Int64
    type TargetIterator = UnsafeSliceIterator[Int64]
    func asSlice() -> &Slice[Int64] { self.base.asSlice() }
    subscript[I](index: I) -> I.SliceOutput where I: SliceIndex[Int64] { ref { self.base(index) } }
    subscript[I](checked index: I)? -> I.SliceOutput where I: SliceIndex[Int64] { ref { self.base(checked: index) } }
    subscript[I](unchecked index: I) -> I.SliceOutput where I: SliceIndex[Int64] { ref { self.base(unchecked: index) } }
    subscript(clamped index: Int64)? -> Int64 { ref { self.base(clamped: index) } }
    subscript[R](within range: R) -> Slice[Int64] where R: SliceClampable[Int64] { ref { self.base(within: range) } }
    subscript(wrapped index: Int64)? -> Int64 { ref { self.base(wrapped: index) } }
    func iter() -> UnsafeSliceIterator[Int64] { self.base.asUnsafeSlice().iter() }
}

func count(xs: some Sliceable[Int64]) -> Int64 { xs.asUnsafeSlice().count }

func caller() -> Int64 {
    let xs = [1, 2, 3];
    let b = &xs;
    count(Borrowed(base: b)) // ERROR: !: Static
}
