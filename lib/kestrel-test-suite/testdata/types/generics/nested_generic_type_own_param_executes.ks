// test: execution
// stdlib: true
// expect-exit: 0

// Fuzz M13: the outer type parameter stays usable in the container and its
// methods, and a nested generic type with its own parameter works.
module N

import std.numeric.(Int64)

struct Outer[T] {
    struct Ok[U] {
        var u: U
        func value() -> U { let x: U = self.u; x }
    }
    var t: T
    func id(x: T) -> T { let y: T = x; y }
}

@main
func main() -> Int64 {
    let o = Outer[Int64](t: 1);
    let k = Outer[Int64].Ok[Int64](u: 2);
    o.id(o.t) + k.value() - 3
}
