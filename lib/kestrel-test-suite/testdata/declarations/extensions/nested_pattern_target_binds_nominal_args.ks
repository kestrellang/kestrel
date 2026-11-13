// test: execution
// stdlib: true
// expect-exit: 0
//
// A name nested inside an extension target's args binds a pattern
// parameter: `extend Box[W[X]]` applies to any `Box[W[_]]`, with `X` the
// inner arg. A nested name that IS a type stays concrete: `extend
// Box[W[Int64]]` applies only to `Box[W[Int64]]`. Sugar nests too (`[E]`).

module Test

struct Box[T] { var v: T }
struct W[U] { var u: U }

extend Box[W[X]] {
    func inner() -> X { self.v.u }
}

extend Box[W[Int64]] {
    func bumped() -> Int64 { self.v.u + 1 }
}

extend Box[[E]] {
    func firstOr(fallback: E) -> E {
        if self.v.count > 0 { self.v(0) } else { fallback }
    }
}

@main
func main() -> Int64 {
    if Box(v: W(u: 7)).inner() != 7 { return 1 }
    if Box(v: W(u: "w")).inner() != "w" { return 2 }
    if Box(v: W(u: 4)).bumped() != 5 { return 3 }
    if Box(v: ["a", "bc"]).firstOr("z") != "a" { return 4 }
    0
}
