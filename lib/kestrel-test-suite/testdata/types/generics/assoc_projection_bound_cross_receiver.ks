// test: diagnostics
// stdlib: true

// G17: a projection bound declared for ONE receiver must not be granted to a
// DIFFERENT receiver's projection of the same associated type. `where A.Item:
// Show` says nothing about `B.Item`, so `needsShow(b.produce())` must be
// rejected. Today the frontend accepts it and the program miscompiles: `B.Item`
// is confused with `A.Item`, so `Int64.show()` runs on a `String` and prints a
// raw heap address. Pairs with `_control.ks`, which deletes `A.Item: Show` and
// is correctly rejected — the pair is the evidence, not either file alone.
//
// EXPECTED TO FAIL until the `where_clause_assoc_subs` re-key lands (plan-3a §1)
// AND the base-aware conformance check lands (plan-3a, mechanism 2). Both are
// required: the re-key alone downgrades this from a miscompile to a post-mono
// error, which a diagnostics test still cannot see.

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "int:\(self)" } }

protocol Producer {
    type Item
    func produce() -> Item
}

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}

// StrSrc.Item = String, and String does NOT conform to Show.
struct StrSrc { var s: String; }
extend StrSrc: Producer {
    public type Item = String;
    public func produce() -> String { self.s.clone() }
}

func needsShow[T](x: T) -> String where T: Show { x.show() }

func bad[A, B](a: A, b: B) -> String where A: Producer, B: Producer, A.Item: Show {
    needsShow(b.produce()) // ERROR: !: Show
}

@main
func main() -> lang.i32 {
    // B = StrSrc → B.Item = String, which has no Show witness.
    print("result=\(bad(IntSrc(v: 1), StrSrc(s: "z")))");
    0
}
