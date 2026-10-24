// test: diagnostics
// stdlib: true
//
// E153: an `is` binding borrows the payload, so it can't be `var`
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
        var c is Circle => c.r, // ERROR: an 'is' pattern binds a borrow of the payload, so it cannot be 'var'
        _ => 0,
    }
}
