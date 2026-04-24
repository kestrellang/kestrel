// test: diagnostics
// stdlib: false

module Main

func factorial(n: lang.i64) -> lang.i64 {
    var result = 1;
    var i = n;
    while lang.i64_signed_gt(i, 1) {
        result = lang.i64_mul(result, i);
        i = lang.i64_sub(i, 1);
    }
    result
}
