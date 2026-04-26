// test: diagnostics
// stdlib: false

module Main

func unaryInBinary() -> lang.i64 {
    lang.i64_add(lang.i64_neg(1), lang.i64_mul(lang.i64_neg(2), lang.i64_neg(3)))
}

func doubleNegation() -> lang.i64 {
    lang.i64_neg(lang.i64_neg(5))
}

func doubleLogicalNot() -> lang.i1 {
    lang.i1_not(lang.i1_not(true))
}
