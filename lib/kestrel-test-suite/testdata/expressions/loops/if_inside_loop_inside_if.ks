// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    if true {
        while lang.i64_signed_lt(x, 10) {
            if lang.i64_eq(x, 5) {
                break;
            }
            x = lang.i64_add(x, 1);
        }
    }
}
