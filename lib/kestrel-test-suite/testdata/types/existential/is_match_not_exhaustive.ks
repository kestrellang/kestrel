// test: diagnostics
// stdlib: true
//
// The set of payload types is open, so a match on an existential needs `_`.
// A repeated `is` arm is unreachable (design §8b).

module Test

import std.numeric.(Int64)

protocol Shape {
    func area() -> Int64
}

struct Circle { var r: Int64; }
extend Circle: Shape {
    public func area() -> Int64 { 3 * self.r * self.r }
}

struct Square { var side: Int64; }
extend Square: Shape {
    public func area() -> Int64 { self.side * self.side }
}

func missing(s: any Shape) -> Int64 {
    match s { // ERROR: non-exhaustive match: missing _
        is Circle => 1,
        is Square => 2,
    }
}

func repeated(s: any Shape) -> Int64 {
    match s {
        is Circle => 1,
        is Circle => 2, // WARNING: unreachable pattern
        _ => 0,
    }
}
