// test: execution
// stdlib: true
// expect-stdout: 20 2 -1 30 2 30 | 20 2\n

// A custom view that is an `Indirection` to `Slice` gets the six index
// places through the peel, both on the type directly and when passed to a
// borrowed `&Slice[T]` param.
module Test

import std.collections.(Array, Slice)
import std.core.(Indirection)

struct View { let base: &Array[Int64] }

extend View: Indirection {
    type Target = Slice[Int64]
    var value: Slice[Int64] { ref { self.base.asSlice() } }
}

func second(s: &Slice[Int64]) -> Int64 {
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
