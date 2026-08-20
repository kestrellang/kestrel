// test: execution
// stdlib: true
// expect-exit: 15

// G18 regression guard for the OTHER direction: `std.core.Bool` is a nominal
// struct (not `MirTy::Bool`), and `coerce_condition_to_i1` deliberately skips
// the `boolValue()` witness call for it — its `boolValue()` is by construction
// `{ self.value }` and both backends already scalarize `Bool` to i1. That skip
// must not change what any of the four condition positions computes.
//
// The MIR-shape side of this claim (that NO witness call is emitted here) is
// pinned by `bool_condition_emits_no_boolvalue_witness_call` in
// kestrel-mir-lower.

module Test
import std.numeric.Int64

func classify(n: Int64) -> Int64 {
    guard n > Int64(raw: 0) else {
        return Int64(raw: 0);
    }
    match n.raw {
        v if n > Int64(raw: 100) => Int64(raw: 1),
        _ => n
    }
}

@main
func main() -> lang.i64 {
    var total = Int64(raw: 0);
    var i = Int64(raw: 0);
    while i < Int64(raw: 6) {
        if i > Int64(raw: 0) {
            total = total + classify(i);
        } else {
            total = total + classify(Int64(raw: 0) - Int64(raw: 4));
        }
        i = i + Int64(raw: 1);
    }
    // classify(-4) == 0, then 1+2+3+4+5 == 15
    total.raw
}
