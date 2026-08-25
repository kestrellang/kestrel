// test: diagnostics
// stdlib: true

// G17 C11 control for `assoc_projection_bound_member_call_site.ks`.
// Byte-identical except the call passes `IntSrc`, whose `Item = Int64` *does*
// conform to `Show`, so the projection bound on the method's own type parameter
// is satisfied and the call must be accepted. Together the pair is the evidence
// that `solve_member`'s new call-site obligation discriminates on the projected
// type rather than rejecting every projection-bounded method call.

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
    print("r=\(Holder(n: 1).render(IntSrc(v: 3)))");
    0
}
