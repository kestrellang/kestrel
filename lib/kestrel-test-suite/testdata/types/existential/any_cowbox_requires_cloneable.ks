// test: diagnostics
// stdlib: true
//
// `CowBox[T]` requires `T: Cloneable`, so `any[CowBox] P` needs
// `P: Cloneable`. The error comes from CowBox's own bound, not from an
// existential-specific rule (design §3).

module Test

import std.numeric.(Int64)
import std.memory.(CowBox)

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

func test() {
    let s: any[CowBox] Shape = Circle(r: 1); // ERROR: type mismatch: does not conform to protocol; does not satisfy constraint
}
