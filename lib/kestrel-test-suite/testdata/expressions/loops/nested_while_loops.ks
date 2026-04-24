// test: diagnostics
// stdlib: false

module Main

func test() {
    var i: lang.i64 = 0;
    var j: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        j = 0;
        while lang.i64_signed_lt(j, 10) {
            j = lang.i64_add(j, 1);
        }
        i = lang.i64_add(i, 1);
    }
}
