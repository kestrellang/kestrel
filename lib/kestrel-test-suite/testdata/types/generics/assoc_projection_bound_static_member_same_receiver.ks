// test: execution
// stdlib: true
// expect-stdout: [0]\n

// G17 S5, permit side: the type-level call `A.Item.zero()` on the receiver the
// bound actually names (`A.Item: Zero`) must keep compiling and running.
// Pairs with `assoc_projection_bound_static_member_other_receiver.ks`, where
// the same call on `B.Item` has no bound to use.

module Test

import std.text.String
import std.numeric.Int64

protocol Zero { static func zero() -> Self }
extend Int64: Zero { public static func zero() -> Int64 { 0 } }

protocol Producer { type Item; func produce() -> Item }

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}
struct StrSrc { var s: String; }
extend StrSrc: Producer {
    public type Item = String;
    public func produce() -> String { self.s.clone() }
}

struct Pair[A, B] where A: Producer, B: Producer, A.Item: Zero {
    var a: A;
    var b: B;
    public func go() -> A.Item { A.Item.zero() }
}

@main
func main() -> lang.i32 {
    print("[\(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go())]");
    0
}
