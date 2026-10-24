// test: execution
// stdlib: true
// expect-exit: 0
//
// The `is` / `is not` operator: in `if`, in a closure, and combined with
// `and` / `or` (design §8b).

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

@main
func main() -> Int64 {
    let shapes: Array[any Shape] = [Circle(r: 1), Square(side: 1), Circle(r: 2)];
    var circles: Int64 = 0;
    for s in shapes { if s is Circle { circles = circles + 1; } }
    if circles != 2 { return 1 }
    if shapes(1) is not Square { return 2 }
    if shapes(0) is Circle and shapes(1) is Circle { return 3 }
    if !(shapes(0) is Square or shapes(2) is Circle) { return 4 }
    let squares = shapes.filter { it is Square };
    if squares.count != 1 { return 5 }
    let b = shapes(0) is Circle;
    if !b { return 6 }
    0
}
