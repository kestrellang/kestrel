// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 100) {
        if lang.i64_signed_lt(x, 10) {
            x = lang.i64_add(x, 1);
        }
        while lang.i64_signed_lt(x, 20) {
            x = lang.i64_add(x, 1);
        }
        loop {
            x = lang.i64_add(x, 1);
            break;
        }
        if lang.i64_signed_gt(x, 50) {
            break;
        }
        x = lang.i64_add(x, 1);
    }
}
