// test: diagnostics
// stdlib: true

// G17 control for `assoc_projection_bound_on_container.ks`: identical struct,
// with the `A.Item: Show` clause DELETED. The compiler then rejects
// `needsShow(self.b.produce())` in the frontend, which proves the sibling
// file's accept is caused by the container-level projection clause leaking to
// `B.Item` and not by a generally absent check on struct method bodies.
//
// This one passes today and MUST stay green.

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

// Same struct as the sibling, with the projection bound removed.
struct Pair[A, B] where A: Producer, B: Producer {
    var a: A;
    var b: B;
    public func go() -> String { needsShow(self.b.produce()) } // ERROR: !: Show
}

@main
func main() -> lang.i32 {
    print("result=\(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go())");
    0
}
