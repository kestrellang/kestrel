// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.collections.(Slice)

// `Wrap[A].run(xs)`: the receiver's explicit `[A]` binds the TYPE's param `H`;
// the method's own param `S` is inferred from the argument. It used to bind
// `A` to `S` ("A !: Lends[UInt8]"). lang/crypto's one-shot
// `HMAC[SHA256].authenticate(key:message:)` had this shape.

// `Lends` stands in for the removed std `Sliceable`: a protocol whose
// conformers lend their elements as a `Slice`.
protocol Lends[T] { func asSlice() -> &Slice[T] }
extend Slice[T]: Lends[T] { }

protocol Named { static func name() -> Int64 }
struct A: Named { static func name() -> Int64 { 7 } }

struct Wrap[H] where H: Named {
    static func run[S](x: &S) -> Int64 where S: Lends[UInt8], S: not Static, S: not Copyable, S: not Sized { H.name() + x.asSlice().count }
}

@main
func main() -> Int64 {
    let xs: [UInt8] = [1, 2, 3];
    if Wrap[A].run(xs.asSlice()) != 10 { return 1 }
    0
}
