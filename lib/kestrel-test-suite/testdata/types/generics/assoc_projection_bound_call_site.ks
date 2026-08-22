// test: diagnostics
// stdlib: true

// G17: the callee here is CORRECT — `good` only touches `a.produce()`, whose
// bound `A.Item: Show` it does declare. The bug is at the CALL SITE: passing
// `StrSrc` means `A.Item = String`, which has no `Show` witness, so the
// obligation `A.Item: Show` must be discharged (and must fail) at `good(…)`.
// Today no call-site obligation is emitted for a projection subject at all, so
// the call is accepted and only fails after monomorphization.
//
// This is the isolated call-site half of G17: no leaking clause is involved,
// which is why it needs no A/B control — the callee is unimpeachable.
//
// EXPECTED TO FAIL: a diagnostics test never monomorphizes, so the current
// post-mono error is invisible to it. Flips green when the direct-`Def`
// call-site obligation stops skipping projection subjects (plan-3a:
// solver.rs:3534, with the `Reduce` projection policy).

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "int:\(self)" } }

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

func needsShow[T](x: T) -> String where T: Show { x.show() }

// Correct body: only uses a.produce(), whose bound IS declared.
func good[A](a: A) -> String where A: Producer, A.Item: Show {
    needsShow(a.produce())
}

@main
func main() -> lang.i32 {
    // A = StrSrc → A.Item = String, which does NOT conform to Show.
    // The call-site obligation `A.Item: Show` should reject this.
    print("result=\(good(StrSrc(s: "z")))"); // ERROR: !: Show
    0
}
