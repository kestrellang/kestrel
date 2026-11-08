// test: execution
// stdlib: true
// expect-exit: 0
//
// Existentials in different boxes never convert (E147, any_box_mismatch.ks):
// to move a payload into another box, unbox it first with `as?`, then box
// the concrete value (design §4).

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
    guard let .Some(c) = inner as? Circle else { return 1 }
    let circle: Circle = c;
    let outer: any[OwnedBox] Shape = circle;
    if outer.area() != 12 { return 2 }
    if inner.area() != 12 { return 3 }
    0
}
