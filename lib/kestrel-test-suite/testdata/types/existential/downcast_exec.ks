// test: execution
// stdlib: true
// expect-exit: 0
//
// `as?` borrows the payload as `Optional[&T]`; hit and miss, with `if let`
// and `guard let` (design §8).

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

func radiusOf(s: any Shape) -> Int64 {
    guard let .Some(c) = s as? Circle else { return -1 }
    c.r
}

@main
func main() -> Int64 {
    let s: any Shape = Circle(r: 4);
    if let .Some(c) = s as? Circle { if c.r != 4 { return 1 } } else { return 2 }
    if let .Some(q) = s as? Square { return 3 }
    if radiusOf(Square(side: 1)) != -1 { return 4 }
    if radiusOf(Circle(r: 9)) != 9 { return 5 }
    0
}
