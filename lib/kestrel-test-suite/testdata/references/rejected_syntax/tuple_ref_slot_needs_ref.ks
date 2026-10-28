// test: diagnostics
// stdlib: false

// Stage 2b: a ref TUPLE ELEMENT is a legal formation (the old E484), but
// there is no auto-borrow — an OWNED value cannot fill a `&T` slot; only
// a ref (binding read / ref-returning call) can.
module Test

func f() {
    let y: lang.i64 = 1;
    let pair: (&lang.i64, lang.i64) = (y, y); // ERROR: expected
}
