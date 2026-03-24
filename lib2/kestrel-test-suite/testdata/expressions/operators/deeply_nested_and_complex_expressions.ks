// test: diagnostics
// stdlib: false

module Main

func deeplyNested() -> lang.i64 {
    lang.i64_add(lang.i64_add(lang.i64_add(lang.i64_add(lang.i64_add(lang.i64_add(lang.i64_add(lang.i64_add(lang.i64_add(1, 2), 3), 4), 5), 6), 7), 8), 9), 10)
}

func mixedPrecedence() -> lang.i1 {
    lang.i1_or(lang.i1_and(lang.i64_signed_lt(lang.i64_add(lang.i64_mul(lang.i64_shl(1, 2), 3), 4), 100), true), false)
}

func parenthesized() -> lang.i64 {
    lang.i64_mul(lang.i64_add(1, 2), 3)
}

func deeplyGrouped() -> lang.i64 {
    lang.i64_mul(lang.i64_add(1, 2), lang.i64_add(3, 4))
}

func comparisonInLogical() -> lang.i1 {
    lang.i1_and(lang.i64_signed_lt(1, 2), lang.i64_signed_gt(3, 2))
}
