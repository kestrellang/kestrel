// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    guard lang.i64_signed_gt(x, 0) else {
        0 // ERROR: diverge
    }
    x
}
