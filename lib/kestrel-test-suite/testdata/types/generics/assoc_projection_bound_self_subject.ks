// test: diagnostics
// stdlib: true

// G17: the projection subject is rooted at `Self` rather than a named type
// parameter — `extend Producer where Self.Item: Show`. `StrSrc.Item = String`,
// which has no `Show` witness, so `StrSrc(…).render()` must not resolve to that
// extension. Today `Self`-rooted projection subjects are dropped outright
// (`resolve_projection_subject` returns `None` for them), so the constrained
// extension applies unconditionally and the error only surfaces after
// monomorphization.
//
// No A/B control: the whole clause is the subject under test, and deleting it
// would change which extension exists rather than which receiver it constrains.
//
// EXPECTED TO FAIL: a diagnostics test never monomorphizes, so the current
// post-mono error is invisible to it. This is the test for the D8 `SelfType`
// flip (plan-3a).

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
    print("result=\(StrSrc(s: "z").render())"); // ERROR: !: Show
    0
}
