// test: execution
// stdlib: true
// expect-stdout: 2 0 30 6 2 -1 10 10 | 7 0\n

// `xs(checked: i)` is a failable place: reads lend `Optional[&T]` (a
// binding decays it to `Optional[T]`), and a `&mutating` binder writes
// through it. Range indices lend `Optional[&Slice[T]]`.
module Test

import std.collections.(Array, Slice)

struct P { var x: Int64 }

func bump(mutating s: Slice[Int64]) {
    if let .Some(&mutating v) = s(checked: 0) { v += 5; }
    if let .Some(&mutating v) = s(checked: 9) { v = 99; }
}

@main
func main() -> lang.i64 {
    var xs = [1, 2, 3];
    let a = xs(checked: 1);
    let b = xs(checked: 9);
    if let .Some(&mutating v) = xs(checked: 2) { v = 30; }
    var ps = [P(x: 1)];
    if let .Some(&mutating p) = ps(checked: 0) { p.x += 5; }
    let s = xs(checked: 0..<2);
    let n = match s { .Some(r) => r.count, .None => -1 };
    let m = match xs(checked: 2..<9) { .Some(r) => r.count, .None => -1 };
    if let .Some(&mutating r) = xs(checked: 0..<2) { r(0) = 10; }
    let c: Int64 = match xs(checked: 0) { .Some(v) => v, .None => 0 };
    print("\(a ?? 0) \(b ?? 0) \(xs(2)) \(ps(0).x) \(n) \(m) \(xs(0)) \(c)");
    var ys = [2, 0];
    bump(ys(0..<2));
    print(" | \(ys(0)) \(ys(1))\n");
    0
}
