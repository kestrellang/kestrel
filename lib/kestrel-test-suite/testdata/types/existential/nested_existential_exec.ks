// test: execution
// stdlib: true
// expect-exit: 0
//
// Boxing an existential into a different box goes through `any P: P`, giving
// a box inside a box (design §4, edge cases).

module Test

import std.numeric.(Int64)
import std.memory.(OwnedBox)

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
    let inner: any Shape = Circle(r: 2);
    let outer: any[OwnedBox] Shape = inner;
    if outer.area() != 12 { return 1 }
    0
}
