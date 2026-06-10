// test: diagnostics
// stdlib: false

// Stage 0.5: references cannot be stored — a `let`/`var` annotation of ref
// type is rejected at HIR lowering.
module Test

func f() {
    let y: lang.i64 = 1;
    let r: &lang.i64 = y; // ERROR(E482)
}
