// test: diagnostics
// stdlib: true

module Test

// Bare `Wrap` inside `Wrap[H]` is `Wrap[_]`, not `Self` (`Wrap[H]`). When the
// arguments don't determine `H`, that is an inference error — never a
// compiler crash. (`Self.helper(1)` is the way to mean the enclosing `H`.)

protocol Named { static func name() -> Int64 }

struct Wrap[H] where H: Named {
    var v: Int64
    init() { self.v = Wrap.helper(1); } // ERROR: does not satisfy constraint
    static func helper(x: Int64) -> Int64 { H.name() + x }
}
