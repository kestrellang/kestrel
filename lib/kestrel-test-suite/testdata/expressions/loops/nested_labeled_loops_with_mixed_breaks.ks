// test: diagnostics
// stdlib: false

module Main

func test() {
    var x: lang.i64 = 0;
    outer: while lang.i64_signed_lt(x, 100) {
        var y: lang.i64 = 0;
        middle: loop {
            var z: lang.i64 = 0;
            inner: while lang.i64_signed_lt(z, 10) {
                z = lang.i64_add(z, 1);
                if lang.i64_eq(z, 5) {
                    break inner;
                }
                if lang.i64_eq(z, 7) {
                    break middle;
                }
            }
            y = lang.i64_add(y, 1);
            if lang.i64_signed_gt(y, 3) {
                break outer;
            }
        }
        x = lang.i64_add(x, 1);
    }
}
