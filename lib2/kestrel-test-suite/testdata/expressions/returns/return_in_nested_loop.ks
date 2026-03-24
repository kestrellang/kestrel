// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        var j: lang.i64 = 0;
        while lang.i64_signed_lt(j, 10) {
            if lang.i64_eq(lang.i64_mul(i, j), 25) {
                return lang.i64_add(i, j)
            }
            j = lang.i64_add(j, 1);
        }
        i = lang.i64_add(i, 1);
    }
    return 0
}
