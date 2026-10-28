// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Wrap[A].run(xs)`: the receiver's explicit `[A]` binds the TYPE's param `H`;
// the method's own param `S` is inferred from the argument. It used to bind
// `A` to `S` ("A !: Slice[UInt8]"). lang/crypto's one-shot
// `HMAC[SHA256].authenticate(key:message:)` has this shape.

protocol Named { static func name() -> Int64 }
struct A: Named { static func name() -> Int64 { 7 } }

struct Wrap[H] where H: Named {
    static func run[S](x: S) -> Int64 where S: Slice[UInt8] { H.name() + x.count }
}

@main
func main() -> Int64 {
    let xs: [UInt8] = [1, 2, 3];
    if Wrap[A].run(xs.asSlice()) != 10 { return 1 }
    0
}
