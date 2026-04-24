// test: diagnostics
// stdlib: false

module Main

func test() {
    var a: lang.i64 = 0;
    while lang.i64_signed_lt(a, 10) {
        var b: lang.i64 = 0;
        while lang.i64_signed_lt(b, 10) {
            var c: lang.i64 = 0;
            while lang.i64_signed_lt(c, 10) {
                var d: lang.i64 = 0;
                loop {
                    d = lang.i64_add(d, 1);
                    if lang.i64_signed_gt(d, 5) {
                        break;
                    }
                }
                c = lang.i64_add(c, 1);
            }
            b = lang.i64_add(b, 1);
        }
        a = lang.i64_add(a, 1);
    }
}
