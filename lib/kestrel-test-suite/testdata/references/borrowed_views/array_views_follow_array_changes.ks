// test: execution
// stdlib: true
// expect-exit: 0

// Array views read through the array on every access, so they see writes
// made after they were created, and growing the array (which moves its
// buffer) doesn't leave them reading freed memory.

module Test

import std.collections.(Array)

@main
func main() -> lang.i64 {
    var xs = [1, 2, 3, 4];
    let rev = xs.reversed;
    let chunks = xs.chunks(of: 2);
    xs(0) = 10;
    for i in 0..<100 { xs.append(i); }
    if rev(3) != 10 { return 1 }
    if chunks(0)(0) != 10 { return 2 }
    var total: Int64 = 0;
    for x in xs.refs() { total += x; }
    if total != 10 + 2 + 3 + 4 + 4950 { return 3 }
    var ys = [1, 2, 3];
    let copy = ys;
    for y in ys.mutatingRefs() { y *= 2; }
    if ys(2) != 6 { return 4 }
    if copy(2) != 3 { return 5 }
    0
}
