// test: diagnostics
// stdlib: true

// Without `and not Static`, an opaque param keeps the implicit `Static`
// bound, so a borrowing view is rejected.

module Test

import std.collections.(Array, Slice)
import std.memory.(UnsafeSlice, UnsafeSliceIterator)

// `Lends` stands in for the removed std `Sliceable`: a protocol whose
// conformers lend their elements as a `Slice`.
protocol Lends[T] { func asSlice() -> &Slice[T] }
extend Array[T, A]: Lends[T] { }

// A borrowing view: it holds a reference, so it is not `Static`.
struct Borrowed { let base: &Array[Int64] }

extend Borrowed: Lends[Int64], Iterable {
    type Item = Int64
    type TargetIterator = UnsafeSliceIterator[Int64]
    func asSlice() -> &Slice[Int64] { self.base.asSlice() }
    func iter() -> UnsafeSliceIterator[Int64] { self.base.asUnsafeSlice().iter() }
}

func count(xs: some Lends[Int64]) -> Int64 { xs.asSlice().asUnsafeSlice().count }

func caller() -> Int64 {
    let xs = [1, 2, 3];
    let b = &xs;
    count(Borrowed(base: b)) // ERROR: !: Static
}
