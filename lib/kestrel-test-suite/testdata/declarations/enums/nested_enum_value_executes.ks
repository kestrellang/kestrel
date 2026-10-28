// test: execution
// stdlib: true
// expect-exit: 0

// Fuzz H2: an enum nested in a struct, with its own method, gets a layout and
// its members are lowered (previously ICE).
module N

import std.numeric.(Int64)

struct Outer {
    enum Kind {
        case A
        case B(Int64)
        func isA() -> Bool { match self { .A => true, .B(_) => false } }
    }
    var k: Kind
}

@main
func main() -> Int64 {
    let o = Outer(k: .B(4));
    if o.k.isA() { return 9 }
    match o.k { .A => 1, .B(n) => n - 4 }
}
