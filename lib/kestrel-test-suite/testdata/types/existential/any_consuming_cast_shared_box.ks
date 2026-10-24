// test: diagnostics
// stdlib: true
//
// A consuming `as?` needs a `ConsumingIndirection` box (E148). The default
// RcBox can't give up a payload other copies may still see (design §8).

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

func test() {
    let s: any Shape = Circle(r: 1);
    let c = consuming s as? Circle; // ERROR: cannot consume the payload of 'any Shape': 'RcBox' shares its payload, so it cannot give it up; use 'any[OwnedBox] Shape'
}
