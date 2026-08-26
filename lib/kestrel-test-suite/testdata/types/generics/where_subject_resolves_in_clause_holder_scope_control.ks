// test: diagnostics
// stdlib: true

// G17 S2 control for `where_subject_resolves_in_clause_holder_scope.ks`:
// identical shape with the method's type parameter renamed `Item` -> `U`, so
// no name collides with the extension's `where Item: Show` subject. With no
// shadowing there is nothing for the old body-scoped resolution to alias, so
// this file was already rejected correctly and at the right span.
//
// It passes today and MUST stay green — it is the fixed point the sibling file
// is measured against.

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
    public func leak[U](x: U) -> String {
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
