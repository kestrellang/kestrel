// test: diagnostics
// stdlib: false

module Main

func subtract() -> lang.i64 {
    lang.i64_sub(lang.i64_sub(10, 3), 2)
}

func divide() -> lang.i64 {
    lang.i64_signed_div(lang.i64_signed_div(24, 4), 2)
}
