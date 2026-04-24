// test: diagnostics
// stdlib: false

module Main

func bitwiseAnd() -> lang.i64 {
    lang.i64_and(5, 3)
}

func bitwiseOr() -> lang.i64 {
    lang.i64_or(5, 3)
}

func bitwiseXor() -> lang.i64 {
    lang.i64_xor(5, 3)
}

func shiftLeft() -> lang.i64 {
    lang.i64_shl(1, 3)
}

func shiftRight() -> lang.i64 {
    lang.i64_signed_shr(8, 2)
}
