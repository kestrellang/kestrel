// test: diagnostics
// stdlib: false

module Main

func compute() -> lang.i1 {
    lang.i1_and(lang.i64_signed_lt(lang.i64_add(1, lang.i64_mul(2, 3)), 10), lang.i64_signed_gt(lang.i64_sub(5, 1), 2))
}
