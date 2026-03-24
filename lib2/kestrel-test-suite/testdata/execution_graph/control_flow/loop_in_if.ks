// test: diagnostics
// stdlib: false

module Main

func loopInIf(x: lang.i64) -> lang.i64 {
    if lang.i64_signed_gt(x, 0) {
        var i = 0;
        while lang.i64_signed_lt(i, x) {
            i = lang.i64_add(i, 1);
        }
        i
    } else {
        0
    }
}
