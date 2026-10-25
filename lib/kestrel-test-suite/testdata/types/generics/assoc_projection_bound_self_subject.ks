// test: diagnostics
// stdlib: true

// G17: the projection subject is rooted at `Self` rather than a named type
// parameter — `extend Producer where Self.Item: Show`. `StrSrc.Item = String`,
// which has no `Show` witness, so `StrSrc(…).render()` must not resolve to that
// extension.
//
// This is an *applicability* shape, not an obligation shape: the clause decides
// whether the constrained protocol extension provides `render` at all. When the
// constraint is unmet there is no resolved member to hang a conformance
// obligation on, so `no member 'render' on type 'StrSrc'` is the correct
// diagnostic — the same message the shipped `where Self: Q` twin
// `declarations/extensions/unconstrained_protocol_extension_not_found_when_constraint_not_met.ks`
// annotates. (This file previously expected `!: Show`, from a template that
// applied the obligation-shape annotation to all eight promoted G17 repros.)
//
// No A/B control: the whole clause is the subject under test, and deleting it
// would change which extension exists rather than which receiver it constrains.
//
// Fixed by the D8 `SelfType` flip (plan-3a, C9): `Self`-rooted projection
// subjects used to be dropped outright, so the extension applied
// unconditionally and the error only surfaced after monomorphization.

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

extend Producer where Self.Item: Show {
    public func render() -> String { needsShow(self.produce()) }
}

@main
func main() -> lang.i32 {
    // StrSrc.Item = String, which does NOT conform to Show.
    print("result=\(StrSrc(s: "z").render())"); // ERROR: member
    0
}
