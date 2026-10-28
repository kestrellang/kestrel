// test: diagnostics
// stdlib: true
//
// A plain struct pattern is not a type test on an existential: type tests
// always say `is`, so this is the ordinary struct-pattern type mismatch
// (design §8b).

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

func test(s: any Shape) -> Int64 {
    match s {
        Circle { r } => r, // ERROR: type mismatch
        _ => 0,
    }
}
