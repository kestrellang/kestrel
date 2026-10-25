// test: diagnostics
// stdlib: true

// G17 control for `assoc_projection_bound_cross_receiver.ks`: identical shape
// with the `A.Item: Show` clause DELETED. With no projection bound anywhere the
// compiler rejects `needsShow(b.produce())` correctly, which proves the sibling
// file's accept comes from the clause leaking across receivers and not from a
// generally missing check.
//
// This one passes today and MUST stay green.

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

protocol Producer {
    type Item
    func produce() -> Item
}

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}

func needsShow[T](x: T) -> String where T: Show { x.show() }

// ONLY A.Item is bounded by Show. B.Item is NOT bounded.
func bad[A, B](a: A, b: B) -> String where A: Producer, B: Producer {
    needsShow(b.produce()) // ERROR: !: Show
}

@main
func main() -> lang.i32 {
    if bad(IntSrc(v: 1), IntSrc(v: 2)) != "i" { return 10 }
    0
}
