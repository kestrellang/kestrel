// test: execution
// stdlib: true
// expect-exit: 0

// A bare `_` argument leaves that slot open: the call becomes a closure
// with one parameter per hole, left to right.

module Test

func add(a: Int64, b: Int64) -> Int64 { a + b }
func sub(a: Int64, b: Int64) -> Int64 { a - b }
func clamp(x: Int64, min lo: Int64, max hi: Int64) -> Int64 {
    if x < lo { return lo }
    if x > hi { return hi }
    x
}

@main
func main() -> lang.i64 {
    let addOne = add(_, 1);
    if addOne(4) != 5 { return 1 }
    // two holes, in order
    let minus = sub(_, _);
    if minus(10, 3) != 7 { return 2 }
    // second slot
    let fromTen = sub(10, _);
    if fromTen(4) != 6 { return 3 }
    // labeled hole
    let atMost = clamp(_, min: 0, max: _);
    if atMost(300, 255) != 255 { return 4 }
    let byte = clamp(_, min: 0, max: 255);
    if byte(-3) != 0 { return 5 }
    // passed straight to a higher-order function
    let xs = [1, 2, 3];
    let ys = xs.map(add(_, 10));
    if ys(0) != 11 or ys(2) != 13 { return 6 }
    0
}
