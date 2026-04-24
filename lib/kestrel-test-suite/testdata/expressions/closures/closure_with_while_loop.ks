// test: diagnostics
// stdlib: false

module Main

func test() -> (lang.i64) -> lang.i64 {
    { (n) in
        var i = 0;
        var sum = 0;
        while lang.i64_signed_lt(i, n) {
            sum = lang.i64_add(sum, i);
            i = lang.i64_add(i, 1);
        }
        sum
    }
}
