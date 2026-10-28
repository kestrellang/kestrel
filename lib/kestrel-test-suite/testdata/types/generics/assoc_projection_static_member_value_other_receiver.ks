// test: diagnostics
// stdlib: true

// G26 Part 1, reject side: the VALUE-position spelling of
// `assoc_projection_bound_static_member_other_receiver.ks`. `Zero` is bounded
// on `A.Item` only, so `let f = B.Item.zero;` must be rejected in the
// frontend by the same base-aware member lookup that rejects the call
// `B.Item.zero()` — the bound on `A.Item` must not leak to `B.Item`.
// Pairs with `assoc_projection_static_member_value_same_receiver.ks`.
// EXPECTED TO FAIL: today the prefix `B.Item` is itself rejected ("no member
// 'Item' on type 'B'"), which is the wrong diagnostic for the wrong reason.

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
    public func go() -> Int64 {
        let f = B.Item.zero; // ERROR: no member 'zero' on type 'B.Item'
        0
    }
}

@main
func main() -> lang.i32 {
    print("[\(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go())]");
    0
}
