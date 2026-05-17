// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 100) {
        if lang.i64_signed_lt(x, 10) {
            x = lang.i64_add(x, 1);
        } else if lang.i64_signed_lt(x, 50) {
            x = lang.i64_add(x, 5);
        } else {
            x = lang.i64_add(x, 10);
        }
    }
}
