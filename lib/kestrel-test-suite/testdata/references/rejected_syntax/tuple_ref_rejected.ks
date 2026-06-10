// test: diagnostics
// stdlib: false

// Stage 0.5: a ref inside a tuple element is storage — rejected.
module Test

func f() {
    let y: lang.i64 = 1;
    let pair: (&lang.i64, lang.i64) = (y, y); // ERROR(E484)
}
