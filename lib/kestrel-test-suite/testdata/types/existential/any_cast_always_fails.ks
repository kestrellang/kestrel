// test: diagnostics
// stdlib: true
//
// E149 (warning): a cast or type test whose target can't conform to the
// existential's protocol always fails. One check covers `as?`, the `is`
// operator, and `is` patterns (design §8, §8b).

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
    let a = s as? Int64; // WARNING: cast from 'any Shape' to 'Int64' always fails: 'Int64' does not conform to 'Shape'
    let b = s is Int64; // WARNING: cast from 'any Shape' to 'Int64' always fails: 'Int64' does not conform to 'Shape'
    match s {
        is Int64 => 1, // WARNING: cast from 'any Shape' to 'Int64' always fails: 'Int64' does not conform to 'Shape'
        _ => 0,
    }
}
