// test: execution
// stdlib: true
// expect-exit: 0
//
// `any P: P`: an existential is accepted where `T: P` is required, by a
// `some P` parameter, and as the receiver of a protocol extension method
// (design §6).

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

extend Shape {
    func doubled() -> Int64 { self.area() * 2 }
}

func viaGeneric[T](x: T) -> Int64 where T: Shape { x.area() }
func viaSome(x: some Shape) -> Int64 { x.area() }

@main
func main() -> Int64 {
    let s: any Shape = Square(side: 3);
    if viaGeneric(s) != 9 { return 1 }
    if viaSome(s) != 9 { return 2 }
    if s.doubled() != 18 { return 3 }
    0
}
