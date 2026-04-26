// test: diagnostics
// stdlib: false

module Main

func sumUntil(limit: lang.i64) -> lang.i64 {
    var sum = 0;
    var i = 0;
    while true {
        if lang.i64_signed_ge(i, limit) {
            break
        }
        if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) {
            i = lang.i64_add(i, 1);
            continue
        }
        sum = lang.i64_add(sum, i);
        i = lang.i64_add(i, 1);
    }
    sum
}
