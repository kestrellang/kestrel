// test: diagnostics
// stdlib: false

module Main

func compute() -> lang.i64 {
    lang.i64_add(lang.i64_shl(1, 2), 3)
}
