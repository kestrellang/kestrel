// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        if lang.i64_eq(i, 5) {
            return i;
        }
        i = lang.i64_add(i, 1);
    }
    0
}
