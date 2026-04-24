// test: diagnostics
// stdlib: false

module Main

func sumOdd(limit: lang.i64) -> lang.i64 {
    var sum = 0;
    var i = 0;
    while lang.i64_signed_lt(i, limit) {
        i = lang.i64_add(i, 1);
        if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) {
            continue
        }
        sum = lang.i64_add(sum, i);
    }
    sum
}
