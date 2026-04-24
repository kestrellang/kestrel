// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 20) {
        x = lang.i64_add(x, 1);
        if lang.i64_eq(x, 5) {
            continue;
        }
        if lang.i64_eq(x, 10) {
            continue;
        }
    }
}
