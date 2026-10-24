// test: execution
// stdlib: true
// expect-exit: 0
//
// `Array[any Shape]` holds two conformers; requirement calls dispatch
// through the function table.

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
    let shapes: Array[any Shape] = [Circle(r: 1), Square(side: 2), Circle(r: 2)];
    var total: Int64 = 0;
    for s in shapes { total = total + s.area(); }
    if total != 3 + 4 + 12 { return 1 }
    0
}
