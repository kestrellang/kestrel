// test: diagnostics
// stdlib: false
//
// The body of an `unsafe func` is not an unsafe context: each unsafe use in
// it still needs its own `unsafe`.

module Test

unsafe func inner() -> lang.i64 { 1 }

unsafe func outer() -> lang.i64 {
    inner() // ERROR: 'inner' is unsafe and requires an `unsafe` expression or block
}

unsafe func outerMarked() -> lang.i64 {
    unsafe inner()
}
