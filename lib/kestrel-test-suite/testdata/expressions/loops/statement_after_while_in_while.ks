// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        while lang.i64_signed_lt(x, 5) {
            x = lang.i64_add(x, 1);
        }
        x = lang.i64_add(x, 1);
    }
}
