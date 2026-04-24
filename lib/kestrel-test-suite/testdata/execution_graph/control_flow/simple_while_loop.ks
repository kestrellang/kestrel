// test: diagnostics
// stdlib: false

module Main

func countdown(n: lang.i64) -> lang.i64 {
    var i = n;
    while lang.i64_signed_gt(i, 0) {
        i = lang.i64_sub(i, 1);
    }
    i
}
