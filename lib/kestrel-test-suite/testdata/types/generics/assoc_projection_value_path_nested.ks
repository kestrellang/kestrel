// test: diagnostics
// stdlib: true

// G26 Part 2: a qualified value path ending in an associated type,
// `Item.Sub`, reaches `lower_path`'s `AssociatedType { container: Some(_) }`
// arm. That arm used to build `Def(Inner.Sub)` — the alias with its base
// `Item` dropped — which typed the local as a projection mono could not
// resolve and ICE'd in post-mono verify ("AssociatedProjection in value").
// It now lowers to `HirExpr::TypeRef`, keeping the base, and a type used as a
// bare value is reported in the frontend, like a bare `T`.

module Test

import std.numeric.Int64

protocol Inner { type Sub; }
struct In1 {}
extend In1: Inner { public type Sub = Int64; }

protocol Outer { type Item: Inner; func tag() -> Int64 }
extend Outer {
    public func b() -> Int64 {
        let x = Item.Sub; // ERROR: type parameter cannot be used as a value
        self.tag()
    }
}

struct O1 {}
extend O1: Outer {
    public type Item = In1;
    public func tag() -> Int64 { 9 }
}

@main
func main() -> lang.i32 {
    print("r=\(O1().b())");
    0
}
