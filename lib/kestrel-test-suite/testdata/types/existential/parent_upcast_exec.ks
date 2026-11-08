// test: execution
// stdlib: true
// expect-exit: 0
//
// Upcasting `any Child` to `any Parent` loads the parent table pointer; no
// allocation (design §4, §5).

module Test

import std.numeric.(Int64)

protocol Named {
    func name() -> Int64
}

protocol Shape: Named {
    func area() -> Int64
}

struct Circle { var r: Int64; }
extend Circle: Named {
    public func name() -> Int64 { 42 }
}
extend Circle: Shape {
    public func area() -> Int64 { self.r }
}

func nameOf(n: any Named) -> Int64 { n.name() }

@main
func main() -> Int64 {
    let s: any Shape = Circle(r: 3);
    if s.area() != 3 { return 1 }
    if s.name() != 42 { return 2 }
    if nameOf(s) != 42 { return 3 }
    0
}
