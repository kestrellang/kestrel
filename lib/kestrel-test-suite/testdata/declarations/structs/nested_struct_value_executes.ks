// test: execution
// stdlib: true
// expect-exit: 0

// Fuzz H2: a struct nested in a struct, used as a field value, must get a
// layout and run (previously ICE: "has no MonoStruct/MonoEnum").
module N

import std.numeric.(Int64)

struct Outer {
    struct Inner { var x: Int64 }
    var i: Inner
}

@main
func main() -> Int64 {
    let o = Outer(i: Outer.Inner(x: 3));
    o.i.x - 3
}
