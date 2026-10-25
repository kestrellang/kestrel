// test: execution
// stdlib: true
// expect-stdout: n=42 m=43\n

// G26 Part 1, permit side: the VALUE-position spelling of
// `assoc_projection_bound_static_member_other_receiver_control.ks`.
// `let f = B.Item.zero;` with `B.Item: Zero` declared names a legal static
// function value and must run with `B.Item`'s own witness (`W.zero`, not
// `A.Item`'s `Int64.zero`). Today it lowers to a `Def(B)` + `Field` chain and
// is rejected with E100 "no member 'Item' on type 'B'".
// EXPECTED TO FAIL: blocked on `static_method_on_type_param_as_value.ks` —
// `let f = T.zero;` fails the same way, so this is not a projection defect.

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
    public func go() -> B.Item {
        let f = B.Item.zero;
        f()
    }
}

@main
func main() -> lang.i32 {
    let w = Pair(a: IntSrc(v: 7), b: WSrc(w: W(n: 1, m: 2))).go();
    print("n=\(w.n) m=\(w.m)");
    0
}
