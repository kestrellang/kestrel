// test: diagnostics
// stdlib: true

// A nested name that resolves to a type is concrete, not a pattern
// parameter: `extend Box[W[Int64]]` gives `bumped()` to `Box[W[Int64]]`
// only, so a `Box[W[Bool]]` receiver has no such member.

module Test

struct Box[T] { var v: T }
struct W[U] { var u: U }

extend Box[W[Int64]] {
    func bumped() -> Int64 { self.v.u + 1 }
}

func test() -> Int64 {
    let b = Box(v: W(u: true));
    b.bumped() // ERROR: no member 'bumped' on type 'Box[W[Bool]]'
}
