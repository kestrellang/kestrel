// test: execution
// stdlib: true
// expect-exit: 0

// Fuzz H2: a struct nested in a generic struct gets a layout and runs.
module N

import std.numeric.(Int64)

struct Outer[T] {
    struct Inner { var x: Int64 }
    var t: T
    var i: Inner
}

@main
func main() -> Int64 {
    let o = Outer[Int64](t: 1, i: Outer[Int64].Inner(x: 2));
    o.i.x + o.t - 3
}
