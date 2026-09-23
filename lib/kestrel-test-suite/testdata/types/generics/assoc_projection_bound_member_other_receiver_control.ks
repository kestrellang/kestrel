// test: diagnostics
// stdlib: true

// G17 S4 control for `assoc_projection_bound_member_other_receiver.ks`:
// identical, with the `A.Item: Show` clause DELETED. `B.Item` then has no
// bound at all and the direct `.show()` is rejected in the frontend. The
// sibling must produce the same diagnostic at the same span — proof that the
// clause on `A.Item` is what used to grant the member to `B.Item`.
//
// This one passed before S4 and MUST stay green.

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

struct Pair[A, B] where A: Producer, B: Producer {
    var a: A;
    var b: B;
    public func go() -> String { self.b.produce().show() } // ERROR: no member 'show'
}

@main
func main() -> lang.i32 {
    print("r=\(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go())");
    0
}
