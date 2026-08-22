// test: diagnostics
// stdlib: true

// G17: two UNRELATED protocols each declare an associated type spelled `Item`.
// `where A.Item: Show` on `ProducerA` must not grant `Show` to `ProducerB`'s
// `Item`, which is a different entity that merely shares a name. Today the
// name-equality fallback in the solver matches them and the call is accepted,
// so `Int64.show()` runs on a `String` — a miscompile, not merely a wrong
// accept. Its output is a constant (`result=i`) rather than a pointer, which is
// why it went unnoticed; a diagnostics test never runs it either way.
//
// Pairs with `_control.ks`, which is the same program with the two associated
// types RENAMED apart (`ItemA` / `ItemB`) and is correctly rejected. The pair
// isolates the name collision as the cause.
//
// EXPECTED TO FAIL until the cross-protocol `Name`-equality fallback is
// narrowed (plan-3a: solver.rs:2836).

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

protocol ProducerA { type Item; func produceA() -> Item }
protocol ProducerB { type Item; func produceB() -> Item }

struct IntSrc { var v: Int64; }
extend IntSrc: ProducerA {
    public type Item = Int64;
    public func produceA() -> Int64 { self.v }
}
struct StrSrc { var s: String; }
extend StrSrc: ProducerB {
    public type Item = String;
    public func produceB() -> String { self.s.clone() }
}

func needsShow[T](x: T) -> String where T: Show { x.show() }

// Bound is on A.Item (a DIFFERENT alias entity than B.Item).
func bad[A, B](a: A, b: B) -> String where A: ProducerA, B: ProducerB, A.Item: Show {
    needsShow(b.produceB()) // ERROR: !: Show
}

@main
func main() -> lang.i32 {
    print("result=\(bad(IntSrc(v: 1), StrSrc(s: "z")))");
    0
}
