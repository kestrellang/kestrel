// test: execution
// stdlib: true
// expect-stdout: 1212 1212 1212 | 7 12 2\n

// Generic code over `Sliceable` lends through the required index places
// (`s(i)`, ranges, `checked:`, `clamped:`, `within:`) on every conformer,
// and an `UnsafeSlice` writes through its own places.
module Test

import std.collections.(Array, Slice, Sliceable)
import std.memory.(UnsafeSlice)

struct P { var x: Int64 }

func probe[S](s: &S) -> Int64 where S: Sliceable[P], S: not Static, S: not Copyable, S: not Sized {
    let a = s(0).x;
    let r = s(1..<3);
    let c = match s(checked: 9) { .Some(p) => p.x, .None => -1 };
    let d = s(clamped: 99) ?? P(x: 0);
    let w = s(within: 2..<99).count;
    a * 1000 + r.count * 100 + w * 10 + d.x + c
}

@main
func main() -> lang.i64 {
    var xs = [P(x: 1), P(x: 2), P(x: 3)];
    print("\(probe(xs)) \(probe(xs.asSlice())) \(probe(xs.asUnsafeSlice()))");
    var u = xs.asUnsafeSlice();
    u(0) = P(x: 7);
    u(1).x += 10;
    let part = u(1..<3);
    print(" | \(xs(0).x) \(xs(1).x) \(part.count)\n");
    0
}
