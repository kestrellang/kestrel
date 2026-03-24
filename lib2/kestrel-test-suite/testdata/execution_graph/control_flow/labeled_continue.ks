// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var result = 0;
    var i = 0;
    outer: while lang.i64_signed_lt(i, 10) {
        var j = 0;
        while lang.i64_signed_lt(j, 10) {
            if lang.i64_eq(j, 5) {
                i = lang.i64_add(i, 1);
                continue outer
            }
            result = lang.i64_add(result, 1);
            j = lang.i64_add(j, 1);
        }
        i = lang.i64_add(i, 1);
    }
    result
}
