// test: diagnostics
// stdlib: false

module Main

func test() {
    var i: lang.i64 = 0;
    outer: loop {
        i = lang.i64_add(i, 1);
        var j: lang.i64 = 0;
        while lang.i64_signed_lt(j, i) {
            j = lang.i64_add(j, 1);
            if lang.i64_eq(j, 3) {
                continue;
            }
            if lang.i64_eq(j, 5) {
                continue outer;
            }
        }
        if lang.i64_signed_gt(i, 10) {
            break outer;
        }
    }
}
