// test: diagnostics
// stdlib: false

module Main

func check() -> lang.i1 {
    lang.i1_and(lang.i64_signed_lt(1, 2), lang.i64_signed_gt(3, 2))
}
