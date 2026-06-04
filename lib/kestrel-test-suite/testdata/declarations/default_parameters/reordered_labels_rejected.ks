// test: diagnostics
// stdlib: false

// Skipping defaulted parameters is allowed, but REORDERING labeled arguments is
// not — arguments must appear in declaration order. Here `z` then `x` is out of
// order, so binding fails. Guards against the binder accepting permutations.
module Main

struct Point {
    public init() {}
    public func make(x: lang.i64 = 0, y: lang.i64 = 0, z: lang.i64 = 0) -> lang.i64 { x }
}

func test() -> lang.i64 {
    let p = Point();
    p.make(z: 1, x: 2) // ERROR: wrong argument label
}
