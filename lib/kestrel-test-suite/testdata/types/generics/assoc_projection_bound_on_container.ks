// test: diagnostics
// stdlib: true

// G17: the projection bound lives on a generic STRUCT's where clause rather
// than a function's. `struct Pair[A, B] where …, A.Item: Show` constrains only
// `A.Item`, so a method body calling `needsShow(self.b.produce())` must be
// rejected. Today it is accepted and only fails after monomorphization.
//
// Two independent mechanisms are visible here. `emit_container_where_clauses`
// skips container-level projection clauses entirely, so nothing is even pushed
// into the substitution vector — yet the accept still happens, via
// `gather_bounds_from_where_clause` resolving the AST path `A.Item` to its LAST
// segment (`Producer.Item`) and thereby granting `Show` to every `_.Item` in
// scope. Pairs with `_control.ks`, which deletes the clause and is correctly
// rejected, proving the clause is what leaks.
//
// EXPECTED TO FAIL: a diagnostics test never monomorphizes, so the current
// post-mono error is invisible to it. The annotation records the diagnostic the
// frontend should produce, and flips green the day it does.

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

// Projection bound lives on the STRUCT, and constrains A.Item only.
struct Pair[A, B] where A: Producer, B: Producer, A.Item: Show {
    var a: A;
    var b: B;
    public func go() -> String { needsShow(self.b.produce()) } // ERROR: !: Show
}

@main
func main() -> lang.i32 {
    print("result=\(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go())");
    0
}
