// test: execution
// stdlib: true
// expect-exit: 0
//
// An empty protocol's table has only the header; `as?` and `is` still work.

module Test

import std.numeric.(Int64)
import std.core.(String)

protocol Marker {}

struct A {}
extend A: Marker {}
struct B {}
extend B: Marker {}

@main
func main() -> Int64 {
    let xs: Array[any Marker] = [A(), B(), A()];
    var count: Int64 = 0;
    for x in xs { if let .Some(_) = x as? A { count = count + 1; } }
    if count != 2 { return 1 }
    if xs(1) is A { return 2 }
    0
}
