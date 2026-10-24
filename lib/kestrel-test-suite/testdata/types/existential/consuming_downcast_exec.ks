// test: execution
// stdlib: true
// expect-exit: 0
//
// `consuming x as? T` on `any[UniqueBox] P` returns `Result[T, any[B] P]`:
// a hit moves the value out, a miss hands the existential back intact
// (design §8).

module Test

import std.numeric.(Int64)
import std.memory.(UniqueBox)

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

@main
func main() -> Int64 {
    let a: any[UniqueBox] Shape = Circle(r: 2);
    match consuming a as? Circle {
        .Ok(c) => { if c.r != 2 { return 1 } },
        .Err(_) => { return 2 },
    }
    let b: any[UniqueBox] Shape = Square(side: 3);
    match consuming b as? Circle {
        .Ok(_) => { return 3 },
        .Err(back) => { if back.area() != 9 { return 4 } },
    }
    0
}
