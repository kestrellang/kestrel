// test: diagnostics
// stdlib: true

// `extend[T] Shape: Describe[T]` makes `Sq` conform to `Describe[T]` for
// every `T`. A member that never mentions `T` (`twin`) leaves the call no
// way to pick one: E105, at the call. It used to stay a free param, reach
// monomorphization as an `Error` type arg and crash post-mono verification.
// A call whose argument fixes `T` (`make(t: 1)`) is fine.
module Test

import std.numeric.(Int64)

protocol Shape {
    func area() -> Int64
}

protocol Describe[T] {
    func twin() -> Self
    static func make(s s: Self, t t: T) -> Int64
}

struct Sq {
    var n: Int64
}

extend Sq: Shape {
    func area() -> Int64 { self.n }
}

extend[T] Shape: Describe[T] where Self: Copyable {
    func twin() -> Self { self }
    static func make(s s: Self, t t: T) -> Int64 { s.area() }
}

func check() -> Int64 {
    let a = Sq(n: 5).twin().n; // ERROR(E105)
    a + Sq.make(s: Sq(n: 2), t: 1)
}
