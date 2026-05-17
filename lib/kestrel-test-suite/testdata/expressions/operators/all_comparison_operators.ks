// test: diagnostics
// stdlib: false

module Main

func isEqual() -> lang.i1 {
    lang.i64_eq(1, 1)
}

func isNotEqual() -> lang.i1 {
    lang.i64_ne(1, 2)
}

func isLess() -> lang.i1 {
    lang.i64_signed_lt(1, 2)
}

func isGreater() -> lang.i1 {
    lang.i64_signed_gt(2, 1)
}

func isLessOrEqual() -> lang.i1 {
    lang.i64_signed_le(1, 2)
}

func isGreaterOrEqual() -> lang.i1 {
    lang.i64_signed_ge(2, 1)
}
