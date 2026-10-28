// test: diagnostics
// stdlib: true

// G17 S2 (scope half): a where-clause subject is resolved in the scope of the
// entity the clause is WRITTEN ON, not in the body under inference. Here the
// clause `where Item: Show` is written on the extension and means the
// protocol's associated type `Item`; the method's own type parameter, also
// spelled `Item`, is unbounded and must NOT pick the bound up by name.
//
// Before the fix the subject was resolved against `body_owner`, so the name
// aliased in both directions at once: the method's `Item` wrongly gained
// `Show` and the protocol's `Item` wrongly lost it. The visible symptom was a
// wrong-reject with a degraded diagnostic — a spanless
// `E100 does not conform to protocol; does not satisfy constraint` attached to
// a synthesized node instead of an error on `x.show()`. Renaming the method's
// type parameter (see `_control.ks`) restored a located error, which is what
// makes the pair evidence: only the NAME differs between the two files.

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "int:\(self)" } }

protocol Producer {
    type Item
    func produce() -> Item
}

extend Producer where Item: Show {
    // `Item` here is a METHOD type parameter shadowing the associated type.
    // It carries no bounds, so `x.show()` must be rejected — with a span.
    public func leak[Item](x: Item) -> String {
        x.show() // ERROR: no member 'show'
    }
}

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}

@main
func main() -> lang.i32 { 0 }
