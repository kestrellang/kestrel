// test: diagnostics
// stdlib: false
//
// F1 — the ranges in column 0 cover every Int64, but only with `true` in
// column 1, so `(5, false)` is unmatched. The checker used to stop at "column
// 0 is fully covered" without looking at column 1, accepted this, and the
// program trapped at run time.

module Main

func test(n: lang.i64, b: lang.i1) -> lang.i64 {
    match (n, b) { // ERROR: exhaustive
        (..<0, true) => 1,
        (0.., true) => 2
    }
}
