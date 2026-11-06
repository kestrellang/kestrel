// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Self.helper(..)` inside `Wrap[H]` is `Wrap[H]` — the enclosing instantiation.

protocol Named { static func name() -> Int64 }
struct A: Named { static func name() -> Int64 { 7 } }

struct Wrap[H] where H: Named {
    var v: Int64
    init() { self.v = Self.helper(1); }
    static func helper(x: Int64) -> Int64 { H.name() + x }
}

@main
func main() -> Int64 {
    if Wrap[A]().v != 8 { return 1 }
    0
}
