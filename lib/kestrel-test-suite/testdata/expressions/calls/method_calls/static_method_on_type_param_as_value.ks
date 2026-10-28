// test: execution
// stdlib: true
// expect-stdout: n=42 m=43\n

// G26: the type-PARAMETER half of `static_method_on_type_as_value.ks`. A
// static protocol requirement named through a bounded type parameter,
// `let f = T.zero;`, is a complete function value, exactly as `Box.stat` is.
// Today the reference is not typed as a function: annotated `() -> T` it is
// a type mismatch ("got T"), and unannotated the call `f()` is rejected with
// E100. Not a projection defect: it is the general reason
// the projection spelling `let f = A.Item.zero;` cannot work either.
// EXPECTED TO FAIL until an unapplied static member on an abstract receiver
// is typed as a function.

module Test

import std.numeric.Int64

protocol Zero { static func zero() -> Self }

struct W { var n: Int64; var m: Int64; }
extend W: Zero { public static func zero() -> W { W(n: 42, m: 43) } }

func make[T]() -> T where T: Zero {
    let f = T.zero;
    f()
}

@main
func main() -> lang.i32 {
    let w: W = make();
    print("n=\(w.n) m=\(w.m)");
    0
}
