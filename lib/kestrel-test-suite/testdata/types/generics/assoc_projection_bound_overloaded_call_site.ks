// test: diagnostics
// stdlib: true

// G17 C10: the call-site obligation for a projection bound (`A.Item: Show`) on
// an *overloaded* callee. `emit_resolved_call`'s only callers are inside
// `solve_overloaded_call`, so this is a strictly different code path from the
// one C7 fixed: an unambiguous call is typed by `lower_entity_ref` and takes
// `generate::emit_where_clause_constraints_with_subs` instead. Giving `good` a
// second, differently-labelled overload forces the `OverloadedCall` constraint
// and so reaches `emit_resolved_call`.
//
// `StrSrc.Item = String`, which has no `Show` witness, so the call must be
// rejected in the frontend. The obligation needs `ProjectionPolicy::Reduce`, not
// `Opaque`: with an opaque `StrSrc.Item` the projection never reduces and
// `solve_conforms` judges an unreduced projection on a concrete base, which
// permits.
//
// A/B evidence: the control below is byte-identical except that it passes
// `IntSrc` (whose `Item = Int64` *does* conform). It must stay green, so a
// diagnostic here cannot be a blanket rejection of every projection-bounded
// overloaded call.

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

// Two overloads of `good`, distinguished by argument label, so the call below
// becomes an `OverloadedCall` constraint rather than a direct `Def` reference.
func good[A](src a: A) -> String where A: Producer, A.Item: Show { "g" }
func good(flag b: Bool) -> String { "b" }

@main
func main() -> lang.i32 {
    print("r=\(good(src: StrSrc(s: "z")))"); // ERROR: !: Show
    0
}
