// test: diagnostics
// stdlib: false

module Main

func fibonacci(n: lang.i64) -> lang.i64 {
    if lang.i64_signed_le(n, 1) {
        n
    } else {
        var a = 0;
        var b = 1;
        var i = 2;
        while lang.i64_signed_le(i, n) {
            let temp = lang.i64_add(a, b);
            a = b;
            b = temp;
            i = lang.i64_add(i, 1);
        }
        b
    }
}
