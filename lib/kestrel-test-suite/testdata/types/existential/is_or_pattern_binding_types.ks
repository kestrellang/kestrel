// test: diagnostics
// stdlib: true
//
// `c is Circle or c is Square` binds `c` at two different types (E154).
// Today no or-pattern check compares binding types (`.A(v) or .B(v)` with
// different payload types is accepted at 80297742), so this is a new rule
// the design adds. Without a binding it is fine (design §8b).

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

func bad(s: any Shape) -> Int64 {
    match s {
        c is Circle or c is Square => 1, // ERROR: variable 'c' has different types in or-pattern alternatives
        _ => 0,
    }
}

func good(s: any Shape) -> Int64 {
    match s {
        is Circle or is Square => 1,
        _ => 0,
    }
}
