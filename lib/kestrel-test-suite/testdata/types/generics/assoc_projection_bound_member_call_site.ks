// test: diagnostics
// stdlib: true

// G17 C11: the call-site obligation for a projection bound on a *method's own*
// type parameter — `func render[U](u: U) where U: Producer, U.Item: Show`. The
// obligation is emitted by `solve_member`, whose where-clause loop skipped every
// non-`Param` subject outright. That site is not the same shape as the two
// function-call sites (C7, C10): it does a two-stage lookup, the method's own
// `type_params` first and the receiver's `subs` second, which `SubjectRoot`
// models as a single flattened substitution (`find` is first-wins).
//
// This is deliberately NOT the shape plan-3a proposed for this site. Its
// suggested test — `extend Box[T]: Show where T.Item: Show` called as
// `Box(inner: StrSrc(…)).show()` — was closed by C9 as an unplanned side
// effect: extension *applicability* is decided during member resolution, before
// `solve_member` ever reaches `resolution.where_clauses`, so it rejects with
// "no member 'show'". A bound on the method's own type param has no
// applicability gate to hide behind, so it reaches this site and is the shape
// that actually exercises it.
//
// `StrSrc.Item = String`, which has no `Show` witness. The obligation needs
// `ProjectionPolicy::Reduce`, not `Opaque`: an unreduced `StrSrc.Item` on a
// concrete base is judged as an opaque projection, which permits.
//
// A/B evidence: the control below is byte-identical except that it passes
// `IntSrc` (whose `Item = Int64` does conform) and must stay green — so the
// diagnostic here cannot be a blanket rejection of projection-bounded methods.

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

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

struct Holder { var n: Int64; }
extend Holder {
    public func render[U](u: U) -> String where U: Producer, U.Item: Show { "ok" }
}

@main
func main() -> lang.i32 {
    print("r=\(Holder(n: 1).render(StrSrc(s: "z")))"); // ERROR: !: Show
    0
}
