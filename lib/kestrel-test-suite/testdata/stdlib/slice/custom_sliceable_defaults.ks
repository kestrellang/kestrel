// test: execution
// stdlib: true
// expect-stdout: 20 2 -1 30 2 30 | 20 2\n

// A custom `Sliceable` only supplies `asSlice()` (and iteration): the six
// index places come from the protocol extension's defaults, both on the
// type directly and through generic code.
module Test

import std.collections.(Array, Slice, Sliceable)
import std.memory.(UnsafeSliceIterator)

struct View { let base: &Array[Int64] }

extend View: Sliceable[Int64], Iterable {
    type Item = Int64
    type TargetIterator = UnsafeSliceIterator[Int64]
    func asSlice() -> &Slice[Int64] { self.base.asSlice() }
    func iter() -> UnsafeSliceIterator[Int64] { self.base.asUnsafeSlice().iter() }
}

func second[S](s: &S) -> Int64 where S: Sliceable[Int64], S: not Static, S: not Copyable, S: not Sized {
    s(1)
}

@main
func main() -> lang.i64 {
    let xs = [10, 20, 30];
    let r = &xs;
    let v = View(base: r);
    let c = v(checked: 9) ?? -1;
    print("\(v(1)) \(v(1..<3).count) \(c) \(v(clamped: 99) ?? 0) \(v(within: 1..<99).count) \(v(wrapped: -1) ?? 0)");
    print(" | \(second(v)) \(second(xs(1..<3)) - 28)\n");
    0
}
