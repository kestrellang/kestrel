// test: execution
// stdlib: true
// expect-exit: 0
//
// `x is T` is typed like a boolean literal: any `ExpressibleByBoolLiteral`
// type, defaulting to Bool (design §8b).

module Test

import std.numeric.(Int64)
import std.core.(BooleanConditional, ExpressibleByBoolLiteral)

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

struct Flag {
    var raw: lang.i1;
}
extend Flag: ExpressibleByBoolLiteral {
    public init(boolLiteral value: lang.i1) { self.raw = value; }
}
extend Flag: BooleanConditional {
    public func boolValue() -> lang.i1 { self.raw }
}

@main
func main() -> Int64 {
    let s: any Shape = Circle(r: 1);
    let f: Flag = s is Circle;
    if f { } else { return 1 }
    let g: Flag = s is Square;
    if g { return 2 }
    0
}
