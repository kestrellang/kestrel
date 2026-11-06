// test: execution
// stdlib: true
// expect-stdout: 1 3 0 | 10 2 30 | 4 | 10 5 6 | 2 0 | 1 9 | 7 7\n

// `xs(clamped: i)` is a failable place that saturates `i` into
// `0..<count` (`.None` only when empty); `xs(within: r)` lends the part of
// `r` inside the array, readable and writable in place. A write through
// either place on a copy runs the copy-on-write check first.
module Test

import std.collections.(Array, Slice)

func bumpAll(mutating s: Slice[Int64]) {
    for i in 0..<s.count { s(i) += 4; }
}

@main
func main() -> lang.i64 {
    var xs = [1, 2, 3];
    let empty: [Int64] = [];
    print("\(xs(clamped: -5) ?? 0) \(xs(clamped: 99) ?? 0) \(empty(clamped: 0) ?? 0)");
    if let .Some(&mutating v) = xs(clamped: -1) { v = 10; }
    if let .Some(&mutating v) = xs(clamped: 7) { v *= 10; }
    print(" | \(xs(0)) \(xs(1)) \(xs(2))");
    let w = xs(within: 1..<99);
    print(" | \(w.count + 2)");
    bumpAll(xs(within: 1..<99));
    print(" | \(xs(0)) \(xs(1) - 1) \(xs(2) - 28)");
    print(" | \(xs(within: -3..<2).count) \(xs(within: 5..=9).count)");
    var ys = [1, 2];
    let copy = ys;
    if let .Some(&mutating v) = ys(clamped: 5) { v = 9; }
    print(" | \(copy(0)) \(ys(1))");
    var s = [7, 8];
    if let .Some(&mutating v) = s.asMutableSlice()(clamped: 9) { v = 7; }
    print(" | \(s(0)) \(s(1))\n");
    0
}
