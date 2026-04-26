// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var result = 0;
    outer: while true {
        var i = 0;
        while lang.i64_signed_lt(i, 10) {
            if lang.i64_eq(i, 5) {
                break outer
            }
            i = lang.i64_add(i, 1);
        }
        result = lang.i64_add(result, 1);
    }
    result
}
