// test: execution
// stdlib: true

// Operator-result shape projection (solver stage 2c): `100..=200` desugars
// to a Member call on an unresolved literal receiver, so nothing downstream
// could pin the literal — it force-defaulted to Int64 and then mismatched
// any non-Int64 expectation. The projection resolves the deferred operator
// member's result to `ClosedRange[?lit]` before defaulting, letting
// annotations (structural unification) and parameterized bounds
// (unify_bound_protocol_args) pin the element type bidirectionally.

module Test

// Pins through a parameterized protocol bound: R = ClosedRange[?lit] must
// satisfy RandomBounds[Int16], which pins ?lit = Int16.
func low16[R](bounds: R) -> Int16 where R: RandomBounds[Int16] {
    bounds.inclusiveBounds().start
}

@main
func main() -> lang.i64 {
    // Annotation pins the literal element through the projected shape.
    let r: ClosedRange[Int16] = 100..=200;
    if r.start != 100 { return 1 }
    if r.end != 200 { return 2 }

    // Half-open range, same mechanism via RangeConstructible.
    let h: Range[Int16] = 5..<10;
    if h.end != 10 { return 3 }

    // Generic call with a parameterized bound — no annotation anywhere.
    if low16(100..=200) != 100 { return 4 }
    if low16(3..<9) != 3 { return 5 }

    // Prefix (unary) range operator projects too.
    let p: RangeThrough[Int16] = ..=50;
    if p.end != 50 { return 6 }

    // Control: an unconstrained literal range still defaults to Int64.
    let d = 1..=3;
    var sum: Int64 = 0;
    for v in d { sum = sum + v }
    if sum != 6 { return 7 }

    0
}
