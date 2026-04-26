// test: diagnostics
// stdlib: false

module Main

func process(x: lang.i64) -> lang.i64 {
    if lang.i64_signed_lt(x, 0) {
        return lang.i64_sub(0, x)
    }

    var result = x;
    while lang.i64_signed_gt(result, 100) {
        result = lang.i64_sub(result, 100);
        if lang.i64_signed_lt(result, 10) {
            return lang.i64_mul(result, 2)
        }
    }

    result
}
