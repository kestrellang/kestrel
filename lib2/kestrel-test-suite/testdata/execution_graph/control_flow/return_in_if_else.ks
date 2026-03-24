// test: diagnostics
// stdlib: false

module Main

func earlyReturn(x: lang.i64) -> lang.i64 {
    if lang.i64_signed_lt(x, 0) {
        return 0
    } else {
        return x
    }
}
