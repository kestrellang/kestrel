// test: diagnostics
// stdlib: true
//
// The box in `any[B] P` is an unapplied generic type (E404).

module Test

import std.numeric.(Int64)
import std.memory.(RcBox)

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

func f(s: any[RcBox[Int64]] Shape) {} // ERROR: the box in 'any[...]' is written without type arguments: 'any[RcBox] Shape'
