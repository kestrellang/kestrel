// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 100) {
        if true {
            while lang.i64_signed_lt(x, 10) {
                loop {
                    break;
                }
                x = lang.i64_add(x, 1);
            }
            x = lang.i64_add(x, 1);
        }
        x = lang.i64_add(x, 1);
    }
}
