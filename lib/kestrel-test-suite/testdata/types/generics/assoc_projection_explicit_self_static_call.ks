// test: execution
// stdlib: true
// expect-stdout: a=5 b=5\n

// Inside `extend Outer`, a static call on an associated type spelled with
// explicit Self (`Self.Item.seed()`) dispatches like the implicit spelling
// (`Item.seed()`): name resolution classifies `Self.Item` as the implicit-Self
// associated type, not a plain `Def` of the alias.

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
