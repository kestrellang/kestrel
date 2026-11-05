// test: execution
// stdlib: true
// expect-exit: 0

// `some P and not Static` in parameter position relaxes the implicit
// `Static` bound, like `where T: not Static` on a named generic: a
// borrowing view is accepted, and a defaulted argument still
// type-checks (the default's annotation used to reject `not Static`, E140).

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

func total(xs: some Sliceable[Int64] and not Static) -> Int64 {
    var t: Int64 = 0;
    for x in xs.asUnsafeSlice() { t += x; }
    t
}

func totalOr(xs: some Sliceable[Int64] and not Static = Array[Int64]()) -> Int64 {
    total(xs)
}

@main
func main() -> lang.i64 {
    let xs = [1, 2, 3, 4];
    let ys = [4, 5];
    let bx = &xs;
    let by = &ys;
    if total(xs) != 10 { return 1 }
    if total(Borrowed(base: bx)) != 10 { return 2 }   // borrowing view: not Static
    if totalOr(Borrowed(base: by)) != 9 { return 3 }
    if totalOr() != 0 { return 4 }          // default argument
    0
}
