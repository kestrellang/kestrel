// test: diagnostics
// stdlib: false

module Main

func negateInt() -> lang.i64 {
    lang.i64_neg(42)
}

func negateFloat() -> lang.f64 {
    lang.f64_neg(3.14)
}

func invert() -> lang.i64 {
    lang.i64_not(42)
}
