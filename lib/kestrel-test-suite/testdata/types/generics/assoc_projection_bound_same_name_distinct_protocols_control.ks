// test: diagnostics
// stdlib: true

// G17 control for `assoc_projection_bound_same_name_distinct_protocols.ks`:
// the same two-protocol program with the associated types RENAMED apart, so
// `ProducerA.ItemA` and `ProducerB.ItemB` no longer collide by name. The
// compiler then rejects `needsShow(b.produceB())` correctly, which pins the
// sibling file's accept on the name collision rather than on a missing check.
//
// This one passes today and MUST stay green.

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

protocol ProducerA { type ItemA; func produceA() -> ItemA }
protocol ProducerB { type ItemB; func produceB() -> ItemB }

struct IntSrc { var v: Int64; }
extend IntSrc: ProducerA {
    public type ItemA = Int64;
    public func produceA() -> Int64 { self.v }
}
struct StrSrc { var s: String; }
extend StrSrc: ProducerB {
    public type ItemB = String;
    public func produceB() -> String { self.s.clone() }
}

func needsShow[T](x: T) -> String where T: Show { x.show() }

// Bound is on A.ItemA (a DIFFERENT alias entity than B.ItemB).
func bad[A, B](a: A, b: B) -> String where A: ProducerA, B: ProducerB, A.ItemA: Show {
    needsShow(b.produceB()) // ERROR: !: Show
}

@main
func main() -> lang.i32 {
    print("result=\(bad(IntSrc(v: 1), StrSrc(s: "z")))");
    0
}
