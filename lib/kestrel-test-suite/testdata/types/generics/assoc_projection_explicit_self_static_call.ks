// test: execution
// stdlib: true
// expect-stdout: a=5 b=5\n

// Found while closing G26: inside `extend Outer`, a static call on an
// associated type works when spelled `Item.seed()` but not `Self.Item.seed()`
// — the explicit-Self spelling is rejected with E100 "no matching subscript
// on type 'Item'". The prefix `Self.Item` resolves through the protocol entity
// to a plain `Def` of the alias, not an associated-type projection, so
// `lower_call` never builds the type-level `MethodCall`. The implicit-Self
// half is the control. EXPECTED TO FAIL until the explicit spelling resolves
// like the implicit one.

module Test

import std.numeric.Int64

protocol Seeded { static func seed() -> Self }
extend Int64: Seeded { public static func seed() -> Int64 { 5 } }

protocol Outer { type Item: Seeded; func tag() -> Int64 }
extend Outer {
    public func implicitSelf() -> Item { Item.seed() }
    public func explicitSelf() -> Item { Self.Item.seed() }
}

struct O1 {}
extend O1: Outer {
    public type Item = Int64;
    public func tag() -> Int64 { 1 }
}

@main
func main() -> lang.i32 {
    print("a=\(O1().implicitSelf()) b=\(O1().explicitSelf())");
    0
}
