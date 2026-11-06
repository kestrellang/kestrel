// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Inside `Wrap[H]`, a bare `Wrap.helper(..)` is `Wrap[_]`: its `H` is
// inferred from the arguments (here `h: H`), exactly as outside the type.
// It used to reach monomorphization with no type args and ICE ("Call
// Callee::Direct not resolved"); lang/crypto's HMAC init (`HMAC.setup(..)`)
// hit it.

protocol Named { static func name() -> Int64 }
struct A: Named { static func name() -> Int64 { 7 } }

struct Wrap[H] where H: Named {
    var v: Int64
    init(h: H) { self.v = Wrap.helper(h, 1); }
    static func helper(h: H, x: Int64) -> Int64 { H.name() + x }
}

@main
func main() -> Int64 {
    let w = Wrap[A](A());
    if w.v != 8 { return 1 }
    0
}
