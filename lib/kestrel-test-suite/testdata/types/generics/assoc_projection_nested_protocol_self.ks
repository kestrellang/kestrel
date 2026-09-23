// test: execution
// stdlib: true
// expect-stdout: r=4 s=5\n

// Found while fixing G26: a TWO-level projection rooted at a protocol
// extension's implicit `Self` — `Item.Sub` inside `extend Outer` — is legal
// in type position and as a static-call receiver, but neither works.
// The annotation `-> Item.Sub` ICEs in post-mono verify ("AssociatedProjection
// in address type"): the one-level `Item` workaround in mir-lower's `ty.rs`
// does not reach a projection whose base is itself a projection on the
// protocol. `Item.Sub.seed()` is rejected with E100 "no matching subscript on
// type 'Sub'". EXPECTED TO FAIL until both are fixed.

module Test

import std.numeric.Int64

protocol Seeded { static func seed() -> Self }
extend Int64: Seeded { public static func seed() -> Int64 { 5 } }

protocol Inner { type Sub: Seeded; func mk() -> Sub }
struct In1 {}
extend In1: Inner {
    public type Sub = Int64;
    public func mk() -> Int64 { 4 }
}

protocol Outer { type Item: Inner; func item() -> Item }
extend Outer {
    public func viaAnnotation() -> Item.Sub { let y: Item.Sub = self.item().mk(); y }
    public func viaStaticCall() -> Item.Sub { Item.Sub.seed() }
}

struct O1 {}
extend O1: Outer {
    public type Item = In1;
    public func item() -> In1 { In1() }
}

@main
func main() -> lang.i32 {
    print("r=\(O1().viaAnnotation()) s=\(O1().viaStaticCall())");
    0
}
