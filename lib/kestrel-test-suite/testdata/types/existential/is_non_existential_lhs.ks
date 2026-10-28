// test: diagnostics
// stdlib: true
//
// E152: `is` needs an existential on the left. A concrete type is known at
// compile time; a generic parameter is rejected for now (design §8b).

module Test

import std.core.(Bool)
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

func concrete(c: Circle) -> Bool {
    c is Circle // ERROR: 'is' needs an existential on the left: 'Circle' is known at compile time
}

func generic[T](x: T) -> Bool {
    x is Circle // ERROR: 'is' needs an existential on the left: 'T' is a generic parameter
}
