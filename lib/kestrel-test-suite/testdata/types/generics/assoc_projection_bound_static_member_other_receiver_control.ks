// test: execution
// stdlib: true
// expect-stdout: n=42 m=43\n

// G17 S5 control for `assoc_projection_bound_static_member_other_receiver.ks`:
// the same type-level call `B.Item.zero()`, with `Zero` now bounded on
// `B.Item` too, so it is legal and must run with `B.Item`'s own witness.
// `B.Item` is `W`, deliberately NOT `A.Item`'s `Int64`: a receiver that has
// lost its base cannot tell the two witnesses apart, and picking `Int64.zero`
// for a `W` result crashes at run time instead of printing.

module Test

import std.numeric.Int64

protocol Zero { static func zero() -> Self }
extend Int64: Zero { public static func zero() -> Int64 { 0 } }

struct W { var n: Int64; var m: Int64; }
extend W: Zero { public static func zero() -> W { W(n: 42, m: 43) } }

protocol Producer { type Item; func produce() -> Item }

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}
struct WSrc { var w: W; }
extend WSrc: Producer {
    public type Item = W;
    public func produce() -> W { self.w }
}

struct Pair[A, B] where A: Producer, B: Producer, A.Item: Zero, B.Item: Zero {
    var a: A;
    var b: B;
    public func go() -> B.Item { B.Item.zero() }
}

@main
func main() -> lang.i32 {
    let w = Pair(a: IntSrc(v: 7), b: WSrc(w: W(n: 1, m: 2))).go();
    print("n=\(w.n) m=\(w.m)");
    0
}
