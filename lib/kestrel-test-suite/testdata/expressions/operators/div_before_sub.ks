// test: diagnostics
// stdlib: false

module Main

func compute() -> lang.i64 {
    lang.i64_sub(10, lang.i64_signed_div(6, 2))
}
