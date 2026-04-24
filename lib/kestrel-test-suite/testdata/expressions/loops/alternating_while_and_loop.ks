// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 5) {
        loop {
            var y: lang.i64 = 0;
            while lang.i64_signed_lt(y, 3) {
                loop {
                    break;
                }
                y = lang.i64_add(y, 1);
            }
            break;
        }
        x = lang.i64_add(x, 1);
    }
}
