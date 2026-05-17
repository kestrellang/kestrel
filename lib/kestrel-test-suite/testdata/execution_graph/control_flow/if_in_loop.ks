// test: diagnostics
// stdlib: false

module Main

func ifInLoop(n: lang.i64) -> lang.i64 {
    var count = 0;
    var i = 0;
    while lang.i64_signed_lt(i, n) {
        if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) {
            count = lang.i64_add(count, 1);
        }
        i = lang.i64_add(i, 1);
    }
    count
}
