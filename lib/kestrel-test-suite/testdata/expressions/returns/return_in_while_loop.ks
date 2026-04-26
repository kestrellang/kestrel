// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        if lang.i64_eq(x, 5) {
            return x
        }
        x = lang.i64_add(x, 1);
    }
    return x
}
