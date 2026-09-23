// test: diagnostics
// stdlib: true

// G17 S4: MEMBER LOOKUP on a projection receiver must honour the receiver a
// where clause names. `struct Pair[A, B] where …, A.Item: Show` bounds A's
// `Item` only, so the direct call `self.b.produce().show()` on `B.Item` has no
// `show` to find. The bound search collected every bound filed against the
// alias entity `Producer.Item` regardless of base, handed `Show` to `B.Item`,
// and the frontend accepted; the program then died after monomorphization
// with `String` lacking a `show` witness.
//
// This is the member-lookup sibling of `assoc_projection_bound_on_container.ks`
// (C4 made the CONFORMANCE answer base-aware; that fix never reached member
// lookup). The fix filters the bound search through the same per-bound
// verdict C4 uses, so the two paths cannot disagree about which receiver a
// clause names.
//
// A/B evidence: `_control.ks` is this file with the `A.Item: Show` clause
// deleted, and it rejects with the same diagnostic at the same span. The
// clause no longer changes what `B.Item` can do — which is the whole claim.
// `assoc_projection_bound_member_same_receiver.ks` shows the clause still
// grants `show` to `A.Item`, so this is not a blanket rejection.

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

struct Pair[A, B] where A: Producer, B: Producer, A.Item: Show {
    var a: A;
    var b: B;
    public func go() -> String { self.b.produce().show() } // ERROR: no member 'show'
}

@main
func main() -> lang.i32 {
    print("r=\(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go())");
    0
}
